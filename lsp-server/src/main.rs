use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command as ProcessCommand;
use tokio::io::{stdin, stdout};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
struct InitializationOptions {
    #[serde(rename = "phpcsPath")]
    phpcs_path: Option<String>,
}

#[derive(Debug)]
struct PhpcsLanguageServer {
    client: Client,
    phpcs_path: std::sync::Arc<std::sync::RwLock<Option<String>>>,
    open_docs: std::sync::Arc<std::sync::RwLock<HashMap<Url, String>>>,
}

impl PhpcsLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            phpcs_path: std::sync::Arc::new(std::sync::RwLock::new(None)),
            open_docs: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    async fn run_phpcs(&self, uri: &Url, file_path: &str, content: Option<&str>) -> Result<Vec<Diagnostic>> {
        let phpcs_path = {
            let path_guard = self.phpcs_path.read().unwrap();
            path_guard.clone().unwrap_or_else(|| {
                eprintln!("PHPCS LSP: No phpcsPath provided via initialization options");
                
                // Try to find bundled PHPCS PHAR relative to LSP server
                if let Ok(current_exe) = std::env::current_exe() {
                    if let Some(exe_dir) = current_exe.parent() {
                        // Look for PHPCS PHAR in same directory as LSP server
                        let bundled_phpcs = exe_dir.join("phpcs.phar");
                        eprintln!("PHPCS LSP: Checking for PHPCS at: {}", bundled_phpcs.display());
                        
                        if bundled_phpcs.exists() {
                            eprintln!("PHPCS LSP: Found bundled PHPCS PHAR in LSP directory");
                            return bundled_phpcs.to_string_lossy().to_string();
                        }
                    }
                }
                
                eprintln!("PHPCS LSP: No bundled PHPCS found, trying system phpcs");
                "phpcs".to_string()
            })
        };

        eprintln!("PHPCS LSP: Using PHPCS path: {}", phpcs_path);
        
        // Debug: Check if the PHPCS path actually exists
        if let Ok(metadata) = std::fs::metadata(&phpcs_path) {
            #[cfg(unix)]
            {
                eprintln!("PHPCS LSP: PHPCS binary exists, size: {} bytes, executable: {}", 
                         metadata.len(), metadata.permissions().mode() & 0o111 != 0);
            }
            #[cfg(not(unix))]
            {
                eprintln!("PHPCS LSP: PHPCS binary exists, size: {} bytes", metadata.len());
            }
        } else {
            eprintln!("PHPCS LSP: PHPCS binary does not exist at: {}", phpcs_path);
        }

        let mut cmd = ProcessCommand::new(&phpcs_path);
        cmd.arg("--report=json")
           .arg("--no-colors")
           .arg("-q")
           .arg("--standard=PSR12");

        if let Some(text) = content {
            cmd.arg("-");
            cmd.stdin(std::process::Stdio::piped())
               .stdout(std::process::Stdio::piped())
               .stderr(std::process::Stdio::piped());
            
            eprintln!("PHPCS LSP: Spawning command with stdin");
            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    eprintln!("PHPCS LSP: Failed to spawn command: {}", e);
                    eprintln!("PHPCS LSP: Command was: {}", phpcs_path);
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
            self.parse_phpcs_output(&raw_output, uri).await
        } else {
            cmd.arg(file_path);
            eprintln!("PHPCS LSP: Running command on file: {}", file_path);
            let output = match cmd.output() {
                Ok(output) => output,
                Err(e) => {
                    eprintln!("PHPCS LSP: Failed to run command: {}", e);
                    eprintln!("PHPCS LSP: Command was: {}", phpcs_path);
                    return Err(anyhow::anyhow!("PHPCS error: {}", e));
                }
            };
            let raw_output = String::from_utf8_lossy(&output.stdout);
            self.parse_phpcs_output(&raw_output, uri).await
        }
    }

    async fn parse_phpcs_output(&self, json_output: &str, uri: &Url) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        
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
        eprintln!("PHPCS LSP: Initialize called");
        
        if let Some(options) = params.initialization_options {
            eprintln!("PHPCS LSP: Received initialization options: {:?}", options);
            if let Ok(init_opts) = serde_json::from_value::<InitializationOptions>(options) {
                eprintln!("PHPCS LSP: Parsed initialization options successfully");
                if let Some(ref phpcs_path) = init_opts.phpcs_path {
                    eprintln!("PHPCS LSP: Setting phpcsPath to: {}", phpcs_path);
                    *self.phpcs_path.write().unwrap() = Some(phpcs_path.clone());
                } else {
                    eprintln!("PHPCS LSP: No phpcsPath in initialization options");
                }
            } else {
                eprintln!("PHPCS LSP: Failed to parse initialization options");
            }
        } else {
            eprintln!("PHPCS LSP: No initialization options received");
        }

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
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        eprintln!("PHPCS Language Server initialized");
        
        // Debug: Check current working directory and PHPCS path
        if let Ok(cwd) = std::env::current_dir() {
            eprintln!("PHPCS LSP: Server working directory: {}", cwd.display());
        }
        
        if let Ok(current_exe) = std::env::current_exe() {
            eprintln!("PHPCS LSP: Server executable: {}", current_exe.display());
            if let Some(exe_dir) = current_exe.parent() {
                let phpcs_path = exe_dir.join("bin").join("phpcs.phar");
                eprintln!("PHPCS LSP: Looking for PHPCS at: {}", phpcs_path.display());
                eprintln!("PHPCS LSP: PHPCS exists: {}", phpcs_path.exists());
            }
        }
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        
        {
            let mut docs = self.open_docs.write().unwrap();
            docs.insert(uri.clone(), text);
        }

        if let Ok(file_path) = uri.to_file_path() {
            if let Some(path_str) = file_path.to_str() {
                let content = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };
                
                if let Ok(diagnostics) = self.run_phpcs(&uri, path_str, content.as_deref()).await {
                    let _ = self.client.publish_diagnostics(uri, diagnostics, None).await;
                }
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        if let Some(change) = params.content_changes.first() {
            let mut docs = self.open_docs.write().unwrap();
            docs.insert(uri.clone(), change.text.clone());
        }

        if let Ok(file_path) = uri.to_file_path() {
            if let Some(path_str) = file_path.to_str() {
                let content = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };
                
                if let Ok(diagnostics) = self.run_phpcs(&uri, path_str, content.as_deref()).await {
                    let _ = self.client.publish_diagnostics(uri, diagnostics, None).await;
                }
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Ok(file_path) = uri.to_file_path() {
            if let Some(path_str) = file_path.to_str() {
                let content = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };
                
                if let Ok(diagnostics) = self.run_phpcs(&uri, path_str, content.as_deref()).await {
                    let _ = self.client.publish_diagnostics(uri, diagnostics, None).await;
                }
            }
        }
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> LspResult<DocumentDiagnosticReportResult> {
        let uri = params.text_document.uri;

        if let Ok(file_path) = uri.to_file_path() {
            if let Some(path_str) = file_path.to_str() {
                let content = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };
                
                if content.is_none() {
                    // Try reading from disk if not in memory
                    eprintln!("PHPCS LSP: Document not in memory, trying to read from disk: {}", path_str);
                    match fs::read_to_string(path_str) {
                        Ok(file_content) => {
                            eprintln!("PHPCS LSP: Successfully read {} bytes from disk", file_content.len());
                            let mut docs = self.open_docs.write().unwrap();
                            docs.insert(uri.clone(), file_content.clone());
                            drop(docs);
                        }
                        Err(e) => {
                            eprintln!("PHPCS LSP: Failed to read file from disk: {}", e);
                        }
                    }
                } else {
                    eprintln!("PHPCS LSP: Using document content from memory");
                }
                
                let content = {
                    let docs = self.open_docs.read().unwrap();
                    docs.get(&uri).cloned()
                };
                
                eprintln!("PHPCS LSP: Diagnostic request for: {}", path_str);
                eprintln!("PHPCS LSP: Content in memory: {}", content.is_some());
                
                if let Ok(diagnostics) = self.run_phpcs(&uri, path_str, content.as_deref()).await {
                    eprintln!("PHPCS LSP: Found {} diagnostics", diagnostics.len());
                    return Ok(DocumentDiagnosticReportResult::Report(
                        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                result_id: None,
                                items: diagnostics,
                            },
                            related_documents: None,
                        }),
                    ));
                } else {
                    eprintln!("PHPCS LSP: Failed to get diagnostics");
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