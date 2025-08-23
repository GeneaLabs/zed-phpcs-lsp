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
        eprintln!("🚀 PhpcsLspServer: language_server_command called");
        let binary_path = self.language_server_binary_path(worktree)?;
        eprintln!("🚀 PhpcsLspServer: Using binary path: {}", binary_path);
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
        eprintln!("🚀 PHPCS Extension: new() called - Extension instance created");
        eprintln!("🚀 PHPCS Extension: Starting initialization process");
        eprintln!("🚀 PHPCS Extension: Environment - Working directory: {:?}", std::env::current_dir());
        Self {
            phpcs_lsp: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        eprintln!("🚀 PHPCS Extension: language_server_command called for: {}", language_server_id.as_ref());
        eprintln!("🚀 PHPCS Extension: Expected ID: {}", PhpcsLspServer::LANGUAGE_SERVER_ID);
        match language_server_id.as_ref() {
            PhpcsLspServer::LANGUAGE_SERVER_ID => {
                eprintln!("🚀 PHPCS Extension: ID matches, getting PhpcsLspServer instance");
                let phpcs_lsp = self.phpcs_lsp.get_or_insert_with(PhpcsLspServer::new);
                phpcs_lsp.language_server_command(language_server_id, worktree)
            }
            language_server_id => {
                eprintln!("🚀 PHPCS Extension: Unknown language server ID: {}", language_server_id);
                Err(format!("unknown language server: {language_server_id}").into())
            }
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        eprintln!("🔧 PHPCS Extension: language_server_initialization_options called!");
        eprintln!("🔧 PHPCS Extension: Server ID received: '{}'", language_server_id.as_ref());
        eprintln!("🔧 PHPCS Extension: Expected ID: '{}'", PhpcsLspServer::LANGUAGE_SERVER_ID);
        
        // Check if this is our language server
        if language_server_id.as_ref() != PhpcsLspServer::LANGUAGE_SERVER_ID {
            eprintln!("🔧 PHPCS Extension: Not our server ('{}' != '{}'), returning None", language_server_id.as_ref(), PhpcsLspServer::LANGUAGE_SERVER_ID);
            return Ok(None);
        }
        
        eprintln!("🔧 PHPCS Extension: Processing initialization options for PHPCS server");
        eprintln!("🔧 PHPCS Extension: worktree path: {:?}", worktree.root_path());
        let mut options = zed::serde_json::Map::new();
        
        // Try to get user-configured settings first
        let user_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone());
        
        // Download PHPCS PHAR to LSP server directory - LSP server will find it automatically
        eprintln!("PHPCS LSP: Ensuring PHPCS PHAR is available in LSP server directory...");
        match Self::download_phar_if_needed("phpcs.phar") {
            Ok(_phar_path) => {
                eprintln!("PHPCS LSP: PHPCS PHAR available for LSP server");
            }
            Err(e) => {
                eprintln!("PHPCS LSP: Failed to download PHPCS PHAR: {}", e);
            }
        }
        
        // Download PHPCBF PHAR to LSP server directory - LSP server will find it automatically  
        eprintln!("PHPCS LSP: Ensuring PHPCBF PHAR is available in LSP server directory...");
        match Self::download_phar_if_needed("phpcbf.phar") {
            Ok(_phar_path) => {
                eprintln!("PHPCS LSP: PHPCBF PHAR available for LSP server");
            }
            Err(e) => {
                eprintln!("PHPCS LSP: Failed to download PHPCBF PHAR: {}", e);
            }
        }
        
        // Determine standard/config to use (priority order: config file -> settings -> env -> default)
        let mut standard_to_use: Option<String> = None;
        
        eprintln!("🔧 PHPCS Extension: ========================================");
        eprintln!("🔧 PHPCS Extension: Starting standard/config determination process");
        eprintln!("🔧 PHPCS Extension: Current working directory: {:?}", std::env::current_dir());
        eprintln!("🔧 PHPCS Extension: Worktree root: {:?}", worktree.root_path());
        
        // Try to find phpcs configuration file first (highest priority)
        eprintln!("🔧 PHPCS Extension: Attempting to find phpcs config file...");
        if let Some(config_file) = Self::find_phpcs_config(worktree) {
            eprintln!("✅ PHPCS Extension: Found phpcs config file: {}", config_file);
            standard_to_use = Some(config_file);
        } else {
            eprintln!("❌ PHPCS Extension: No phpcs config file found, trying other methods");
        }
        
        // Check for user-configured coding standard from settings.json
        if standard_to_use.is_none() {
            if let Some(settings) = user_settings.as_ref() {
                // Support both string and array formats for standards
                if let Some(standard_value) = settings.get("standard") {
                    match standard_value {
                        // Single standard as string
                        zed::serde_json::Value::String(standard) => {
                            if !standard.trim().is_empty() {
                                eprintln!("PHPCS LSP: Using standard from settings: {}", standard);
                                standard_to_use = Some(standard.clone());
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
                                standard_to_use = Some(combined_standards);
                            }
                        },
                        _ => {
                            eprintln!("PHPCS LSP: Invalid standard format in settings (expected string or array)");
                        }
                    }
                }
            }
        }
        
        // Fall back to environment variable for coding standard
        if standard_to_use.is_none() {
            if let Ok(env_standard) = env::var("PHPCS_STANDARD") {
                if !env_standard.trim().is_empty() {
                    eprintln!("PHPCS LSP: Using standard from PHPCS_STANDARD env: {}", env_standard);
                    standard_to_use = Some(env_standard);
                }
            }
        }
        
        // Pass the standard to the LSP server if we have one
        if let Some(standard) = standard_to_use {
            eprintln!("✅ PHPCS Extension: Final decision - passing standard to LSP server: {}", standard);
            eprintln!("✅ PHPCS Extension: Standard type: {}", if standard.ends_with(".xml") { "CONFIG FILE" } else { "STANDARD NAME" });
            eprintln!("✅ PHPCS Extension: Adding 'standard' key to options map");
            options.insert("standard".to_string(), zed::serde_json::Value::String(standard.clone()));
            eprintln!("✅ PHPCS Extension: Options map after insert: {:?}", options);
        } else {
            eprintln!("❌ PHPCS Extension: Final decision - no custom standard specified, LSP server will use default PSR1,PSR2,PSR12");
        }
        
        eprintln!("🚀 PHPCS Extension: ========================================");
        eprintln!("🚀 PHPCS Extension: Final initialization options being sent to LSP server:");
        eprintln!("🚀 PHPCS Extension: Options count: {}", options.len());
        eprintln!("🚀 PHPCS Extension: Options content: {:?}", options);
        
        if options.is_empty() {
            eprintln!("🚀 PHPCS Extension: Returning None (no options to pass)");
            Ok(None)
        } else {
            let json_value = zed::serde_json::Value::Object(options);
            eprintln!("🚀 PHPCS Extension: Returning Some with JSON: {}", zed::serde_json::to_string_pretty(&json_value).unwrap_or("Failed to serialize".to_string()));
            Ok(Some(json_value))
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
        eprintln!("🔍 PHPCS Extension: ========================================");
        eprintln!("🔍 PHPCS Extension: Starting config file discovery process");
        eprintln!("🔍 PHPCS Extension: Called from: {:?}", std::env::current_dir());
        
        let config_files = [
            ".phpcs.xml",
            "phpcs.xml", 
            ".phpcs.xml.dist",
            "phpcs.xml.dist",
        ];
        
        eprintln!("🔍 PHPCS Extension: Looking for config files: {:?}", config_files);
        
        let root_path = worktree.root_path();
        eprintln!("🔍 PHPCS Extension: Raw worktree root path: '{}'", root_path);
        let root_path = std::path::PathBuf::from(root_path);
        eprintln!("🔍 PHPCS Extension: PathBuf worktree root path: {:?}", root_path);
        eprintln!("🔍 PHPCS Extension: Path exists: {}", root_path.exists());
        eprintln!("🔍 PHPCS Extension: Path is dir: {}", root_path.is_dir());
        
        for config_file in &config_files {
            let config_path = root_path.join(config_file);
            eprintln!("🔍 PHPCS Extension: Checking for config file: {}", config_file);
            eprintln!("🔍 PHPCS Extension: Full path to check: {:?}", config_path);
            eprintln!("🔍 PHPCS Extension: Canonical path attempt: {:?}", config_path.canonicalize());
            
            if config_path.exists() {
                eprintln!("✅ PHPCS Extension: Config file EXISTS at: {:?}", config_path);
                eprintln!("✅ PHPCS Extension: File metadata: {:?}", config_path.metadata());
                if let Some(path_str) = config_path.to_str() {
                    eprintln!("✅ PHPCS Extension: Successfully converted to string: {}", path_str);
                    eprintln!("✅ PHPCS Extension: Returning config file path: {}", path_str);
                    eprintln!("🔍 PHPCS Extension: ========================================");
                    return Some(path_str.to_string());
                } else {
                    eprintln!("❌ PHPCS Extension: Could not convert path to string: {:?}", config_path);
                }
            } else {
                eprintln!("❌ PHPCS Extension: Config file does NOT exist at: {:?}", config_path);
                // Try to list directory contents to debug
                if let Ok(entries) = std::fs::read_dir(&root_path) {
                    eprintln!("🔍 PHPCS Extension: Directory contents of {:?}:", root_path);
                    for entry in entries.flatten() {
                        eprintln!("  - {:?}", entry.file_name());
                    }
                }
            }
        }
        
        eprintln!("❌ PHPCS Extension: No config files found in worktree root");
        eprintln!("❌ PHPCS Extension: Config file discovery complete - no config found, will use defaults");
        eprintln!("🔍 PHPCS Extension: ========================================");
        None
    }
}

zed::register_extension!(PhpcsLspExtension);

#[cfg(test)]
mod test;
