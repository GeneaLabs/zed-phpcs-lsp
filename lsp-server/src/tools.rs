use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum PhpTool {
    Phpcs,
    Phpcbf,
}

impl PhpTool {
    pub fn name(&self) -> &'static str {
        match self {
            PhpTool::Phpcs => "phpcs",
            PhpTool::Phpcbf => "phpcbf",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PhpTool::Phpcs => "PHPCS",
            PhpTool::Phpcbf => "PHPCBF",
        }
    }

    pub fn vendor_bin(&self) -> &'static str {
        match self {
            PhpTool::Phpcs => "vendor/bin/phpcs",
            PhpTool::Phpcbf => "vendor/bin/phpcbf",
        }
    }

    pub fn phar_name(&self) -> &'static str {
        match self {
            PhpTool::Phpcs => "phpcs.phar",
            PhpTool::Phpcbf => "phpcbf.phar",
        }
    }

    pub fn env_var_name(&self) -> &'static str {
        match self {
            PhpTool::Phpcs => "PHPCS_PATH",
            PhpTool::Phpcbf => "PHPCBF_PATH",
        }
    }
}

/// Check if a command exists in the system PATH
pub fn command_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        std::process::Command::new("where")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Detect the path to a PHP tool using the following priority:
/// 1. Project vendor/bin/{tool} (project-local Composer install)
/// 2. User-configured path from LSP settings
/// 3. Environment variable (PHPCS_PATH / PHPCBF_PATH)
/// 4. System {tool} (in PATH)
/// 5. Bundled {tool}.phar
/// 6. Fallback to tool name (will fail at runtime if not found)
pub fn detect_tool_path(tool: PhpTool, workspace_root: Option<&Path>, user_path: Option<&str>) -> String {
    let display = tool.display_name();
    let name = tool.name();

    // Priority 1: Project vendor/bin
    if let Some(workspace_root) = workspace_root {
        let vendor_path = workspace_root.join(tool.vendor_bin());
        eprintln!(
            "🔍 PHPCS LSP: Checking for project {} at: {}",
            display,
            vendor_path.display()
        );

        if vendor_path.exists() {
            eprintln!("✅ PHPCS LSP: Found project-local {}", display);
            return vendor_path.to_string_lossy().to_string();
        }
        eprintln!("❌ PHPCS LSP: No project-local {} found", display);
    }

    // Priority 2: User-configured path
    if let Some(path) = user_path {
        if !path.trim().is_empty() {
            eprintln!("🎯 PHPCS LSP: Using user-configured {} path: {}", display, path);
            return path.to_string();
        }
    }

    // Priority 3: Environment variable
    let env_var = tool.env_var_name();
    eprintln!("🔍 PHPCS LSP: Checking {} env var for {}...", env_var, display);
    if let Ok(path) = std::env::var(env_var) {
        if !path.trim().is_empty() {
            eprintln!("✅ PHPCS LSP: Found {} via {} env var", display, env_var);
            return path;
        }
    }
    eprintln!("❌ PHPCS LSP: No {} env var set", env_var);

    // Priority 4: System command
    eprintln!("🔍 PHPCS LSP: Checking for system {}...", name);
    if command_exists(name) {
        eprintln!("✅ PHPCS LSP: Found system {}", name);
        return name.to_string();
    }
    eprintln!("❌ PHPCS LSP: No system {} found", name);

    // Priority 5: Bundled PHAR
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let bundled = exe_dir.join(tool.phar_name());
            eprintln!(
                "🔍 PHPCS LSP: Checking for bundled {} at: {}",
                display,
                bundled.display()
            );

            if bundled.exists() {
                eprintln!("✅ PHPCS LSP: Found bundled {} PHAR", display);
                return bundled.to_string_lossy().to_string();
            }
            eprintln!("❌ PHPCS LSP: No bundled {} found", display);
        }
    }

    // Fallback
    eprintln!(
        "⚠️ PHPCS LSP: No {} found, using '{}' as fallback",
        display, name
    );
    name.to_string()
}
