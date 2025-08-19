use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
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

    async fn run_phpcs(&self, file_path: &str, content: Option<&str>) -> Result<Vec<Diagnostic>> {
        let phpcs_path = {
            let path_guard = self.phpcs_path.read().unwrap();
            path_guard.clone().unwrap_or_else(|| {
                eprintln!("PHPCS LSP: No phpcsPath provided via initialization options");
                
                // Try to find bundled PHPCS PHAR relative to LSP server
                if let Ok(current_exe) = std::env::current_exe() {
                    if let Some(exe_dir) = current_exe.parent() {
                        // Look for PHPCS PHAR in bin subdirectory
                        let bundled_phpcs = exe_dir.join("bin").join("phpcs.phar");
                        eprintln!("PHPCS LSP: Checking for bundled PHPCS at: {}", bundled_phpcs.display());
                        
                        if bundled_phpcs.exists() {
                            eprintln!("PHPCS LSP: Found bundled PHPCS PHAR");
                            return bundled_phpcs.to_string_lossy().to_string();
                        }
                        
                        // Also try in same directory as LSP server
                        let bundled_phpcs2 = exe_dir.join("phpcs.phar");
                        eprintln!("PHPCS LSP: Checking for PHPCS at: {}", bundled_phpcs2.display());
                        
                        if bundled_phpcs2.exists() {
                            eprintln!("PHPCS LSP: Found bundled PHPCS PHAR in LSP directory");
                            return bundled_phpcs2.to_string_lossy().to_string();
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
            eprintln!("PHPCS LSP: PHPCS binary exists, size: {} bytes, executable: {}", 
                     metadata.len(), metadata.permissions().mode() & 0o111 != 0);
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
            self.parse_phpcs_output(&raw_output).await
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
            self.parse_phpcs_output(&raw_output).await
        }
    }

    async fn parse_phpcs_output(&self, json_output: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        
        let phpcs_result: serde_json::Value = match serde_json::from_str(json_output) {
            Ok(result) => result,
            Err(_) => return Ok(vec![]),
        };
        
        if let Some(files) = phpcs_result.get("files").and_then(|f| f.as_object()) {
            for (_, file_data) in files {
                if let Some(messages) = file_data.get("messages").and_then(|m| m.as_array()) {
                    for message in messages {
                        if let Some(diagnostic) = self.convert_message_to_diagnostic(message).await {
                            diagnostics.push(diagnostic);
                        }
                    }
                }
            }
        }

        Ok(diagnostics)
    }

    async fn convert_message_to_diagnostic(&self, message: &serde_json::Value) -> Option<Diagnostic> {
        let line = message.get("line")?.as_u64()? as u32;
        let column = message.get("column")?.as_u64()? as u32;
        let msg = message.get("message")?.as_str()?;
        let severity_str = message.get("type")?.as_str()?;

        let severity = match severity_str {
            "ERROR" => DiagnosticSeverity::ERROR,
            "WARNING" => DiagnosticSeverity::WARNING,
            _ => DiagnosticSeverity::INFORMATION,
        };

        // Convert to 0-based indexing for LSP
        let line = if line > 0 { line - 1 } else { 0 };
        let column = if column > 0 { column - 1 } else { 0 };
        
        // Simple range - just highlight a few characters
        let range = Range {
            start: Position { line, character: column },
            end: Position { line, character: column + 3 },
        };

        Some(Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            source: Some("phpcs".to_string()),
            message: msg.to_string(),
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        })
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
                
                if let Ok(diagnostics) = self.run_phpcs(path_str, content.as_deref()).await {
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
                
                if let Ok(diagnostics) = self.run_phpcs(path_str, content.as_deref()).await {
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
                
                if let Ok(diagnostics) = self.run_phpcs(path_str, content.as_deref()).await {
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
                
                if let Ok(diagnostics) = self.run_phpcs(path_str, content.as_deref()).await {
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