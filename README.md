# PHPCS LSP for Zed Editor

> A Language Server Protocol implementation that brings PHP_CodeSniffer integration to Zed Editor

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![PHP](https://img.shields.io/badge/PHP-8.0%2B-777BB4?logo=php&logoColor=white)](https://php.net)
[![Zed](https://img.shields.io/badge/Zed-Editor-blue?logo=zed&logoColor=white)](https://zed.dev)
[![PHPCS](https://img.shields.io/badge/PHPCS-Compatible-green)](https://github.com/squizlabs/PHP_CodeSniffer)

This extension integrates PHP_CodeSniffer with Zed Editor to provide real-time code style checking. It highlights violations as you code and supports various PHP coding standards including PSR-12, custom rulesets, and project-specific configurations.

## Features

- **Real-time diagnostics** - See code style violations as you type
- **Zero configuration** - Works out of the box with PSR1,PSR2,PSR12
- **Multiple standards** - PSR-12, PSR-2, Squiz, custom rulesets
- **Project awareness** - Automatically discovers phpcs.xml configuration
- **Cross-platform** - Includes binaries for Linux, macOS, and Windows
- **Flexible configuration** - Via Zed settings, environment variables, or project files

## Quick Start

### Installation

```bash
# Via Zed Extensions (coming soon)
# For now: manual installation for development
```

### Basic Usage

1. **Enable the language server** in your Zed settings.json:

```json
{
  "languages": {
    "PHP": {
      "language_servers": ["intelephense", "phpcs"]
    }
  }
}
```

2. **Open any PHP project** and the extension will start analyzing your code:

```php
<?php
// This will show underlines for style violations
if($x==1){echo "test";}

// This follows PSR-12 and won't show any issues
if ($x === 1) {
    echo "test";
}
```

## Configuration

> **Note:** The extension works without any configuration using PSR1,PSR2,PSR12 standards and bundled PHPCS binaries.

### Coding Standards

<details>
<summary><strong>Automatic Discovery</strong> (recommended)</summary>

The extension follows **PHP_CodeSniffer's native discovery behavior** with this priority order:

1. **Project config files** (discovered automatically, same as PHPCS):
   - `.phpcs.xml` (highest priority)
   - `phpcs.xml`
   - `.phpcs.xml.dist`
   - `phpcs.xml.dist` (lowest config file priority)
2. **Zed settings** - Custom configuration in settings.json  
3. **Environment variables** - `PHPCS_STANDARD`
4. **Default fallback** - PSR1,PSR2,PSR12 standards

</details>

<details>
<summary><strong>Zed Settings Configuration</strong></summary>

Configure standards in your **Zed settings.json** file (open with `Cmd+,` or `Ctrl+,`):

**Single standard:**
```json
{
  "lsp": {
    "phpcs": {
      "settings": {
        "standard": "PSR12"
      }
    }
  }
}
```

**Multiple standards (comma-separated):**
```json
{
  "lsp": {
    "phpcs": {
      "settings": {
        "standard": ["PSR12", "Squiz.Commenting", "Generic.Files.LineLength"]
      }
    }
  }
}
```

**Path to custom ruleset:**
```json
{
  "lsp": {
    "phpcs": {
      "settings": {
        "standard": "/path/to/custom-phpcs.xml"
      }
    }
  }
}
```

**Relative path to project ruleset:**
```json
{
  "lsp": {
    "phpcs": {
      "settings": {
        "standard": "./ruleset.xml"
      }
    }
  }
}
```

> **💡 Tip:** You can also set these in **local project settings** by creating `.zed/settings.json` in your project root.

</details>

<details>
<summary><strong>Environment Variables</strong></summary>

```bash
export PHPCS_STANDARD="PSR12"
export PHPCS_PATH="/custom/path/to/phpcs"
export PHPCBF_PATH="/custom/path/to/phpcbf"
```

</details>

### PHPCS Executable

<details>
<summary><strong>Automatic Discovery</strong> (recommended)</summary>

The extension finds PHPCS in this order:

1. **Project composer** - `vendor/bin/phpcs`
2. **Bundled PHAR** - `bin/phpcs.phar` (included with extension)
3. **System PATH** - Global phpcs installation

</details>

<details>
<summary><strong>Custom Paths</strong></summary>

Specify custom PHPCS/PHPCBF paths in settings.json:

```json
{
  "lsp": {
    "phpcs": {
      "settings": {
        "phpcsPath": "/custom/path/to/phpcs",
        "phpcbfPath": "/custom/path/to/phpcbf"
      }
    }
  }
}
```

</details>

## Out-of-the-box Standards

| Standard | Description |
|----------|-------------|
| **PSR1,PSR2,PSR12** | Combined standards (default) |
| **PSR-12** | Extended coding style |
| **PSR-2** | Coding style guide |
| **PSR-1** | Basic coding standard |
| **Squiz** | Comprehensive rules |
| **PEAR** | PEAR coding standard |
| **Zend** | Zend framework standard |
| **Custom** | Your phpcs.xml ruleset |

## Project Configuration

Create a `phpcs.xml` in your project root for team consistency. The extension will automatically discover and use any of these files (in priority order):

- `.phpcs.xml` (typically for local overrides, often gitignored)
- `phpcs.xml` (main project configuration) 
- `.phpcs.xml.dist` (distributable version, lower priority)
- `phpcs.xml.dist` (template version, lowest priority)

```xml
<?xml version="1.0"?>
<ruleset name="Project Standards">
    <description>Custom coding standard for our project</description>

    <rule ref="PSR12"/>

    <!-- Customize line length -->
    <rule ref="Generic.Files.LineLength">
        <properties>
            <property name="lineLimit" value="120"/>
        </properties>
    </rule>

    <!-- Exclude directories -->
    <exclude-pattern>*/vendor/*</exclude-pattern>
    <exclude-pattern>*/storage/*</exclude-pattern>
</ruleset>
```

## Development

### Building from Source
You only need to build the LSP during development.
```bash
cd lsp-server
cargo build --release
cp target/release/phpcs-lsp-server ../bin/phpcs-lsp-server
chmod +x ../bin/phpcs-lsp-server
```

### Project Structure

```
zed-phpcs-lsp/
├── src/lib.rs              # Zed extension (Rust → WASM)
├── lsp-server/src/main.rs  # LSP server implementation
├── bin/                    # Cross-platform binaries (auto-updated via CI)
└── extension.toml          # Extension metadata
```

### Contributing

Contributions are welcome! Please feel free to:

- Report bugs or request features via [GitHub Issues](https://github.com/GeneaLabs/zed-phpcs-lsp/issues)
- Submit pull requests for improvements
- Share feedback in [Discussions](https://github.com/GeneaLabs/zed-phpcs-lsp/discussions)

## Troubleshooting

<details>
<summary><strong>Extension not working?</strong></summary>

1. Check Zed's debug console for error messages
2. Verify PHPCS is accessible (custom paths must exist)
3. Try restarting Zed after configuration changes

</details>

<details>
<summary><strong>No diagnostics showing?</strong></summary>

1. Ensure you're editing a `.php` file
2. Check that your configured standard exists
3. Test with a file containing obvious style violations

</details>

<details>
<summary><strong>Custom rules not working?</strong></summary>

1. Validate your `phpcs.xml` syntax
2. Ensure paths are relative to your project root
3. Test your configuration manually with `phpcs --config-show`

</details>

## Resources & Credits

### Learn More
- [PHP_CodeSniffer Documentation](https://github.com/squizlabs/PHP_CodeSniffer/wiki)
- [PSR Standards](https://www.php-fig.org/psr/)
- [Zed Editor Documentation](https://zed.dev/docs)

### Built With
- [PHP_CodeSniffer](https://github.com/squizlabs/PHP_CodeSniffer) - The excellent tool that powers code analysis
- [Zed Editor](https://zed.dev) - The fast, collaborative editor
- [Tower LSP](https://github.com/ebkalderon/tower-lsp) - Rust LSP framework

## License
This project is licensed under the [MIT License](LICENSE).

-----
**Made with ❤️ and lots of ☕ for the PHP community.**
