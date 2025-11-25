#!/bin/bash
# WASM build script for KoruDelta
# Builds a WebAssembly package for browser/edge deployment

set -e

echo "🚀 KoruDelta WASM Build Script"
echo "==============================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo -e "${RED}Error: wasm-pack is not installed${NC}"
    echo ""
    echo "Install wasm-pack with:"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    echo ""
    exit 1
fi

echo -e "${BLUE}Building WASM package...${NC}"
echo ""

# Get version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
echo "📦 Version: $VERSION"
echo ""

# Build for web (ES modules)
echo -e "${BLUE}Building for web target (ES modules)...${NC}"
wasm-pack build --target web --out-dir pkg/web --features wasm

echo ""
echo -e "${GREEN}✓ Web build complete${NC}"
echo ""

# Build for bundler (webpack, etc.)
echo -e "${BLUE}Building for bundler target (webpack, etc.)...${NC}"
wasm-pack build --target bundler --out-dir pkg/bundler --features wasm

echo ""
echo -e "${GREEN}✓ Bundler build complete${NC}"
echo ""

# Build for Node.js
echo -e "${BLUE}Building for Node.js target...${NC}"
wasm-pack build --target nodejs --out-dir pkg/nodejs --features wasm

echo ""
echo -e "${GREEN}✓ Node.js build complete${NC}"
echo ""

# Show build artifacts
echo "=== Build Complete ==="
echo ""
echo "Built WASM packages:"
echo ""
echo "  📦 Web (ES modules):  ./pkg/web/"
echo "  📦 Bundler (webpack): ./pkg/bundler/"
echo "  📦 Node.js:           ./pkg/nodejs/"
echo ""

# Show package sizes
echo "Package sizes:"
for target in web bundler nodejs; do
    if [ -f "pkg/${target}/koru_delta_bg.wasm" ]; then
        SIZE=$(ls -lh "pkg/${target}/koru_delta_bg.wasm" | awk '{print $5}')
        echo "  ${target}: ${SIZE}"
    fi
done
echo ""

echo -e "${GREEN}✅ WASM build successful!${NC}"
echo ""
echo "Usage examples:"
echo ""
echo "  # Web (ES modules)"
echo "  import init, { KoruDeltaWasm } from './pkg/web/koru_delta.js';"
echo ""
echo "  # Bundler (webpack)"
echo "  import { KoruDeltaWasm } from 'koru-delta';"
echo ""
echo "  # Node.js"
echo "  const { KoruDeltaWasm } = require('./pkg/nodejs/koru_delta');"
echo ""
