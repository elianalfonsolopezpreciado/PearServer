#!/usr/bin/env bash
# Pear Server Runtime Asset Manager
# Downloads pre-compiled WebAssembly runtimes for polyglot support

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Configuration
RUNTIME_DIR="./assets/runtimes"
TMP_DIR="/tmp/pear-runtimes"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}🍐 Pear Server Runtime Setup${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Create directories
echo -e "${GREEN}Creating runtime directories...${NC}"
mkdir -p "$RUNTIME_DIR"
mkdir -p "$TMP_DIR"

cd "$TMP_DIR"

# Function to download and verify
download_runtime() {
    local name=$1
    local url=$2
    local output=$3
    
    echo -e "${BLUE}Downloading $name...${NC}"
    
    if command -v curl &> /dev/null; then
        curl -L -o "$output" "$url"
    elif command -v wget &> /dev/null; then
        wget -O "$output" "$url"
    else
        echo -e "${RED}Error: Neither curl nor wget found. Please install one.${NC}"
        exit 1
    fi
    
    if [ -f "$output" ]; then
        echo -e "${GREEN}✓ Downloaded $name${NC}"
        return 0
    else
        echo -e "${RED}✗ Failed to download $name${NC}"
        return 1
    fi
}

# 1. PHP CGI Runtime (php.wasm from VMWare Wasm Labs)
echo ""
echo -e "${YELLOW}[1/3] PHP Runtime${NC}"
PHP_URL="https://github.com/vmware-labs/webassembly-language-runtimes/releases/download/php%2F8.2.0%2B20230707-1755149/php-cgi-8.2.0.wasm"
download_runtime "PHP 8.2 CGI" "$PHP_URL" "php-cgi.wasm"

# 2. Python Runtime (python.wasm from VMWare Wasm Labs)
echo ""
echo -e "${YELLOW}[2/3] Python Runtime${NC}"
PYTHON_URL="https://github.com/vmware-labs/webassembly-language-rt/releases/download/python%2F3.11.3%2B20230428-173305/python-3.11.3.wasm"
download_runtime "Python 3.11" "$PYTHON_URL" "python.wasm"

# 3. Static Web Server (custom or from Wasm4 static-web-server)
echo ""
echo -e "${YELLOW}[3/3] Static Web Server${NC}"
STATIC_URL="https://github.com/static-web-server/static-web-server/releases/download/v2.24.2/static-web-server-v2.24.2-x86_64-unknown-linux-gnu-wasm32-wasi.wasm"
download_runtime "Static Web Server" "$STATIC_URL" "static-server.wasm"

# Move to runtime directory
echo ""
echo -e "${GREEN}Installing runtimes...${NC}"
cp *.wasm "$RUNTIME_DIR/" 2>/dev/null || echo -e "${YELLOW}Some runtimes may have failed to download${NC}"

# Return to original directory
cd - > /dev/null

# Verify installations
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}✓ Setup Complete${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Runtime directory: $RUNTIME_DIR"
echo ""
echo "Installed runtimes:"

if [ -f "$RUNTIME_DIR/php-cgi.wasm" ]; then
    PHP_SIZE=$(du -h "$RUNTIME_DIR/php-cgi.wasm" | cut -f1)
    echo -e "${GREEN}✓ PHP 8.2 CGI ($PHP_SIZE)${NC}"
else
    echo -e "${RED}✗ PHP runtime missing${NC}"
fi

if [ -f "$RUNTIME_DIR/python.wasm" ]; then
    PY_SIZE=$(du -h "$RUNTIME_DIR/python.wasm" | cut -f1)
    echo -e "${GREEN}✓ Python 3.11 ($PY_SIZE)${NC}"
else
    echo -e "${RED}✗ Python runtime missing${NC}"
fi

if [ -f "$RUNTIME_DIR/static-server.wasm" ]; then
    STATIC_SIZE=$(du -h "$RUNTIME_DIR/static-server.wasm" | cut -f1)
    echo -e "${GREEN}✓ Static Web Server ($STATIC_SIZE)${NC}"
else
    echo -e "${RED}✗ Static runtime missing${NC}"
fi

echo ""
echo -e "${BLUE}Note: These are pre-compiled WebAssembly binaries from trusted sources:${NC}"
echo "  - PHP & Python: VMWare Wasm Labs"
echo "  - Static Server: static-web-server project"
echo ""
echo -e "${GREEN}You can now run: cargo build --release${NC}"

# Cleanup
rm -rf "$TMP_DIR"
