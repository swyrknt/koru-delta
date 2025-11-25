#!/bin/bash
# Universal build script for KoruDelta
# Builds binaries for Linux, macOS, and Windows

set -e

echo "🚀 KoruDelta Universal Build Script"
echo "===================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
echo "📦 Building version: $VERSION"
echo ""

# Create dist directory
mkdir -p dist

# Function to build for a target
build_target() {
    local target=$1
    local name=$2

    echo -e "${BLUE}Building for ${name}...${NC}"

    # Check if target is installed
    if ! rustup target list | grep -q "${target} (installed)"; then
        echo -e "${YELLOW}Installing target ${target}...${NC}"
        rustup target add ${target}
    fi

    # Build
    cargo build --release --target ${target} --bin kdelta

    # Copy binary to dist with appropriate name
    if [[ "$target" == *"windows"* ]]; then
        cp "target/${target}/release/kdelta.exe" "dist/kdelta-${VERSION}-${target}.exe"
        echo -e "${GREEN}✓ Created: dist/kdelta-${VERSION}-${target}.exe${NC}"
    else
        cp "target/${target}/release/kdelta" "dist/kdelta-${VERSION}-${target}"
        chmod +x "dist/kdelta-${VERSION}-${target}"
        echo -e "${GREEN}✓ Created: dist/kdelta-${VERSION}-${target}${NC}"
    fi
    echo ""
}

# Detect current platform
PLATFORM=$(uname -s)

echo "Detected platform: $PLATFORM"
echo ""

# Build for current platform first
case "$PLATFORM" in
    Darwin*)
        echo "=== Building for macOS ==="
        build_target "x86_64-apple-darwin" "macOS (Intel)"
        build_target "aarch64-apple-darwin" "macOS (Apple Silicon)"
        ;;
    Linux*)
        echo "=== Building for Linux ==="
        build_target "x86_64-unknown-linux-gnu" "Linux (x86_64)"
        build_target "aarch64-unknown-linux-gnu" "Linux (ARM64)"
        ;;
    *)
        echo -e "${YELLOW}Unknown platform: $PLATFORM${NC}"
        ;;
esac

# Build for other platforms if cross-compilation is set up
if command -v cross &> /dev/null; then
    echo "=== Cross-compiling for additional platforms ==="
    echo -e "${BLUE}Using 'cross' for cross-compilation${NC}"
    echo ""

    # Windows
    echo "Building for Windows..."
    cross build --release --target x86_64-pc-windows-gnu --bin kdelta
    cp "target/x86_64-pc-windows-gnu/release/kdelta.exe" "dist/kdelta-${VERSION}-x86_64-pc-windows-gnu.exe"
    echo -e "${GREEN}✓ Created: dist/kdelta-${VERSION}-x86_64-pc-windows-gnu.exe${NC}"
    echo ""
else
    echo -e "${YELLOW}Note: Install 'cross' for full cross-compilation support:${NC}"
    echo "  cargo install cross"
    echo ""
fi

# Create checksums
echo "=== Creating checksums ==="
cd dist
if command -v sha256sum &> /dev/null; then
    sha256sum kdelta-* > checksums.txt
    echo -e "${GREEN}✓ Created: dist/checksums.txt${NC}"
elif command -v shasum &> /dev/null; then
    shasum -a 256 kdelta-* > checksums.txt
    echo -e "${GREEN}✓ Created: dist/checksums.txt${NC}"
fi
cd ..
echo ""

# List all build artifacts
echo "=== Build Complete ==="
echo ""
echo "Built artifacts:"
ls -lh dist/ | tail -n +2
echo ""

echo -e "${GREEN}✅ Build successful!${NC}"
echo ""
echo "Binaries are located in: ./dist/"
