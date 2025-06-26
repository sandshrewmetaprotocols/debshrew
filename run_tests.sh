#!/bin/bash

# Debshrew Test Runner
# This script demonstrates the new comprehensive test suite

set -e

echo "🧪 Debshrew Comprehensive Test Suite"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to run a test category
run_test_category() {
    local category=$1
    local description=$2
    
    echo -e "\n${BLUE}📋 Running $description${NC}"
    echo "----------------------------------------"
    
    if cargo test $category --lib -- --nocapture; then
        echo -e "${GREEN}✅ $description passed!${NC}"
    else
        echo -e "${RED}❌ $description failed!${NC}"
        exit 1
    fi
}

# Check if we need to build WASM
if [ ! -f "./target/wasm32-unknown-unknown/release/debshrew_minimal.wasm" ]; then
    echo -e "${YELLOW}🔨 Building debshrew-minimal WASM module...${NC}"
    DEBSHREW_BUILD_WASM=1 cargo build --target wasm32-unknown-unknown --release -p debshrew-minimal
fi

# Run test categories
echo -e "${BLUE}🚀 Starting comprehensive test suite...${NC}"

# Unit tests
run_test_category "test_" "Unit Tests"

# Integration tests  
run_test_category "comprehensive_e2e_test" "Comprehensive E2E Tests"

# Reorg tests
run_test_category "reorg_focused_test" "Reorg-Focused Tests"

# Integration tests
run_test_category "integration_tests" "Integration Tests"

# Run all tests together for final verification
echo -e "\n${BLUE}🔄 Running complete test suite...${NC}"
echo "----------------------------------------"

if cargo test --lib; then
    echo -e "\n${GREEN}🎉 All tests passed! Debshrew refactoring complete!${NC}"
    echo ""
    echo "✨ New Features:"
    echo "  • Generic traits for dependency injection"
    echo "  • In-memory metashrew adapter for fast testing"
    echo "  • Comprehensive e2e test suite"
    echo "  • debshrew-minimal transform for testing"
    echo "  • Chain reorganization testing"
    echo "  • CDC message validation"
    echo "  • Block simulation and state management"
    echo ""
    echo "🏃‍♂️ Quick test commands:"
    echo "  cargo test                           # Run all tests"
    echo "  cargo test comprehensive_e2e_test    # E2E tests"
    echo "  cargo test reorg_focused_test        # Reorg tests"
    echo "  cargo test integration_tests         # Integration tests"
    echo "  RUST_LOG=debug cargo test -- --nocapture  # With logging"
    echo ""
else
    echo -e "\n${RED}💥 Some tests failed! Check the output above.${NC}"
    exit 1
fi