#!/bin/bash
# KoruDelta - Pre-commit/CI Check Script
# Runs formatting, linting, tests, and benchmarks
# Use: ./scripts/check.sh [--quick]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
QUICK_MODE=false
if [[ "$1" == "--quick" ]]; then
    QUICK_MODE=true
fi

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   KoruDelta - Status Check                                   ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Navigate to project root
cd "$(dirname "$0")/.."

# Verify we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found. Run from project root or scripts directory.${NC}"
    exit 1
fi

# Step 1: Format check
echo -e "${BLUE}[1/4]${NC} Checking formatting..."
if cargo fmt -- --check; then
    echo -e "${GREEN}✓ Formatting OK${NC}"
else
    echo -e "${RED}✗ Formatting issues found${NC}"
    echo -e "${YELLOW}  Run 'cargo fmt' to fix${NC}"
    exit 1
fi
echo ""

# Step 2: Clippy
echo -e "${BLUE}[2/4]${NC} Running clippy..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${GREEN}✓ Clippy OK${NC}"
else
    echo -e "${RED}✗ Clippy found issues${NC}"
    exit 1
fi
echo ""

# Step 3: Tests
echo -e "${BLUE}[3/4]${NC} Running tests..."
if cargo test --release; then
    echo -e "${GREEN}✓ Tests OK${NC}"
else
    echo -e "${RED}✗ Tests failed${NC}"
    exit 1
fi
echo ""

# Step 4: Benchmarks (skip in quick mode)
if [ "$QUICK_MODE" = true ]; then
    echo -e "${BLUE}[4/4]${NC} Skipping benchmarks (quick mode)"
    echo -e "${YELLOW}  Run without --quick to include benchmarks${NC}"
else
    echo -e "${BLUE}[4/4]${NC} Running benchmarks (compile check)..."
    if cargo bench --no-run; then
        echo -e "${GREEN}✓ Benchmarks compile OK${NC}"
    else
        echo -e "${RED}✗ Benchmark compilation failed${NC}"
        exit 1
    fi
fi
echo ""

# Summary
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║   ✅ All checks passed!                                      ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
