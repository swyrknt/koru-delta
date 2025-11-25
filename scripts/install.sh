#!/bin/bash
# Installation script for KoruDelta CLI
# Usage: curl -fsSL https://get.korudelta.com | sh

set -e

echo "🚀 KoruDelta Installer"
echo "======================"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

echo -e "${BLUE}Detected OS: ${OS}${NC}"
echo -e "${BLUE}Detected Architecture: ${ARCH}${NC}"
echo ""

# Map to Rust target triples
case "$OS" in
    darwin*)
        case "$ARCH" in
            x86_64)
                TARGET="x86_64-apple-darwin"
                ;;
            arm64|aarch64)
                TARGET="aarch64-apple-darwin"
                ;;
            *)
                echo -e "${RED}Unsupported architecture: ${ARCH}${NC}"
                exit 1
                ;;
        esac
        ;;
    linux*)
        case "$ARCH" in
            x86_64)
                TARGET="x86_64-unknown-linux-gnu"
                ;;
            aarch64|arm64)
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            *)
                echo -e "${RED}Unsupported architecture: ${ARCH}${NC}"
                exit 1
                ;;
        esac
        ;;
    *)
        echo -e "${RED}Unsupported OS: ${OS}${NC}"
        exit 1
        ;;
esac

echo -e "${GREEN}Installing kdelta for ${TARGET}${NC}"
echo ""

# Get latest version (from GitHub releases or default to a version)
VERSION="${KDELTA_VERSION:-0.1.0}"

# Download URL (adjust this to your actual release location)
DOWNLOAD_URL="https://github.com/swyrknt/koru-delta/releases/download/v${VERSION}/kdelta-${VERSION}-${TARGET}"

# Install directory
INSTALL_DIR="${KDELTA_INSTALL_DIR:-$HOME/.local/bin}"

# Create install directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Download binary
echo "Downloading kdelta..."
TEMP_FILE=$(mktemp)

if command -v curl &> /dev/null; then
    curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_FILE"
elif command -v wget &> /dev/null; then
    wget -q "$DOWNLOAD_URL" -O "$TEMP_FILE"
else
    echo -e "${RED}Error: Neither curl nor wget is installed${NC}"
    exit 1
fi

# Make executable
chmod +x "$TEMP_FILE"

# Move to install directory
mv "$TEMP_FILE" "$INSTALL_DIR/kdelta"

echo ""
echo -e "${GREEN}✅ kdelta installed successfully!${NC}"
echo ""
echo "Installation location: $INSTALL_DIR/kdelta"
echo ""

# Check if install directory is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH${NC}"
    echo ""
    echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    echo ""
else
    echo "You can now use kdelta:"
    echo "  kdelta --help"
    echo ""
fi

# Show quick start
echo "Quick Start:"
echo "  kdelta set users/alice '{\"name\": \"Alice\", \"age\": 30}'"
echo "  kdelta get users/alice"
echo "  kdelta log users/alice"
echo "  kdelta status"
echo ""
echo "Documentation: https://github.com/swyrknt/koru-delta"
