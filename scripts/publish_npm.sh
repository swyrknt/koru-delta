#!/bin/bash
# KoruDelta - Publish npm package
# Builds WASM and publishes to npm

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   KoruDelta - Publish to npm                                 ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Navigate to project root
cd "$(dirname "$0")/.."

# Check if npm is logged in
echo "Checking npm authentication..."
if ! npm whoami &>/dev/null; then
    echo -e "${RED}❌ Not logged into npm${NC}"
    echo "Run: npm login"
    exit 1
fi
echo -e "${GREEN}✓ Logged into npm as: $(npm whoami)${NC}"
echo ""

# Get version
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo -e "Version: ${YELLOW}${VERSION}${NC}"
echo ""

# Build WASM
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Building WASM package                                      ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

cd bindings/javascript

# Build web target
echo "Building for web..."
wasm-pack build ../../ --target web -- --no-default-features --features wasm
cp -r ../../pkg pkg-web

# Build nodejs target
echo "Building for nodejs..."
wasm-pack build ../../ --target nodejs -- --no-default-features --features wasm
cp -r ../../pkg pkg-nodejs

# Update package.json with proper exports
echo "Configuring package.json..."
cat > pkg/package.json << EOF
{
  "name": "koru-delta",
  "version": "${VERSION}",
  "description": "The causal database for JavaScript - content-addressed storage with time-travel queries",
  "main": "index.js",
  "types": "index.d.ts",
  "files": [
    "index.js",
    "index.d.ts",
    "koru_delta_bg.wasm",
    "koru_delta_bg.wasm.d.ts",
    "README.md",
    "LICENSE"
  ],
  "scripts": {},
  "repository": {
    "type": "git",
    "url": "https://github.com/swyrknt/koru-delta.git"
  },
  "keywords": [
    "database",
    "causal",
    "time-travel",
    "content-addressed",
    "wasm",
    "browser",
    "edge"
  ],
  "author": "Sawyer Kent <sawyerkent.me@gmail.com>",
  "license": "MIT OR Apache-2.0",
  "bugs": {
    "url": "https://github.com/swyrknt/koru-delta/issues"
  },
  "homepage": "https://github.com/swyrknt/koru-delta#readme"
}
EOF

echo -e "${GREEN}✓ WASM build complete${NC}"
echo ""

# Publish
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Ready to Publish                                           ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Package: ${YELLOW}koru-delta${NC}"
echo -e "Version: ${YELLOW}${VERSION}${NC}"
echo ""
read -p "Publish to npm? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Publish cancelled."
    exit 0
fi

echo ""
echo "Publishing..."
cd pkg
if npm publish --access public; then
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   ✅ Successfully published to npm!                          ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "Package: ${GREEN}koru-delta@${VERSION}${NC}"
    echo -e "View at: ${BLUE}https://www.npmjs.com/package/koru-delta${NC}"
    echo ""
    echo "Install with:"
    echo "  npm install koru-delta"
    echo ""
else
    echo -e "${RED}❌ Publish failed${NC}"
    exit 1
fi
