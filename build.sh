#!/bin/bash
set -e

echo "Building PHPCS LSP Extension..."

# Build LSP server
echo "Building LSP server..."
cd lsp-server
cargo build --release
cd ..

# Copy LSP server to root for bundling
echo "Copying LSP server binary..."
cp lsp-server/target/release/phpcs-lsp-server ./phpcs-lsp-server

# Keep bin/ directory structure for relative path access
echo "Bin directory already exists with PHPCS binaries"

# Build Zed extension  
echo "Building Zed extension..."
cargo build --release --target wasm32-wasip1

echo "Build complete!"
echo "- LSP server: phpcs-lsp-server"
echo "- Extension: target/wasm32-wasip1/release/phpcs_lsp.wasm"