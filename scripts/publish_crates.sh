#!/bin/bash
# KoruDelta - Publish to crates.io
# Runs all checks before publishing

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Capture script directory BEFORE changing directories
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   KoruDelta - Publish to crates.io                           ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Navigate to project root
cd "$(dirname "$0")/.."

# Verify we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found.${NC}"
    exit 1
fi

# Get current version
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo -e "Version: ${YELLOW}${VERSION}${NC}"
echo ""

# Pre-flight checks
echo -e "${BLUE}Running pre-flight checks...${NC}"
echo ""

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    echo -e "${YELLOW}⚠️  Warning: You have uncommitted changes${NC}"
    echo ""
    git status --short
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborting."
        exit 1
    fi
fi

# Check if on main/master branch
BRANCH=$(git branch --show-current)
if [[ "$BRANCH" != "main" && "$BRANCH" != "master" ]]; then
    echo -e "${YELLOW}⚠️  Warning: Not on main/master branch (current: ${BRANCH})${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborting."
        exit 1
    fi
fi

# Check if logged in to crates.io
echo "Checking crates.io authentication..."
if ! cargo login --help &>/dev/null; then
    echo -e "${RED}❌ cargo not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Cargo available${NC}"
echo ""

# Run all checks
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Running All Checks                                         ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Run check.sh
if [ -f "$SCRIPT_DIR/check.sh" ]; then
    "$SCRIPT_DIR/check.sh"
else
    echo -e "${RED}Error: check.sh not found${NC}"
    exit 1
fi

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Checks failed. Fix issues before publishing.${NC}"
    exit 1
fi

echo ""

# Dry run
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Dry Run                                                    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo "Running cargo publish --dry-run..."
if cargo publish --dry-run; then
    echo -e "${GREEN}✓ Dry run successful${NC}"
else
    echo -e "${RED}❌ Dry run failed${NC}"
    exit 1
fi
echo ""

# Confirm publish
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Ready to Publish                                           ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Package: ${YELLOW}koru-delta${NC}"
echo -e "Version: ${YELLOW}${VERSION}${NC}"
echo ""
read -p "Publish to crates.io? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Publish cancelled."
    exit 0
fi

# Publish
echo ""
echo "Publishing..."
if cargo publish; then
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   ✅ Successfully published to crates.io!                    ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "Package: ${GREEN}koru-delta@${VERSION}${NC}"
    echo -e "View at: ${BLUE}https://crates.io/crates/koru-delta${NC}"
    echo ""
    echo "Install with:"
    echo "  cargo add koru-delta"
    echo "  cargo install koru-delta  # For CLI"
    echo ""

    # Suggest tagging
    echo -e "${YELLOW}Don't forget to tag this release:${NC}"
    echo "  git tag v${VERSION}"
    echo "  git push --tags"
    echo ""
else
    echo -e "${RED}❌ Publish failed${NC}"
    exit 1
fi
