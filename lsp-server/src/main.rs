use anyhow::Result;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use tokio::process::Command as ProcessCommand;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tokio::io::{stdin, stdout};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use url::Url;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct InitializationOptions {
    standard: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PhpcsSettings {
    standard: Option<String>,
}

#[derive(Debug, Clone)]
struct CompressedDocument {
    compressed_data: Vec<u8>,
    original_size: usize,
    checksum: String,
    compression_ratio: f32,
}

#[derive(Debug, Clone)]
struct CachedResults {
    diagnostics: Vec<Diagnostic>,
    result_id: String,
    generated_at: Instant,
}

#[derive(Debug, Clone)]
struct PhpcsLanguageServer {
    client: Client,
    // Compressed document storage to reduce memory usage
    open_docs: std::sync::Arc<std::sync::RwLock<HashMap<Url, CompressedDocument>>>,
    // Cache PHPCS results to avoid redundant linting
    results_cache: std::sync::Arc<std::sync::RwLock<HashMap<Url, CachedResults>>>,
    // Memory tracking
    total_memory_usage: std::sync::Arc<AtomicUsize>,
    standard: std::sync::Arc<std::sync::RwLock<Option<String>>>,  // None means use PHPCS defaults
    phpcs_path: std::sync::Arc<std::sync::RwLock<Option<String>>>,
    workspace_root: std::sync::Arc<std::sync::RwLock<Option<std::path::PathBuf>>>,
    // Limit concurrent PHPCS processes to prevent system overload
    process_semaphore: std::sync::Arc<Semaphore>,
}

impl PhpcsLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            open_docs: std::sync::Arc::new(std::sync::RwLock::new(HashMap::with_capacity(100))),
            results_cache: std::sync::Arc::new(std::sync::RwLock::new(HashMap::with_capacity(100))),
            total_memory_usage: std::sync::Arc::new(AtomicUsize::new(0)),
            standard: std::sync::Arc::new(std::sync::RwLock::new(None)),  // Let PHPCS use its defaults
            phpcs_path: std::sync::Arc::new(std::sync::RwLock::new(None)),
            workspace_root: std::sync::Arc::new(std::sync::RwLock::new(None)),
            // Limit to 4 concurrent PHPCS processes to avoid overwhelming the system
            process_semaphore: std::sync::Arc::new(Semaphore::new(4)),
        }
    }

    fn compress_document(&self, content: &str) -> CompressedDocument {
        let start = Instant::now();
        let original_size = content.len();

        // Use LZ4 for fast compression
        let compressed_data = compress_prepend_size(content.as_bytes());
        let compressed_size = compressed_data.len();
        let compression_ratio = compressed_size as f32 / original_size as f32;

        // Compute checksum for cache invalidation
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        let elapsed = start.elapsed();
        eprintln!("📦 PHPCS LSP: Compressed in {:.2}ms: {}KB → {}KB ({:.1}% ratio)",
            elapsed.as_secs_f64() * 1000.0,
            original_size / 1024,
            compressed_size / 1024,
            compression_ratio * 100.0
        );

        // Update memory tracking
        self.total_memory_usage.fetch_add(compressed_size, Ordering::Relaxed);

        CompressedDocument {
            compressed_data,
            original_size,
            checksum,
            compression_ratio,
        }
    }

    fn decompress_document(&self, doc: &CompressedDocument) -> Result<String> {
        let start = Instant::now();
        let decompressed = decompress_size_prepended(&doc.compressed_data)
            .map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))?;

        let content = String::from_utf8(decompressed)
            .map_err(|e| anyhow::anyhow!("UTF-8 conversion failed: {}", e))?;

        let elapsed = start.elapsed();
        if elapsed.as_millis() > 5 {
            eprintln!("⚠️ PHPCS LSP: Slow decompression: {:.2}ms for {}KB",
                elapsed.as_secs_f64() * 1000.0,
                doc.original_size / 1024
            );
        }

        Ok(content)
    }

    fn get_memory_usage_mb(&self) -> f32 {
        self.total_memory_usage.load(Ordering::Relaxed) as f32 / 1_048_576.0
    }

    fn log_memory_stats(&self) {
        if let Ok(docs) = self.open_docs.read() {
            let doc_count = docs.len();
            let total_original: usize = docs.values().map(|d| d.original_size).sum();
            let total_compressed: usize = docs.values().map(|d| d.compressed_data.len()).sum();
            let avg_ratio = if doc_count > 0 {
                docs.values().map(|d| d.compression_ratio).sum::<f32>() / doc_count as f32
            } else {
                0.0
            };

            eprintln!("📊 PHPCS LSP Memory Stats:");
            eprintln!("  📁 Documents: {}", doc_count);
            eprintln!("  💾 Compressed: {:.1}MB (from {:.1}MB original)",
                total_compressed as f32 / 1_048_576.0,
                total_original as f32 / 1_048_576.0
            );
            eprintln!("  📉 Average compression: {:.1}%", avg_ratio * 100.0);
            eprintln!("  🗄️ Results cached: {}",
                self.results_cache.read().map(|c| c.len()).unwrap_or(0)
            );
        }
    }

    fn get_phpcs_path(&self) -> String {
        // First check the cache
        if let Ok(guard) = self.phpcs_path.read() {
            if let Some(cached_path) = &*guard {
                eprintln!("📂 PHPCS LSP: Using cached PHPCS path: {}", cached_path);
                return cached_path.clone();
            }
        }

        eprintln!("🔍 PHPCS LSP: Detecting PHPCS path...");

        // Not cached, find and cache it
        let phpcs_path = {
            // First priority: Check for project-local vendor/bin/phpcs
            if let Ok(workspace_guard) = self.workspace_root.read() {
                if let Some(ref workspace_root) = *workspace_guard {
                    let vendor_phpcs = workspace_root.join("vendor/bin/phpcs");
                    eprintln!("🔍 PHPCS LSP: Checking for project PHPCS at: {}", vendor_phpcs.display());

                    if vendor_phpcs.exists() {
                        eprintln!("✅ PHPCS LSP: Found project-local PHPCS");
                        vendor_phpcs.to_string_lossy().to_string()
                    } else {
                        eprintln!("❌ PHPCS LSP: No project-local PHPCS found");
                        self.get_bundled_or_system_phpcs()
                    }
                } else {
                    eprintln!("❌ PHPCS LSP: No workspace root available");
                    self.get_bundled_or_system_phpcs()
                }
            } else {
                eprintln!("❌ PHPCS LSP: Could not access workspace root");
                self.get_bundled_or_system_phpcs()
            }
        };

        eprintln!("🎯 PHPCS LSP: Final PHPCS path: {}", phpcs_path);

        // Cache the result
        if let Ok(mut guard) = self.phpcs_path.write() {
            *guard = Some(phpcs_path.clone());
        }

        phpcs_path
    }

    fn get_bundled_or_system_phpcs(&self) -> String {
        // Second priority: Check for bundled PHPCS
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                let bundled_phpcs = exe_dir.join("phpcs.phar");
                eprintln!("🔍 PHPCS LSP: Checking for bundled PHPCS at: {}", bundled_phpcs.display());

                if bundled_phpcs.exists() {
                    eprintln!("✅ PHPCS LSP: Found bundled PHPCS PHAR");
                    return bundled_phpcs.to_string_lossy().to_string();
                } else {
                    eprintln!("❌ PHPCS LSP: No bundled PHPCS found");
                }
            } else {
                eprintln!("❌ PHPCS LSP: Could not get LSP directory");
            }
        } else {
            eprintln!("❌ PHPCS LSP: Could not get current executable path");
        }

        // Third priority: Fall back to system phpcs
        eprintln!("🔄 PHPCS LSP: Using system phpcs");
        "phpcs".to_string()
    }

    fn discover_standard(&self, workspace_root: Option<&std::path::Path>) {
        eprintln!("🔍 PHPCS LSP: Discovering coding standard...");

        if let Some(root) = workspace_root {
            let config_files = [
                ".phpcs.xml",
                "phpcs.xml",
                ".phpcs.xml.dist",
                "phpcs.xml.dist",
            ];

            for config_file in &config_files {
                let config_path = root.join(config_file);

                if config_path.exists() {
                    if let Some(path_str) = config_path.to_str() {
                        eprintln!("✅ PHPCS LSP: Found config file: {}", path_str);
                        if let Ok(mut standard_guard) = self.standard.write() {
                            *standard_guard = Some(path_str.to_string());
                        }
                        return;
                    }
                }
            }
        }

        // No config file found - use PHPCS defaults
        eprintln!("🎯 PHPCS LSP: No config files found - will use PHPCS defaults");
        if let Ok(mut standard_guard) = self.standard.write() {
            *standard_guard = None;
        }
    }

    async fn run_phpcs(&self, uri: &Url, _file_path: &str, content: Option<&str>) -> Result<Vec<Diagnostic>> {
        let start_time = Instant::now();
        let file_name = uri.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");
        
        eprintln!("🔍 PHPCS LSP: Starting lint for file: {}", file_name);
        
        // Acquire semaphore permit to limit concurrent PHPCS processes
        let available_permits = self.process_semaphore.available_permits();
        let _permit = self.process_semaphore.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to acquire process semaphore: {}", e))?;
        eprintln!("🎫 PHPCS LSP: Acquired process slot for {} (slots in use: {}/4)", 
            file_name, 4 - available_permits);
        
        // Use cached PHPCS path
        let phpcs_path = self.get_phpcs_path();

        // Always use stdin for content to avoid file system reads
        if content.is_none() {
            eprintln!("❌ PHPCS LSP: No content provided for {}", file_name);
            return Ok(vec![]);
        }

        let text = content.unwrap();
        eprintln!("📝 PHPCS LSP: Content size: {} bytes", text.len());

        let mut cmd = ProcessCommand::new(&phpcs_path);
        cmd.arg("--report=json")
           .arg("--no-colors")
           .arg("-q");

        // Only add standard if explicitly configured and file still exists
        let standard_info = if let Ok(standard_guard) = self.standard.read() {
            if let Some(ref standard) = *standard_guard {
                // Check if it's a file path and validate it exists
                if (standard.starts_with('/') || standard.starts_with("./") || standard.ends_with(".xml")) && !std::path::Path::new(standard).exists() {
                    eprintln!("⚠️ PHPCS LSP: Config file no longer exists: {}", standard);
                    eprintln!("🔄 PHPCS LSP: Re-discovering standard...");

                    // Get workspace root from the file URI
                    let workspace_root = if let Ok(file_path) = uri.to_file_path() {
                        file_path.parent().map(|p| p.to_path_buf())
                    } else {
                        None
                    };

                    // Re-discover the standard
                    self.discover_standard(workspace_root.as_deref());

                    // Use default for this run
                    eprintln!("🎯 PHPCS LSP: Using PHPCS default standard for this run");
                    " with default standard (config file missing)".to_string()
                } else {
                    eprintln!("⚙️ PHPCS LSP: Using configured standard: {}", standard);
                    cmd.arg(format!("--standard={}", standard));
                    format!(" with standard '{}'" , standard)
                }
            } else {
                eprintln!("🎯 PHPCS LSP: Using PHPCS default standard (no --standard flag)");
                " with default standard".to_string()
            }
        } else {
            " (failed to read standard)".to_string()
        };

        // Always use stdin to avoid file system reads
        // Add --stdin-path to provide the actual filename to PHPCS
        let stdin_path_info = if let Ok(file_path) = uri.to_file_path() {
            cmd.arg(format!("--stdin-path={}", file_path.display()));
            format!(" (stdin-path: {})", file_path.display())
        } else {
            " (stdin-path: not available)".to_string()
        };
        cmd.arg("-");

        eprintln!("🚀 PHPCS LSP: Running PHPCS on {}{}{}", file_name, standard_info, stdin_path_info);
        cmd.stdin(std::process::Stdio::piped())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped())
           .kill_on_drop(true);  // Ensure process is killed if dropped

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("❌ PHPCS LSP: Failed to spawn PHPCS for {}: {}", file_name, e);
                return Err(anyhow::anyhow!("PHPCS error: {}", e));
            }
        };

        // Async write to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            match timeout(Duration::from_secs(5), stdin.write_all(text.as_bytes())).await {
                Ok(Ok(_)) => {
                    drop(stdin); // Close stdin to signal EOF
                }
                Ok(Err(e)) => {
                    eprintln!("⚠️ PHPCS LSP: Failed to write {} bytes to stdin for {}: {}", 
                        text.len(), file_name, e);
                    child.kill().await.ok();
                    return Err(anyhow::anyhow!("Failed to send content to PHPCS for {}: {}", file_name, e));
                }
                Err(_) => {
                    eprintln!("⏱️ PHPCS LSP: Timeout writing {} bytes to PHPCS stdin for {} (>5s)", 
                        text.len(), file_name);
                    child.kill().await.ok();
                    return Err(anyhow::anyhow!("Timeout writing to PHPCS for {} after 5 seconds", file_name));
                }
            }
        }

        // Wait for output with timeout (10 seconds for PHPCS execution)
        let output = match timeout(Duration::from_secs(10), child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let elapsed = start_time.elapsed();
                eprintln!("⚡ PHPCS LSP: Process completed for {} in {:.2}s", 
                    file_name, elapsed.as_secs_f64());
                output
            }
            Ok(Err(e)) => {
                let elapsed = start_time.elapsed();
                eprintln!("❌ PHPCS LSP: PHPCS process error for {} after {:.2}s: {}", 
                    file_name, elapsed.as_secs_f64(), e);
                return Err(anyhow::anyhow!("PHPCS process error for {}: {}", file_name, e));
            }
            Err(_) => {
                eprintln!("⏱️ PHPCS LSP: PHPCS timeout for {} (>10s) with {} bytes of content", 
                    file_name, text.len());
                // Process will be killed automatically due to kill_on_drop(true)
                return Err(anyhow::anyhow!("PHPCS execution timeout for {} after 10 seconds", file_name));
            }
        };
        let raw_output = String::from_utf8_lossy(&output.stdout);
        
        // Permit is automatically released when it goes out of scope
        drop(_permit);
        let available_after = self.process_semaphore.available_permits();
        eprintln!("🎫 PHPCS LSP: Released process slot for {} (slots available: {}/4)", 
            file_name, available_after);
        
        let diagnostics = self.parse_phpcs_output(&raw_output, uri).await?;

        // Log results with timing
        let total_time = start_time.elapsed();
        let issue_count = diagnostics.len();
        if issue_count == 0 {
            eprintln!("✅ PHPCS LSP: {} is clean! No issues found (took {:.2}s)", 
                file_name, total_time.as_secs_f64());
        } else {
            let errors = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::ERROR)).count();
            let warnings = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::WARNING)).count();
            let infos = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::INFORMATION)).count();

            eprintln!("📊 PHPCS LSP: {} issues found in {}: {} errors, {} warnings, {} info (took {:.2}s)",
                issue_count, file_name, errors, warnings, infos, total_time.as_secs_f64());
        }

        Ok(diagnostics)
    }

    async fn parse_phpcs_output(&self, json_output: &str, uri: &Url) -> Result<Vec<Diagnostic>> {
        // Early return if empty output
        if json_output.trim().is_empty() {
            return Ok(vec![]);
        }

        let mut diagnostics = Vec::with_capacity(10); // Pre-allocate for common case

        let phpcs_result: serde_json::Value = match serde_json::from_str(json_output) {
            Ok(result) => result,
            Err(_) => return Ok(vec![]),
        };

        if let Some(files) = phpcs_result.get("files").and_then(|f| f.as_object()) {
            for (_, file_data) in files {
                if let Some(messages) = file_data.get("messages").and_then(|m| m.as_array()) {
                    for message in messages {
                        if let Some(diagnostic) = self.convert_message_to_diagnostic(message, uri).await {
                            diagnostics.push(diagnostic);
                        }
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    async fn convert_message_to_diagnostic(&self, message: &serde_json::Value, uri: &Url) -> Option<Diagnostic> {
        let line = message.get("line")?.as_u64()? as u32;
        let column = message.get("column")?.as_u64()? as u32;
        let msg = message.get("message")?.as_str()?;
        let severity_str = message.get("type")?.as_str()?;
        let source = message.get("source")?.as_str().unwrap_or("");
        let fixable = message.get("fixable")?.as_bool().unwrap_or(false);

        let severity = match severity_str {
            "ERROR" => DiagnosticSeverity::ERROR,
            "WARNING" => DiagnosticSeverity::WARNING,
            _ => DiagnosticSeverity::INFORMATION,
        };

        // Convert to 0-based indexing for LSP
        let line = if line > 0 { line - 1 } else { 0 };
        let column = if column > 0 { column - 1 } else { 0 };

        // Determine if this is a line-level or tag-level issue
        let is_line_level = msg.contains("Line exceeds") ||
                           msg.contains("line is too long") ||
                           msg.contains("Whitespace found at end of line") ||
                           msg.contains("Line indented incorrectly") ||
                           msg.contains("separated by a single blank line") ||
                           msg.contains("blocks must be separated") ||
                           source.contains("Generic.Files.LineLength") ||
                           source.contains("Generic.WhiteSpace.DisallowTabIndent") ||
                           source.contains("Squiz.WhiteSpace.SuperfluousWhitespace") ||
                           source.contains("PSR12.Files.FileHeader.SpacingAfterBlock");

        let is_tag_level = msg.contains("closing tag") ||
                          msg.contains("Opening PHP tag") ||
                          msg.contains("<?php") ||
                          msg.contains("?>") ||
                          source.contains("PSR2.Files.ClosingTag") ||
                          source.contains("PSR12.Files.OpenTag");

        // Get the line content from the stored document
        let range = if let Ok(docs) = self.open_docs.read() {
            if let Some(compressed_doc) = docs.get(uri) {
                // Decompress to get line content
                if let Ok(content) = self.decompress_document(compressed_doc) {
                    if let Some(line_content) = content.lines().nth(line as usize) {
                    if is_line_level {
                        // Underline from first non-whitespace character to end of line
                        let first_non_whitespace = line_content.chars()
                            .position(|c| !c.is_whitespace())
                            .unwrap_or(0) as u32;
                        Range {
                            start: Position { line, character: first_non_whitespace },
                            end: Position { line, character: line_content.len() as u32 },
                        }
                    } else if is_tag_level {
                        // Find and underline the PHP tag
                        self.find_tag_range(line_content, line, column)
                    } else {
                        // Normal token-based underlining
                        self.find_token_range(line_content, line, column)
                    }
                    } else {
                        // Fallback if line not found
                        Range {
                            start: Position { line, character: column },
                            end: Position { line, character: column + 1 },
                        }
                    }
                } else {
                    // Fallback if decompression fails
                    Range {
                        start: Position { line, character: column },
                        end: Position { line, character: column + 1 },
                    }
                }
            } else {
                // Fallback if no document content
                Range {
                    start: Position { line, character: column },
                    end: Position { line, character: column + 1 },
                }
            }
        } else {
            // Fallback if lock fails
            Range {
                start: Position { line, character: column },
                end: Position { line, character: column + 1 },
            }
        };

        // Create enhanced source with standard information
        let enhanced_source = "phpcs".to_string();


        // Store additional data for potential future features
        let data = serde_json::json!({
            "fixable": fixable,
            "phpcs_source": source,
            "phpcs_severity": message.get("severity")
        });

        Some(Diagnostic {
            range,
            severity: Some(severity),
            code: if !source.is_empty() {
                Some(NumberOrString::String(source.to_string()))
            } else {
                None
            },
            source: Some(enhanced_source),
            message: msg.to_string(),
            related_information: None,
            tags: None,
            code_description: None,
            data: Some(data),
        })
    }

    fn find_tag_range(&self, line_content: &str, line: u32, column: u32) -> Range {
        let col = column as usize;

        // Look for all possible PHP tags and find the one closest to column position
        let mut best_match: Option<(usize, usize)> = None; // (start_pos, end_pos)

        // Check for opening tag "<?php"
        if let Some(pos) = line_content.find("<?php") {
            let distance = if col >= pos && col <= pos + 5 { 0 } else { col.abs_diff(pos) };
            if best_match.is_none() || distance <= col.abs_diff(best_match.unwrap().0) {
                best_match = Some((pos, pos + 5));
            }
        }

        // Check for closing tag "?>"
        if let Some(pos) = line_content.find("?>") {
            let distance = if col >= pos && col <= pos + 2 { 0 } else { col.abs_diff(pos) };
            if best_match.is_none() || distance < col.abs_diff(best_match.unwrap().0) {
                best_match = Some((pos, pos + 2));
            }
        }

        // Check for short opening tag "<?" (but only if not part of "<?php")
        let mut search_pos = 0;
        while let Some(pos) = line_content[search_pos..].find("<?") {
            let actual_pos = search_pos + pos;
            // Make sure it's not part of "<?php"
            if !line_content[actual_pos..].starts_with("<?php") {
                let distance = if col >= actual_pos && col <= actual_pos + 2 { 0 } else { col.abs_diff(actual_pos) };
                if best_match.is_none() || distance < col.abs_diff(best_match.unwrap().0) {
                    best_match = Some((actual_pos, actual_pos + 2));
                }
            }
            search_pos = actual_pos + 2;
            if search_pos >= line_content.len() { break; }
        }

        if let Some((start, end)) = best_match {
            Range {
                start: Position { line, character: start as u32 },
                end: Position { line, character: end as u32 },
            }
        } else {
            // If no tag found, underline from column position with a reasonable default
            Range {
                start: Position { line, character: column },
                end: Position { line, character: column.saturating_add(2) },
            }
        }
    }

    fn find_token_range(&self, line_content: &str, line: u32, column: u32) -> Range {
        let chars: Vec<char> = line_content.chars().collect();
        let col = column as usize;

        // If column is beyond line length, use end of line
        if col >= chars.len() {
            return Range {
                start: Position { line, character: column.saturating_sub(1) },
                end: Position { line, character: column },
            };
        }

        // Find token boundaries
        let mut start = col;
        let mut end = col;

        // Determine token type at column position
        let ch = chars[col];

        if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            // Identifier or variable token
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_' || chars[start - 1] == '$') {
                start -= 1;
            }
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
        } else if ch.is_whitespace() {
            // For whitespace issues, highlight the space
            while end < chars.len() && chars[end].is_whitespace() {
                end += 1;
            }
        } else {
            // Operator or punctuation
            let operator_chars = ['=', '!', '<', '>', '+', '-', '*', '/', '%', '&', '|', '^', '~'];
            if operator_chars.contains(&ch) {
                // Check for multi-character operators
                while end < chars.len() && operator_chars.contains(&chars[end]) {
                    end += 1;
                }
                // Also check backward for multi-char operators
                while start > 0 && operator_chars.contains(&chars[start - 1]) {
                    start -= 1;
                }
            } else {
                // Single character token (parenthesis, bracket, semicolon, etc.)
                end = col + 1;
            }
        }

        // Ensure we have at least one character highlighted
        if start == end {
            end = (col + 1).min(chars.len());
        }

        Range {
            start: Position { line, character: start as u32 },
            end: Position { line, character: end as u32 },
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for PhpcsLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        eprintln!("🚀 PHPCS LSP: Server initializing...");
        eprintln!("🔧 PHPCS LSP: Client info: {:?}", params.client_info);

        // Determine workspace root for config file lookup
        let workspace_root = params.root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().ok());

        if let Some(ref root) = workspace_root {
            eprintln!("📁 PHPCS LSP: Workspace root: {}", root.display());
        } else {
            eprintln!("❌ PHPCS LSP: No workspace root detected");
        }

        // Store workspace root for PHPCS path detection
        if let Ok(mut workspace_guard) = self.workspace_root.write() {
            *workspace_guard = workspace_root.clone();
        }

        if let Some(options) = params.initialization_options {
            // Parse initialization options
            eprintln!("📦 PHPCS LSP: Processing initialization options from extension");
            match serde_json::from_value::<InitializationOptions>(options.clone()) {
                Ok(init_options) => {
                    if let Some(standard) = init_options.standard {
                        eprintln!("⚙️ PHPCS LSP: Extension provided standard: '{}'", standard);
                        if let Ok(mut standard_guard) = self.standard.write() {
                            *standard_guard = Some(standard.clone());
                        }
                    } else {
                        eprintln!("🎯 PHPCS LSP: No standard provided by extension - will use PHPCS defaults");
                    }
                },
                Err(e) => {
                    eprintln!("❌ PHPCS LSP: Failed to parse initialization options: {}", e);
                }
            }
        } else {
            // No initialization options provided, discover from workspace
            self.discover_standard(workspace_root.as_deref());
        }

        // Log final initialization state
        if let Ok(standard_guard) = self.standard.read() {
            match &*standard_guard {
                Some(standard) => eprintln!("🎯 PHPCS LSP: Initialized with standard: '{}'", standard),
                None => eprintln!("🎯 PHPCS LSP: Initialized with no explicit standard (PHPCS defaults)"),
            }
        }

        eprintln!("✅ PHPCS LSP: Server initialization complete!");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("phpcs".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        ..Default::default()
                    },
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        eprintln!("🎉 PHPCS LSP: Server is ready and operational!");
        // Pre-cache the PHPCS path on initialization
        let _ = self.get_phpcs_path();
        eprintln!("🚀 PHPCS LSP: Ready to lint PHP files!");
    }

    async fn shutdown(&self) -> LspResult<()> {
        eprintln!("🔄 PHPCS LSP: Shutting down, clearing caches...");

        // Clear all cached data on shutdown
        if let Ok(mut docs) = self.open_docs.write() {
            docs.clear();
        }
        if let Ok(mut cache) = self.results_cache.write() {
            cache.clear();
        }

        // Reset memory counter
        self.total_memory_usage.store(0, Ordering::Relaxed);

        eprintln!("✅ PHPCS LSP: Shutdown complete");
        Ok(())
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clear document from memory to prevent memory leaks
        let uri = params.text_document.uri;

        // Remove compressed document and update memory tracking
        if let Ok(mut docs) = self.open_docs.write() {
            if let Some(doc) = docs.remove(&uri) {
                let freed_memory = doc.compressed_data.len();
                self.total_memory_usage.fetch_sub(freed_memory, Ordering::Relaxed);
                eprintln!("🗑️ PHPCS LSP: Closed file, freed {}KB, total memory: {:.1}MB",
                    freed_memory / 1024,
                    self.get_memory_usage_mb()
                );
            }
        }

        // Clear cached results
        if let Ok(mut cache) = self.results_cache.write() {
            cache.remove(&uri);
        }

        // Clear diagnostics for closed file
        let _ = self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn did_change_workspace_folders(&self, _params: DidChangeWorkspaceFoldersParams) {
        // Clear cached PHPCS path when workspace changes
        if let Ok(mut guard) = self.phpcs_path.write() {
            *guard = None;
        }

        // Clear results cache as paths may have changed
        if let Ok(mut cache) = self.results_cache.write() {
            cache.clear();
        }

        eprintln!("🔄 PHPCS LSP: Workspace changed, cleared caches");

        // Re-detect PHPCS configuration for new workspace
        // This will be done lazily on next PHPCS run
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        eprintln!("🔄 PHPCS LSP: Configuration change detected!");

        // Clear cached PHPCS path to force re-detection
        if let Ok(mut guard) = self.phpcs_path.write() {
            *guard = None;
            eprintln!("🗑️ PHPCS LSP: Cleared cached PHPCS path - will re-detect on next use");
        }

        // Parse the settings
        if let Some(settings) = params.settings.as_object() {
            // Look for phpcs settings
            if let Some(phpcs_settings) = settings.get("phpcs") {
                // Try to parse as PhpcsSettings
                if let Ok(parsed_settings) = serde_json::from_value::<PhpcsSettings>(phpcs_settings.clone()) {
                    // Update the standard if provided
                    if let Some(new_standard) = parsed_settings.standard {
                        if let Ok(mut standard_guard) = self.standard.write() {
                            *standard_guard = Some(new_standard);
                        }
                    }
                }
            }

            // Also check for standard directly in settings (for compatibility)
            if let Some(standard_value) = settings.get("standard") {
                if let Some(new_standard) = standard_value.as_str() {
                    if let Ok(mut standard_guard) = self.standard.write() {
                        *standard_guard = Some(new_standard.to_string());
                    }
                }
            }
        }

        // Clear results cache to force re-linting with new config
        if let Ok(mut cache) = self.results_cache.write() {
            cache.clear();
            eprintln!("🗑️ PHPCS LSP: Cleared results cache after config change");
        }

        // Note: Documents will be re-linted on next diagnostic() call
        // No need to proactively re-run PHPCS on all files
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;

        let file_name = uri.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");

        eprintln!("📂 PHPCS LSP: File opened: {} ({} bytes)", file_name, text.len());

        // Compress and store the document
        let compressed_doc = self.compress_document(&text);

        {
            let mut docs = self.open_docs.write().unwrap();
            docs.insert(uri.clone(), compressed_doc);

            // Log memory stats on significant changes
            if docs.len() % 25 == 0 {
                drop(docs); // Release lock before logging
                self.log_memory_stats();
            }
        }

        // Invalidate any cached results for this file
        if let Ok(mut cache) = self.results_cache.write() {
            cache.remove(&uri);
        }

        // Log memory stats periodically (every 10 files)
        if let Ok(docs) = self.open_docs.read() {
            if docs.len() % 10 == 0 {
                drop(docs); // Release lock before logging
                self.log_memory_stats();
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        // With FULL sync, we always get the complete document content
        if let Some(change) = params.content_changes.first() {
            // Remove old compressed document to update memory tracking
            let old_size = if let Ok(docs) = self.open_docs.read() {
                docs.get(&uri).map(|doc| doc.compressed_data.len())
            } else {
                None
            };

            if let Some(size) = old_size {
                self.total_memory_usage.fetch_sub(size, Ordering::Relaxed);
            }

            // Compress and store new content
            let compressed_doc = self.compress_document(&change.text);

            let mut docs = self.open_docs.write().unwrap();
            docs.insert(uri.clone(), compressed_doc);

            // Invalidate cached results since content changed
            if let Ok(mut cache) = self.results_cache.write() {
                cache.remove(&uri);
            }
        }

        // Diagnostics will be provided via diagnostic() method
        // This reduces unnecessary PHPCS runs during rapid typing
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        let file_name = uri.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");

        eprintln!("💾 PHPCS LSP: File saved: {}", file_name);

        // Note: Diagnostics will be provided via diagnostic() method calls from Zed
        // We don't need to proactively run PHPCS here to avoid duplicate linting
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> LspResult<DocumentDiagnosticReportResult> {
        let uri = params.text_document.uri;
        let file_name = uri.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");

        if let Ok(file_path) = uri.to_file_path() {
            if let Some(path_str) = file_path.to_str() {
                // First check if we have cached results
                if let Ok(cache) = self.results_cache.read() {
                    if let Some(cached) = cache.get(&uri) {
                        eprintln!("⚡ PHPCS LSP: Using cached results for {} (age: {:.1}s)",
                            file_name,
                            cached.generated_at.elapsed().as_secs_f64()
                        );

                        // Check if client has the same version
                        if let Some(previous_result_id) = params.previous_result_id {
                            if previous_result_id == cached.result_id {
                                eprintln!("✅ PHPCS LSP: Client has current version for {}", file_name);
                                return Ok(DocumentDiagnosticReportResult::Report(
                                    DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                                        unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                                            result_id: cached.result_id.clone(),
                                        },
                                        related_documents: None,
                                    }),
                                ));
                            }
                        }

                        // Return cached diagnostics
                        return Ok(DocumentDiagnosticReportResult::Report(
                            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                    result_id: Some(cached.result_id.clone()),
                                    items: cached.diagnostics.clone(),
                                },
                                related_documents: None,
                            }),
                        ));
                    }
                }

                // No cached results, need to get content and run PHPCS
                let compressed_doc = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };

                // Handle missing document (rare edge case)
                let compressed_doc = if compressed_doc.is_none() {
                    // Try to read from disk as fallback
                    match fs::read_to_string(path_str) {
                        Ok(file_content) => {
                            eprintln!("⚠️ PHPCS LSP: Document not in memory, reading from disk: {}", file_name);
                            let compressed = self.compress_document(&file_content);
                            let mut docs = self.open_docs.write().unwrap();
                            docs.insert(uri.clone(), compressed.clone());
                            Some(compressed)
                        }
                        Err(e) => {
                            eprintln!("❌ PHPCS LSP: Failed to read file {}: {}", file_name, e);
                            None
                        }
                    }
                } else {
                    compressed_doc
                };

                if let Some(compressed_doc) = compressed_doc {
                    // Decompress content
                    let content = match self.decompress_document(&compressed_doc) {
                        Ok(content) => content,
                        Err(e) => {
                            eprintln!("❌ PHPCS LSP: Failed to decompress {}: {}", file_name, e);
                            return Ok(DocumentDiagnosticReportResult::Report(
                                DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                                    full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                        result_id: None,
                                        items: vec![],
                                    },
                                    related_documents: None,
                                }),
                            ));
                        }
                    };

                    let version_id = compressed_doc.checksum.clone();
                    eprintln!("📋 PHPCS LSP: Running PHPCS for {} with version: {}", file_name, &version_id[..16]);

                    // Run PHPCS
                    if let Ok(diagnostics) = self.run_phpcs(&uri, path_str, Some(&content)).await {
                        eprintln!("📊 PHPCS LSP: Generated {} diagnostics for {}",
                            diagnostics.len(), file_name);

                        // Cache the results
                        let cached_results = CachedResults {
                            diagnostics: diagnostics.clone(),
                            result_id: version_id.clone(),
                            generated_at: Instant::now(),
                        };

                        if let Ok(mut cache) = self.results_cache.write() {
                            cache.insert(uri.clone(), cached_results);
                        }

                        return Ok(DocumentDiagnosticReportResult::Report(
                            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                    result_id: Some(version_id),
                                    items: diagnostics,
                                },
                                related_documents: None,
                            }),
                        ));
                    }
                }
            }
        }

        // Fallback: return empty diagnostics with no version
        eprintln!("⚠️ PHPCS LSP: Unable to generate diagnostics for {}", file_name);
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![],
                },
                related_documents: None,
            }),
        ))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = stdin();
    let stdout = stdout();

    let (service, socket) = LspService::new(|client| PhpcsLanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
