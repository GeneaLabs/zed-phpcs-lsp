use zed_extension_api::{self as zed, settings::LspSettings, Result};
use std::env;
use std::fs;

struct PhpcsLspExtension {
    phpcs_lsp: Option<PhpcsLspServer>,
}

struct PhpcsLspServer {
    cached_binary_path: Option<String>,
}

impl PhpcsLspServer {
    const LANGUAGE_SERVER_ID: &'static str = "phpcs";

    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_path = self.language_server_binary_path(worktree)?;
        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }
    
    fn language_server_binary_path(&mut self, worktree: &zed::Worktree) -> Result<String> {
        // Check if we have a cached binary path
        if let Some(cached_path) = &self.cached_binary_path {
            if fs::metadata(cached_path).is_ok() {
                return Ok(cached_path.clone());
            }
        }

        // Try to find the binary locally first (for development)
        let binary_name = Self::get_platform_binary_name();
        if let Some(path) = worktree.which(&binary_name) {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // Download the binary from GitHub
        eprintln!("PHPCS LSP: Binary not found locally, downloading from GitHub...");
        let downloaded_path = self.download_binary(&binary_name)?;
        self.cached_binary_path = Some(downloaded_path.clone());
        Ok(downloaded_path)
    }
    
    fn download_binary(&self, binary_name: &str) -> Result<String> {
        // Use the same pattern as Gleam extension
        let version = env!("CARGO_PKG_VERSION");
        let version_dir = format!("phpcs-{}", version);
        let binary_path = format!("{}/{}", version_dir, binary_name);
        
        // Check if binary already exists
        if fs::metadata(&binary_path).is_ok() {
            eprintln!("PHPCS LSP: Binary already exists at {}", binary_path);
            return Ok(binary_path);
        }
        
        // Try to download from release assets first
        let version = env!("CARGO_PKG_VERSION");
        let (os, _arch) = zed::current_platform();
        let archive_ext = match os {
            zed::Os::Windows => "zip",
            _ => "tar.gz",
        };
        let archive_name = format!("{}.{}", binary_name, archive_ext);
        
        let release_url = format!(
            "https://github.com/GeneaLabs/zed-phpcs-lsp/releases/download/{}/{}",
            version,
            archive_name
        );
        
        eprintln!("PHPCS LSP: Attempting to download from release: {}", release_url);
        
        // Try downloading from release
        let file_type = match os {
            zed::Os::Windows => zed::DownloadedFileType::Zip,
            _ => zed::DownloadedFileType::GzipTar,
        };
        
        // Download the archive from release to version directory
        zed::download_file(&release_url, &version_dir, file_type)
            .map_err(|e| format!("Failed to download binary from release: {}. Please ensure the release {} exists with assets.", e, version))?;
        
        // After extraction, the file should be in the bin directory
        if !fs::metadata(&binary_path).is_ok() {
            return Err(format!("Binary not found after extraction. Expected at: {}", binary_path));
        }
        
        eprintln!("PHPCS LSP: Successfully downloaded and extracted binary");
        
        // Make the binary executable on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&binary_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&binary_path, perms)
                    .map_err(|e| format!("Failed to set binary permissions: {}", e))?;
            }
        }
        
        eprintln!("PHPCS LSP: Binary downloaded successfully to {}", binary_path);
        Ok(binary_path)
    }

    fn get_platform_binary_name() -> String {
        let (os, arch) = zed::current_platform();
        match (os, arch) {
            (zed::Os::Windows, zed::Architecture::X8664) => "phpcs-lsp-server-windows-x64.exe".to_string(),
            (zed::Os::Windows, zed::Architecture::Aarch64) => "phpcs-lsp-server-windows-arm64.exe".to_string(),
            (zed::Os::Windows, _) => "phpcs-lsp-server.exe".to_string(),
            (zed::Os::Mac, zed::Architecture::Aarch64) => "phpcs-lsp-server-macos-arm64".to_string(),
            (zed::Os::Mac, zed::Architecture::X8664) => "phpcs-lsp-server-macos-x64".to_string(),
            (zed::Os::Mac, _) => "phpcs-lsp-server".to_string(),
            (zed::Os::Linux, zed::Architecture::X8664) => "phpcs-lsp-server-linux-x64".to_string(),
            (zed::Os::Linux, zed::Architecture::Aarch64) => "phpcs-lsp-server-linux-arm64".to_string(),
            (zed::Os::Linux, _) => "phpcs-lsp-server".to_string(),
        }
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
        
        // Try to get user-configured settings first
        let user_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone());
        
        // Check for user-configured PHPCS path from settings.json
        let mut found_phpcs_path = false;
        if let Some(settings) = user_settings.as_ref() {
            if let Some(phpcs_path) = settings.get("phpcsPath").and_then(|v| v.as_str()) {
                if !phpcs_path.trim().is_empty() {
                    eprintln!("PHPCS LSP: Using custom PHPCS path from settings: {}", phpcs_path);
                    options.insert("phpcsPath".to_string(), zed::serde_json::Value::String(phpcs_path.to_string()));
                    found_phpcs_path = true;
                }
            }
        }
        
        // Fall back to environment variable if no settings configured
        if !found_phpcs_path {
            if let Ok(custom_phpcs_path) = env::var("PHPCS_PATH") {
                if !custom_phpcs_path.trim().is_empty() {
                    eprintln!("PHPCS LSP: Using custom PHPCS path from PHPCS_PATH env: {}", custom_phpcs_path);
                    options.insert("phpcsPath".to_string(), zed::serde_json::Value::String(custom_phpcs_path));
                    found_phpcs_path = true;
                }
            }
        }
        
        // Fall back to auto-discovery if no custom path specified
        if !found_phpcs_path {
            if let Some(phpcs_path) = Self::find_phpcs_binary(worktree) {
                eprintln!("PHPCS LSP: Found PHPCS: {}", phpcs_path);
                options.insert("phpcsPath".to_string(), zed::serde_json::Value::String(phpcs_path));
            } else {
                // Try to download PHPCS PHAR
                eprintln!("PHPCS LSP: No local PHPCS found, attempting to download...");
                match Self::download_phar_if_needed("phpcs.phar") {
                    Ok(_phar_path) => {
                        eprintln!("PHPCS LSP: Downloaded PHPCS successfully");
                        // Since the LSP server runs from the version directory, just provide the filename
                        options.insert("phpcsPath".to_string(), zed::serde_json::Value::String("phpcs.phar".to_string()));
                    }
                    Err(e) => {
                        eprintln!("PHPCS LSP: Failed to download PHPCS: {}", e);
                    }
                }
            }
        }
        
        // Check for user-configured PHPCBF path from settings.json
        let mut found_phpcbf_path = false;
        if let Some(settings) = user_settings.as_ref() {
            if let Some(phpcbf_path) = settings.get("phpcbfPath").and_then(|v| v.as_str()) {
                if !phpcbf_path.trim().is_empty() {
                    eprintln!("PHPCS LSP: Using custom PHPCBF path from settings: {}", phpcbf_path);
                    options.insert("phpcbfPath".to_string(), zed::serde_json::Value::String(phpcbf_path.to_string()));
                    found_phpcbf_path = true;
                }
            }
        }
        
        // Fall back to environment variable if no settings configured
        if !found_phpcbf_path {
            if let Ok(custom_phpcbf_path) = env::var("PHPCBF_PATH") {
                if !custom_phpcbf_path.trim().is_empty() {
                    eprintln!("PHPCS LSP: Using custom PHPCBF path from PHPCBF_PATH env: {}", custom_phpcbf_path);
                    options.insert("phpcbfPath".to_string(), zed::serde_json::Value::String(custom_phpcbf_path));
                    found_phpcbf_path = true;
                }
            }
        }
        
        // Fall back to auto-discovery if no custom path specified
        if !found_phpcbf_path {
            if let Some(phpcbf_path) = Self::find_phpcbf_binary(worktree) {
                eprintln!("PHPCS LSP: Found PHPCBF: {}", phpcbf_path);
                options.insert("phpcbfPath".to_string(), zed::serde_json::Value::String(phpcbf_path));
            } else {
                // Try to download PHPCBF PHAR
                eprintln!("PHPCS LSP: No local PHPCBF found, attempting to download...");
                match Self::download_phar_if_needed("phpcbf.phar") {
                    Ok(_phar_path) => {
                        eprintln!("PHPCS LSP: Downloaded PHPCBF successfully");
                        // Since the LSP server runs from the version directory, just provide the filename
                        options.insert("phpcbfPath".to_string(), zed::serde_json::Value::String("phpcbf.phar".to_string()));
                    }
                    Err(e) => {
                        eprintln!("PHPCS LSP: Failed to download PHPCBF: {}", e);
                    }
                }
            }
        }
        
        // Try to find phpcs configuration file
        if let Some(config_file) = Self::find_phpcs_config(worktree) {
            eprintln!("PHPCS LSP: Found phpcs config: {}", config_file);
            options.insert("configFile".to_string(), zed::serde_json::Value::String(config_file));
        } else {
            eprintln!("PHPCS LSP: No phpcs config found");
        }

        // Check for user-configured coding standard from settings.json
        let mut found_standard = false;
        if let Some(settings) = user_settings.as_ref() {
            // Support both string and array formats for standards
            if let Some(standard_value) = settings.get("standard") {
                match standard_value {
                    // Single standard as string
                    zed::serde_json::Value::String(standard) => {
                        if !standard.trim().is_empty() {
                            eprintln!("PHPCS LSP: Using standard from settings: {}", standard);
                            options.insert("standard".to_string(), zed::serde_json::Value::String(standard.clone()));
                            found_standard = true;
                        }
                    },
                    // Multiple standards as array
                    zed::serde_json::Value::Array(standards) => {
                        let standard_strings: Vec<String> = standards
                            .iter()
                            .filter_map(|v| v.as_str())
                            .filter(|s| !s.trim().is_empty())
                            .map(|s| s.to_string())
                            .collect();
                        
                        if !standard_strings.is_empty() {
                            let combined_standards = standard_strings.join(",");
                            eprintln!("PHPCS LSP: Using multiple standards from settings: {}", combined_standards);
                            options.insert("standard".to_string(), zed::serde_json::Value::String(combined_standards));
                            found_standard = true;
                        }
                    },
                    _ => {
                        eprintln!("PHPCS LSP: Invalid standard format in settings (expected string or array)");
                    }
                }
            }
        }
        
        // Fall back to environment variable for coding standard
        if !found_standard {
            if let Ok(env_standard) = env::var("PHPCS_STANDARD") {
                if !env_standard.trim().is_empty() {
                    eprintln!("PHPCS LSP: Using standard from PHPCS_STANDARD env: {}", env_standard);
                    options.insert("standard".to_string(), zed::serde_json::Value::String(env_standard));
                    found_standard = true;
                }
            }
        }
        
        // Auto-discover standard if none specified and no config file found
        if !found_standard && !options.contains_key("configFile") {
            // Default to PSR12 if no configuration is found
            eprintln!("PHPCS LSP: No standard specified and no config file found, defaulting to PSR12");
            options.insert("standard".to_string(), zed::serde_json::Value::String("PSR12".to_string()));
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
    
    fn download_phar_if_needed(phar_name: &str) -> Result<String> {
        // Use the same pattern as Gleam extension for consistency
        let version = env!("CARGO_PKG_VERSION");
        let version_dir = format!("phpcs-{}", version);
        let phar_path = format!("{}/{}", version_dir, phar_name);
        
        // Check if PHAR already exists
        if fs::metadata(&phar_path).is_ok() {
            eprintln!("PHPCS LSP: {} already exists at {}", phar_name, phar_path);
            return Ok(phar_path);
        }
        
        // Try to download from release assets first
        let version = env!("CARGO_PKG_VERSION");
        let archive_name = format!("{}.tar.gz", phar_name);
        
        let release_url = format!(
            "https://github.com/GeneaLabs/zed-phpcs-lsp/releases/download/{}/{}",
            version,
            archive_name
        );
        
        eprintln!("PHPCS LSP: Attempting to download {} from release: {}", phar_name, release_url);
        
        // Download the archive from release to version directory
        zed::download_file(&release_url, &version_dir, zed::DownloadedFileType::GzipTar)
            .map_err(|e| format!("Failed to download {} from release: {}. Please ensure the release {} exists with assets.", phar_name, e, version))?;
        
        // After extraction, the file should be in the bin directory
        if !fs::metadata(&phar_path).is_ok() {
            return Err(format!("{} not found after extraction. Expected at: {}", phar_name, phar_path));
        }
        
        eprintln!("PHPCS LSP: Successfully downloaded and extracted {}", phar_name);
        
        // Make the PHAR executable on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&phar_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&phar_path, perms)
                    .map_err(|e| format!("Failed to set {} permissions: {}", phar_name, e))?;
            }
        }
        
        eprintln!("PHPCS LSP: {} downloaded successfully to {}", phar_name, phar_path);
        Ok(phar_path)
    }

    fn find_phpcs_binary(worktree: &zed::Worktree) -> Option<String> {
        eprintln!("PHPCS LSP: Searching for PHPCS binary...");
        
        // First try project-specific PHPCS
        if let Some(path) = worktree.which("vendor/bin/phpcs") {
            eprintln!("PHPCS LSP: Found project PHPCS at: {}", path);
            return Some(path);
        }
        
        // Try bundled PHPCS PHAR using worktree.which()
        if let Some(path) = worktree.which("phpcs.phar") {
            eprintln!("PHPCS LSP: Found bundled PHPCS PHAR at: {}", path);
            return Some(path);
        }
        
        // Try to download PHPCS PHAR if not found
        eprintln!("PHPCS LSP: No local PHPCS found, attempting to download...");
        match Self::download_phar_if_needed("phpcs.phar") {
            Ok(path) => {
                eprintln!("PHPCS LSP: Using downloaded PHPCS at: {}", path);
                Some(path)
            }
            Err(e) => {
                eprintln!("PHPCS LSP: Failed to download PHPCS: {}", e);
                None
            }
        }
    }

    fn find_phpcbf_binary(worktree: &zed::Worktree) -> Option<String> {
        eprintln!("PHPCS LSP: Searching for PHPCBF binary...");
        
        // First try project-specific PHPCBF
        if let Some(path) = worktree.which("vendor/bin/phpcbf") {
            eprintln!("PHPCS LSP: Found project PHPCBF at: {}", path);
            return Some(path);
        }
        
        // Try bundled PHPCBF PHAR using worktree.which()
        if let Some(path) = worktree.which("phpcbf.phar") {
            eprintln!("PHPCS LSP: Found bundled PHPCBF PHAR at: {}", path);
            return Some(path);
        }
        
        // Try to download PHPCBF PHAR if not found
        eprintln!("PHPCS LSP: No local PHPCBF found, attempting to download...");
        match Self::download_phar_if_needed("phpcbf.phar") {
            Ok(path) => {
                eprintln!("PHPCS LSP: Using downloaded PHPCBF at: {}", path);
                Some(path)
            }
            Err(e) => {
                eprintln!("PHPCS LSP: Failed to download PHPCBF: {}", e);
                None
            }
        }
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
