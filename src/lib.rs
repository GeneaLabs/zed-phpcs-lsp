use zed_extension_api::{self as zed, Result};
use std::{env, fs};

struct PhpcsLspExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for PhpcsLspExtension {
    fn new() -> Self {
        eprintln!("🚀 PHPCS LSP Extension: new() called - extension is loading!");
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let language_server_id = language_server_id.as_ref();
        eprintln!("🚀 PHPCS LSP: language_server_command called with ID: {}", language_server_id);
        eprintln!("🚀 PHPCS LSP: worktree path: {:?}", worktree.root_path());
        
        if language_server_id != "phpcs-lsp-server" {
            eprintln!("PHPCS LSP: Unknown language server ID: {}", language_server_id);
            return Err(format!("Unknown language server: {}", language_server_id).into());
        }

        // Try to find the LSP server binary
        eprintln!("PHPCS LSP: About to call find_lsp_server_binary");
        let lsp_server_path = match self.find_lsp_server_binary(worktree) {
            Ok(path) => {
                eprintln!("PHPCS LSP: Successfully found LSP server at: {}", path);
                path
            }
            Err(e) => {
                eprintln!("PHPCS LSP: Failed to find LSP server: {}", e);
                return Err(e);
            }
        };
        
        eprintln!("PHPCS LSP: Returning command with path: {}", lsp_server_path);
        Ok(zed::Command {
            command: lsp_server_path.to_string(),
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        eprintln!("🔧 PHPCS LSP: language_server_initialization_options called for: {}", language_server_id.as_ref());
        eprintln!("🔧 PHPCS LSP: worktree path: {:?}", worktree.root_path());
        let mut options = zed::serde_json::Map::new();
        
        // Try to find phpcs binary (bundled PHAR or user-provided)
        if let Some(phpcs_path) = Self::find_phpcs_binary(worktree) {
            eprintln!("PHPCS LSP: Found PHPCS: {}", phpcs_path);
            options.insert("phpcsPath".to_string(), zed::serde_json::Value::String(phpcs_path));
        } else {
            eprintln!("PHPCS LSP: No PHPCS found via worktree.which(), trying absolute path");
            // Fallback: try to find bundled PHPCS using absolute path from worktree root
            let worktree_root = worktree.root_path();
            let worktree_path = std::path::Path::new(&worktree_root);
            let bundled_phpcs = worktree_path.join("bin/phpcs.phar");
            if bundled_phpcs.exists() {
                let phpcs_path = bundled_phpcs.to_string_lossy().to_string();
                eprintln!("PHPCS LSP: Found bundled PHPCS at absolute path: {}", phpcs_path);
                options.insert("phpcsPath".to_string(), zed::serde_json::Value::String(phpcs_path));
            } else {
                eprintln!("PHPCS LSP: No bundled PHPCS found at: {}", bundled_phpcs.display());
            }
        }
        
        // Try to find phpcbf binary (PHIVE-managed PHAR or user-provided)
        if let Some(phpcbf_path) = Self::find_phpcbf_binary(worktree) {
            eprintln!("PHPCS LSP: Found PHPCBF: {}", phpcbf_path);
            options.insert("phpcbfPath".to_string(), zed::serde_json::Value::String(phpcbf_path));
        } else {
            eprintln!("PHPCS LSP: No PHPCBF PHAR found");
        }
        
        // Try to find phpcs configuration file
                if let Some(config_file) = Self::find_phpcs_config(worktree) {
                    eprintln!("PHPCS LSP: Found phpcs config: {}", config_file);
                    options.insert("configFile".to_string(), zed::serde_json::Value::String(config_file));
                } else {
                    eprintln!("PHPCS LSP: No phpcs config found");
                }

                // Environment override for coding standard
                if let Ok(env_standard) = env::var("PHPCS_STANDARD") {
                    if !env_standard.trim().is_empty() {
                        eprintln!("PHPCS LSP: Using standard from PHPCS_STANDARD env: {}", env_standard);
                        options.insert("standard".to_string(), zed::serde_json::Value::String(env_standard));
                    }
                }
        
                eprintln!("PHPCS LSP: Initialization options: {:?}", options);
        
        if options.is_empty() {
            Ok(None)
        } else {
            Ok(Some(zed::serde_json::Value::Object(options)))
        }
    }
}

impl PhpcsLspExtension {
    fn find_lsp_server_binary(&mut self, _worktree: &zed::Worktree) -> Result<String> {
        eprintln!("PHPCS LSP: Searching for LSP server binary...");
        
        // Check if we already have a cached binary path
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok() {
                eprintln!("PHPCS LSP: Using cached binary: {}", path);
                return Ok(path.clone());
            }
        }

        // For Zed extensions, the binary name should be just the name
        // Zed will look for it in the extension directory
        let binary_name = "phpcs-lsp-server".to_string();
        eprintln!("PHPCS LSP: Returning binary path: {}", binary_name);
        self.cached_binary_path = Some(binary_name.clone());
        Ok(binary_name)
    }

    fn find_phpcs_binary(worktree: &zed::Worktree) -> Option<String> {
        eprintln!("PHPCS LSP: Searching for PHPCS binary...");
        
        // First try project-specific PHPCS
        if let Some(path) = worktree.which("vendor/bin/phpcs") {
            eprintln!("PHPCS LSP: Found project PHPCS at: {}", path);
            return Some(path);
        }
        
        // Try bundled PHPCS PHAR using worktree.which()
        if let Some(path) = worktree.which("bin/phpcs.phar") {
            eprintln!("PHPCS LSP: Found bundled PHPCS PHAR at: {}", path);
            return Some(path);
        }
        
        eprintln!("PHPCS LSP: No PHPCS found, will use system phpcs");
        None
    }

    fn find_phpcbf_binary(worktree: &zed::Worktree) -> Option<String> {
        eprintln!("PHPCS LSP: Searching for PHPCBF binary...");
        
        // First try project-specific PHPCBF
        if let Some(path) = worktree.which("vendor/bin/phpcbf") {
            eprintln!("PHPCS LSP: Found project PHPCBF at: {}", path);
            return Some(path);
        }
        
        // Try bundled PHPCBF PHAR using worktree.which()
        if let Some(path) = worktree.which("bin/phpcbf.phar") {
            eprintln!("PHPCS LSP: Found bundled PHPCBF PHAR at: {}", path);
            return Some(path);
        }
        
        eprintln!("PHPCS LSP: No PHPCBF found");
        None
    }
    
    fn find_phpcs_config(worktree: &zed::Worktree) -> Option<String> {
        let config_files = [
            ".phpcs.xml",
            ".phpcs.xml.dist", 
            "phpcs.xml",
            "phpcs.xml.dist",
            "phpcs.ruleset.xml",
        ];
        
        for config_file in &config_files {
            if let Some(path) = worktree.which(config_file) {
                return Some(path);
            }
        }
        
        None
    }
}

zed::register_extension!(PhpcsLspExtension);
