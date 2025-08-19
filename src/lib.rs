use zed_extension_api::{self as zed, Result};
use std::env;

struct PhpcsLspExtension {
    phpcs_lsp: Option<PhpcsLspServer>,
}

struct PhpcsLspServer;

impl PhpcsLspServer {
    const LANGUAGE_SERVER_ID: &'static str = "phpcs-lsp-server";

    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_name = Self::get_platform_binary_name();
        Ok(zed::Command {
            command: format!("bin/{}", binary_name),
            args: vec![],
            env: Default::default(),
        })
    }

    fn get_platform_binary_name() -> String {
        #[cfg(target_os = "windows")]
        {
            if cfg!(target_arch = "x86_64") {
                "phpcs-lsp-server-windows-x64.exe"
            } else if cfg!(target_arch = "aarch64") {
                "phpcs-lsp-server-windows-arm64.exe"
            } else {
                "phpcs-lsp-server.exe"
            }
        }
        #[cfg(target_os = "macos")]
        {
            if cfg!(target_arch = "aarch64") {
                "phpcs-lsp-server-macos-arm64"
            } else if cfg!(target_arch = "x86_64") {
                "phpcs-lsp-server-macos-x64"
            } else {
                "phpcs-lsp-server"
            }
        }
        #[cfg(target_os = "linux")]
        {
            if cfg!(target_arch = "x86_64") {
                "phpcs-lsp-server-linux-x64"
            } else if cfg!(target_arch = "aarch64") {
                "phpcs-lsp-server-linux-arm64"
            } else {
                "phpcs-lsp-server"
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            "phpcs-lsp-server"
        }
        .to_string()
    }
}

impl zed::Extension for PhpcsLspExtension {
    fn new() -> Self {
        eprintln!("🚀 PHPCS LSP Extension: new() called - extension is loading!");
        Self {
            phpcs_lsp: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            PhpcsLspServer::LANGUAGE_SERVER_ID => {
                let phpcs_lsp = self.phpcs_lsp.get_or_insert_with(PhpcsLspServer::new);
                phpcs_lsp.language_server_command(language_server_id, worktree)
            }
            language_server_id => Err(format!("unknown language server: {language_server_id}").into()),
        }
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
