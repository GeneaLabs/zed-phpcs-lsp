# 🎯 PHPCS LSP for Zed Editor

> **Real-time PHP code quality checking** directly in your Zed editor with PHP_CodeSniffer integration

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![PHP](https://img.shields.io/badge/PHP-8.0%2B-777BB4?logo=php&logoColor=white)](https://php.net)
[![Zed](https://img.shields.io/badge/Zed-Editor-blue?logo=zed&logoColor=white)](https://zed.dev)
[![PHPCS](https://img.shields.io/badge/PHPCS-Compatible-green)](https://github.com/squizlabs/PHP_CodeSniffer)

## ✨ Features

- 🔍 **Real-time Diagnostics** - See code style violations as you type
- ⚡ **Zero Configuration** - Works out of the box with PSR-12 standards
- 🎨 **Visual Feedback** - Red underlines for errors, yellow for warnings
- 📦 **Self-contained** - Bundled PHPCS binaries, no external dependencies
- 🔧 **Highly Configurable** - Support for custom rulesets and standards
- 🚀 **Performance Optimized** - Uses stdin for fast, race-condition-free analysis

## 🚀 Quick Start

### Installation

1. **Install the Extension**
   ```bash
   # Via Zed Extensions (coming soon)
   # Or manual installation for development
   ```

2. **Open a PHP Project**
   ```bash
   zed your-php-project/
   ```

3. **Start Coding!** 
   The extension will automatically highlight code style violations:

   ```php
   <?php
   // ❌ This will show errors
   if($x==1){echo "test";}
   
   // ✅ This follows PSR-12
   if ($x == 1) {
       echo "test";
   }
   ```

## 📋 Supported Standards

- **PSR-12** (default) - The extended coding style guide
- **PSR-2** - Coding style guide (legacy)
- **PSR-1** - Basic coding standard
- **Squiz** - Comprehensive coding standard
- **PEAR** - PEAR coding standard
- **Zend** - Zend framework standard
- **Custom** - Your own phpcs.xml configuration

## ⚙️ Configuration

### Project-level Configuration

Create a `phpcs.xml` or `.phpcs.xml` in your project root:

```xml
<?xml version="1.0"?>
<ruleset name="My Project Standard">
    <description>Custom coding standard</description>
    
    <!-- Use PSR-12 as base -->
    <rule ref="PSR12"/>
    
    <!-- Add custom rules -->
    <rule ref="Generic.Files.LineLength">
        <properties>
            <property name="lineLimit" value="120"/>
        </properties>
    </rule>
    
    <!-- Exclude specific files/directories -->
    <exclude-pattern>*/vendor/*</exclude-pattern>
    <exclude-pattern>*/storage/*</exclude-pattern>
</ruleset>
```

### Environment Variables

```bash
# Override the coding standard
export PHPCS_STANDARD="PSR2"

# Use project-specific PHPCS
export PHPCS_PATH="/path/to/your/phpcs"
```

## 🔧 Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/mikebronner/zed-phpcs-lsp.git
cd zed-phpcs-lsp

# Build the extension
./build.sh

# Install for development
ln -s "$(pwd)" ~/.config/zed/extensions/phpcs-lsp
```

### Project Structure

```
zed-phpcs-lsp/
├── src/lib.rs              # Zed extension implementation
├── lsp-server/src/main.rs  # LSP server implementation  
├── extension.toml          # Extension metadata
├── bin/                    # Bundled PHPCS binaries
├── build.sh               # Build script
└── test.php               # Test file for development
```

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. **🐛 Report Bugs** - Found an issue? [Open an issue](https://github.com/mikebronner/zed-phpcs-lsp/issues)
2. **💡 Suggest Features** - Have an idea? We'd love to hear it!
3. **🔧 Submit PRs** - Fix bugs or add features
4. **📖 Improve Docs** - Help make our documentation better

### Development Setup

```bash
# Fork and clone the repo
git clone https://github.com/YOUR_USERNAME/zed-phpcs-lsp.git

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target for Zed extensions
rustup target add wasm32-wasip1

# Build and test
./build.sh
```

## 📚 How It Works

This extension bridges PHP_CodeSniffer with Zed's Language Server Protocol:

1. **Real-time Analysis** - Content is sent via stdin to PHPCS for immediate analysis
2. **LSP Integration** - PHPCS output is converted to LSP diagnostics
3. **Visual Feedback** - Diagnostics are displayed as underlines in the editor
4. **Performance** - Uses stdin to avoid file system race conditions

## 🆚 Comparison with Other Solutions

| Feature | PHPCS LSP | VS Code PHP | PhpStorm |
|---------|-----------|-------------|----------|
| Real-time diagnostics | ✅ | ✅ | ✅ |
| Zero configuration | ✅ | ❌ | ✅ |
| Custom rulesets | ✅ | ✅ | ✅ |
| Performance | ⚡ Fast | 🐌 Slow | ⚡ Fast |
| Free | ✅ | ✅ | ❌ |

## 📖 Documentation

- [Installation Guide](docs/installation.md) *(coming soon)*
- [Configuration Reference](docs/configuration.md) *(coming soon)*
- [Troubleshooting](docs/troubleshooting.md) *(coming soon)*
- [API Reference](docs/api.md) *(coming soon)*

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [PHP_CodeSniffer](https://github.com/squizlabs/PHP_CodeSniffer) - The amazing tool that powers this extension
- [Zed Editor](https://zed.dev) - The lightning-fast collaborative editor
- [Tower LSP](https://github.com/ebkalderon/tower-lsp) - Rust LSP framework
- The PHP community for maintaining excellent coding standards

---

<div align="center">

**Made with ❤️ for the PHP community**

[⭐ Star this repo](https://github.com/mikebronner/zed-phpcs-lsp) • [🐛 Report issues](https://github.com/mikebronner/zed-phpcs-lsp/issues) • [💬 Discussions](https://github.com/mikebronner/zed-phpcs-lsp/discussions)

</div>