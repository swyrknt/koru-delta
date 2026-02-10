#!/bin/bash
# KoruDelta - Publish Python package
# Builds and publishes to PyPI

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   KoruDelta - Publish to PyPI                                ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Navigate to project root
cd "$(dirname "$0")/../bindings/python"

# Check if logged into PyPI
echo "Checking PyPI authentication..."
if ! python3 -m twine --version &>/dev/null; then
    echo -e "${YELLOW}Installing twine...${NC}"
    pip3 install twine
fi

# Get version
VERSION=$(grep '^version' Cargo.toml 2>/dev/null || grep '^version' pyproject.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo -e "Version: ${YELLOW}${VERSION}${NC}"
echo ""

# Setup build environment
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Building Python package                                    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Create virtual environment if needed
if [ ! -d "venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
fi

source venv/bin/activate

# Install build tools
pip install maturin build twine -q

# Build wheel
echo "Building wheel with maturin..."
maturin build --release

echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Check
echo -e "${BLUE}Running twine check...${NC}"
twine check target/wheels/*.whl
echo ""

# Publish
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Ready to Publish                                           ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Package: ${YELLOW}koru-delta${NC}"
echo -e "Version: ${YELLOW}${VERSION}${NC}"
echo ""
echo "Upload to:"
echo "  1) PyPI (production)"
echo "  2) TestPyPI"
echo ""
read -p "Choice (1/2/cancel): " -n 1 -r
echo ""

if [[ $REPLY =~ ^[1]$ ]]; then
    echo "Publishing to PyPI..."
    twine upload target/wheels/*.whl
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   ✅ Successfully published to PyPI!                         ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "Package: ${GREEN}koru-delta==${VERSION}${NC}"
    echo -e "View at: ${BLUE}https://pypi.org/project/koru-delta/${NC}"
    echo ""
    echo "Install with:"
    echo "  pip install koru-delta"
    echo ""
elif [[ $REPLY =~ ^[2]$ ]]; then
    echo "Publishing to TestPyPI..."
    twine upload --repository-url https://test.pypi.org/legacy/ target/wheels/*.whl
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   ✅ Published to TestPyPI                                   ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Install with:"
    echo "  pip install --index-url https://test.pypi.org/simple/ koru-delta"
    echo ""
else
    echo "Publish cancelled."
    exit 0
fi
