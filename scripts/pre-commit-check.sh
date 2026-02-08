#!/bin/bash
# Pre-commit check script for AGG Rust Port

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}Running pre-commit checks for AGG Rust Port...${NC}"

EXIT_CODE=0

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Cargo is not installed or not in PATH${NC}"
    exit 1
fi

echo -e "\n${YELLOW}Running unit tests...${NC}"
if cargo test --lib --quiet; then
    echo -e "${GREEN}All unit tests passed${NC}"
else
    echo -e "${RED}Some unit tests failed${NC}"
    EXIT_CODE=1
fi

# Run integration tests if they exist
if [ -d "tests" ] && ls tests/*.rs 1> /dev/null 2>&1; then
    echo -e "\n${YELLOW}Running integration tests...${NC}"
    if cargo test --test "*" --quiet; then
        echo -e "${GREEN}Integration tests passed${NC}"
    else
        echo -e "${RED}Integration tests failed${NC}"
        EXIT_CODE=1
    fi
fi

echo -e "\n${YELLOW}Checking code formatting...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}Code formatting is correct${NC}"
else
    echo -e "${RED}Code formatting issues found${NC}"
    echo -e "${YELLOW}   Run 'cargo fmt --all' to fix formatting${NC}"
    EXIT_CODE=1
fi

echo -e "\n${YELLOW}Running clippy lints...${NC}"
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${GREEN}No clippy warnings found${NC}"
else
    echo -e "${RED}Clippy warnings found${NC}"
    EXIT_CODE=1
fi

echo -e "\n${YELLOW}Running build check...${NC}"
if cargo build --all-targets; then
    echo -e "${GREEN}Build successful${NC}"
else
    echo -e "${RED}Build failed${NC}"
    EXIT_CODE=1
fi

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}All pre-commit checks passed!${NC}"
    echo -e "${GREEN}   Your AGG Rust code is ready for commit.${NC}"
else
    echo -e "${RED}Pre-commit checks failed!${NC}"
    echo -e "${RED}   Please fix the issues above before committing.${NC}"
    echo -e "\n${CYAN}Helpful commands:${NC}"
    echo "   cargo test --verbose                    - Run tests with detailed output"
    echo "   cargo fmt --all                         - Fix formatting issues"
    echo "   cargo clippy --fix --all-targets        - Fix clippy warnings automatically"
fi

exit $EXIT_CODE
