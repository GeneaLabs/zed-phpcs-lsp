#!/bin/bash
set -e

echo "🚀 Building PHPCS LSP Extension..."
echo

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored status
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Clean previous builds
print_status "Cleaning previous builds..."
cargo clean
rm -f phpcs-lsp-server extension.wasm

# Build LSP server
print_status "Building LSP server..."
cd lsp-server
cargo build --release
cd ..

# Copy LSP server binary to extension root
print_status "Copying LSP server binary..."
if [ -f "lsp-server/target/release/phpcs-lsp-server" ]; then
    cp lsp-server/target/release/phpcs-lsp-server ./phpcs-lsp-server
    chmod +x ./phpcs-lsp-server
    print_success "LSP server binary copied"
else
    echo "Error: LSP server binary not found!"
    exit 1
fi

# Verify PHPCS binaries exist
print_status "Verifying PHPCS binaries..."
if [ -d "bin" ] && [ -f "bin/phpcs" ] && [ -f "bin/phpcbf" ]; then
    print_success "PHPCS binaries found in bin/ directory"
else
    print_warning "PHPCS binaries not found in bin/ directory"
    echo "Make sure you have phpcs and phpcbf in the bin/ directory"
fi

# Build Zed extension WASM
print_status "Building Zed extension (WASM)..."
cargo build --release --target wasm32-wasip1

# Copy WASM to extension root
print_status "Copying extension WASM..."
if [ -f "target/wasm32-wasip1/release/phpcs_lsp.wasm" ]; then
    cp target/wasm32-wasip1/release/phpcs_lsp.wasm extension.wasm
    print_success "Extension WASM copied"
else
    echo "Error: Extension WASM not found!"
    exit 1
fi

# Optional: Copy to Zed work directory for development
ZED_WORK_DIR="$HOME/Library/Application Support/Zed/extensions/work/phpcs-lsp"
if [ -d "$ZED_WORK_DIR" ]; then
    print_status "Copying to Zed work directory for development..."
    cp ./phpcs-lsp-server "$ZED_WORK_DIR/"
    chmod +x "$ZED_WORK_DIR/phpcs-lsp-server"
    
    if [ -d "bin" ]; then
        cp -r bin "$ZED_WORK_DIR/"
    fi
    
    print_success "Development files copied to Zed work directory"
fi

echo
print_success "Build complete!"
echo
echo "📦 Generated files:"
echo "  - LSP server: phpcs-lsp-server"
echo "  - Extension WASM: extension.wasm"
echo "  - PHPCS binaries: bin/"
echo
echo "📋 Next steps:"
echo "  1. Install/reload the extension in Zed"
echo "  2. Open a PHP file to test the language server"
echo "  3. Check diagnostics with test.php"
echo