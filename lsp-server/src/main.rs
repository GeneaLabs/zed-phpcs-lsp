use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Command as ProcessCommand;
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
struct PhpcsLanguageServer {
    client: Client,
    open_docs: std::sync::Arc<std::sync::RwLock<HashMap<Url, String>>>,
    standard: std::sync::Arc<std::sync::RwLock<Option<String>>>,  // None means use PHPCS defaults
    phpcs_path: std::sync::Arc<std::sync::RwLock<Option<String>>>,
}

impl PhpcsLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            open_docs: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            standard: std::sync::Arc::new(std::sync::RwLock::new(None)),  // Let PHPCS use its defaults
            phpcs_path: std::sync::Arc::new(std::sync::RwLock::new(None)),
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
        let phpcs_path = if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                let bundled_phpcs = exe_dir.join("phpcs.phar");
                eprintln!("🔍 PHPCS LSP: Checking for bundled PHPCS at: {}", bundled_phpcs.display());
                
                if bundled_phpcs.exists() {
                    eprintln!("✅ PHPCS LSP: Found bundled PHPCS PHAR");
                    bundled_phpcs.to_string_lossy().to_string()
                } else {
                    eprintln!("❌ PHPCS LSP: No bundled PHPCS found, using system phpcs");
                    "phpcs".to_string()
                }
            } else {
                eprintln!("❌ PHPCS LSP: Could not get LSP directory");
                "phpcs".to_string()
            }
        } else {
            eprintln!("❌ PHPCS LSP: Could not get current executable path");
            "phpcs".to_string()
        };
        
        eprintln!("🎯 PHPCS LSP: Final PHPCS path: {}", phpcs_path);
        
        // Cache the result
        if let Ok(mut guard) = self.phpcs_path.write() {
            *guard = Some(phpcs_path.clone());
        }
        
        phpcs_path
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
        let file_name = uri.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");
        
        eprintln!("🔍 PHPCS LSP: Starting lint for file: {}", file_name);
        
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
        cmd.arg("-");
        
        eprintln!("🚀 PHPCS LSP: Running PHPCS on {}{}", file_name, standard_info);
        cmd.stdin(std::process::Stdio::piped())
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("❌ PHPCS LSP: Failed to spawn PHPCS for {}: {}", file_name, e);
                return Err(anyhow::anyhow!("PHPCS error: {}", e));
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
            drop(stdin);
        }

        let output = child.wait_with_output()?;
        let raw_output = String::from_utf8_lossy(&output.stdout);
        
        
        let diagnostics = self.parse_phpcs_output(&raw_output, uri).await?;
        
        // Log results
        let issue_count = diagnostics.len();
        if issue_count == 0 {
            eprintln!("✅ PHPCS LSP: {} is clean! No issues found", file_name);
        } else {
            let errors = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::ERROR)).count();
            let warnings = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::WARNING)).count();
            let infos = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::INFORMATION)).count();
            
            eprintln!("📊 PHPCS LSP: {} issues found in {}: {} errors, {} warnings, {} info", 
                issue_count, file_name, errors, warnings, infos);
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
            if let Some(content) = docs.get(uri) {
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
        // Clear cached data on shutdown
        if let Ok(mut docs) = self.open_docs.write() {
            docs.clear();
        }
        Ok(())
    }
    
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clear document from memory to prevent memory leaks
        let uri = params.text_document.uri;
        if let Ok(mut docs) = self.open_docs.write() {
            docs.remove(&uri);
        }
        
        // Clear diagnostics for closed file
        let _ = self.client.publish_diagnostics(uri, vec![], None).await;
    }
    
    async fn did_change_workspace_folders(&self, _params: DidChangeWorkspaceFoldersParams) {
        // Clear cached PHPCS path when workspace changes
        if let Ok(mut guard) = self.phpcs_path.write() {
            *guard = None;
        }
        
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
        
        // Re-run diagnostics for all open documents with new settings
        if let Ok(docs) = self.open_docs.read() {
            for (uri, content) in docs.iter() {
                if let Ok(file_path) = uri.to_file_path() {
                    if let Some(path_str) = file_path.to_str() {
                        let content_clone = content.clone();
                        let uri_clone = uri.clone();
                        let self_clone = self.clone();
                        let path_str_owned = path_str.to_string();
                        
                        // Spawn async task to avoid blocking
                        tokio::spawn(async move {
                            if let Ok(diagnostics) = self_clone.run_phpcs(&uri_clone, &path_str_owned, Some(&content_clone)).await {
                                let _ = self_clone.client.publish_diagnostics(uri_clone, diagnostics, None).await;
                            }
                        });
                    }
                }
            }
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        
        let file_name = uri.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("unknown");
        
        eprintln!("📂 PHPCS LSP: File opened: {} ({} bytes)", file_name, text.len());

        // Just store the document content - diagnostics will be provided via diagnostic() method
        {
            let mut docs = self.open_docs.write().unwrap();
            docs.insert(uri.clone(), text);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        // With FULL sync, we always get the complete document content
        if let Some(change) = params.content_changes.first() {
            let mut docs = self.open_docs.write().unwrap();
            docs.insert(uri.clone(), change.text.clone());
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

        if let Ok(file_path) = uri.to_file_path() {
            if let Some(path_str) = file_path.to_str() {
                // Always prefer in-memory content
                let content = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };
                
                // Only read from disk if document not in memory (rare case)
                let content = if content.is_none() {
                    match fs::read_to_string(path_str) {
                        Ok(file_content) => {
                            let mut docs = self.open_docs.write().unwrap();
                            docs.insert(uri.clone(), file_content.clone());
                            drop(docs);
                            
                            let docs = self.open_docs.read().unwrap();
                            docs.get(&uri).cloned()
                        }
                        Err(_) => None
                    }
                } else {
                    content
                };

                if let Some(content) = content {
                    if let Ok(diagnostics) = self.run_phpcs(&uri, path_str, Some(&content)).await {
                        return Ok(DocumentDiagnosticReportResult::Report(
                            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                    result_id: None,
                                    items: diagnostics,
                                },
                                related_documents: None,
                            }),
                        ));
                    }
                }
            }
        }

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
