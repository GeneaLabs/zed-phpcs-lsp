# PHPCS LSP Extension for Zed Editor

This is a Zed editor extension that provides PHP CodeSniffer (PHPCS) integration as a dedicated Language Server Protocol (LSP) implementation for PHP linting and code quality checking.

## Project Structure

```
zed-phpcs-lsp/
├── Cargo.toml              # Zed extension configuration
├── extension.toml          # Extension metadata
├── src/
│   └── lib.rs             # Zed extension implementation
├── lsp-server/
│   ├── Cargo.toml         # LSP server dependencies
│   └── src/
│       └── main.rs        # LSP server implementation
├── build.sh               # Build script
├── test.php              # Test file with PHPCS violations
├── phpcs.xml             # Sample PHPCS configuration
├── DEVELOPMENT.md        # Development guide
└── README.md             # Project documentation
```

## Build Commands

- **Build LSP server**: `cd lsp-server && cargo build --release`
- **Build Zed extension**: `cargo build --release`
- **Build everything**: `./build.sh`
- **Test PHPCS directly**: `phpcs --standard=PSR12 --report=json test.php`

## Development Workflow

1. **Prerequisites**: Ensure Rust and PHPCS are installed
2. **Build**: Run `./build.sh` to compile both components
3. **Install**: Copy binaries to appropriate locations (see DEVELOPMENT.md)
4. **Test**: Use test.php file to verify PHPCS integration

## Key Features

- Real-time PHP linting using PHPCS
- Support for multiple coding standards (PSR-12, Slevomat, custom)
- Automatic discovery of PHPCS configuration files
- Works alongside other PHP language servers
- Configurable through standard PHPCS configuration files

## Architecture

- **Zed Extension** (`src/lib.rs`): Finds PHPCS binary and manages configuration
- **LSP Server** (`lsp-server/src/main.rs`): Bridges PHPCS output to Language Server Protocol
- **Configuration**: Automatic discovery of .phpcs.xml, phpcs.xml, etc.
- **Diagnostics**: Converts PHPCS JSON output to LSP diagnostic messages

## Testing

Use the provided `test.php` file which contains various PHPCS violations to test the extension functionality. The `phpcs.xml` configuration file demonstrates how to set up project-specific coding standards.

## Dependencies

- **Zed Extension**: zed_extension_api, serde, serde_json
- **LSP Server**: tower-lsp, tokio, serde, anyhow, regex
- **External**: PHP CodeSniffer (phpcs) binary

## Installation Notes

The LSP server binary (`phpcs-lsp-server`) must be accessible in the system PATH or the extension will not function. The extension automatically looks for PHPCS in `vendor/bin/phpcs` first, then falls back to global installation.

## Important: Executable Locations
**ALL executables are stored in the `bin/` folder** - including phpcs, phpcbf, and other tools. Always check `bin/` directory for any executables.
- reference PHPCS documentation: https://github.com/squizlabs/PHP_CodeSniffer
- reference Zed documentation: https://zed.dev/docs/
- most importantly reference Zed plugin development documentation: https://zed.dev/docs/extensions/developing-extensions
- never copy things to the user bin directory
- do not just create random test files, instead create unit tests as part of the project
- after every completed fix or feature, ALWAYS ask me if it was successful/working as expected
- if I do not answer affirmatively, it implies that it was not successful and further work needs to be done
- if I answer affirmatively, then commit the changes using:
  - Conventional Commits format (conventionalcommits.org)
  - Include an appropriate gitmoji (gitmoji.dev) right before the description, followed by a space
  - Format: `<type>: <gitmoji> <description>`
  - Example: `fix: 🐛 correct diagnostic range to underline tokens`