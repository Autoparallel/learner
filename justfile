# List available commands
default:
    @just --list

# Install required system dependencies
install-deps:
    #!/usr/bin/env bash
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install filosottile/musl-cross/musl-cross
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if command -v apt-get &> /dev/null; then
            sudo apt-get update
            sudo apt-get install -y musl-tools
        elif command -v dnf &> /dev/null; then
            sudo dnf install -y musl-gcc
        elif command -v pacman &> /dev/null; then
            sudo pacman -Sy musl
        fi
    fi

install-cargo-tools:
    #!/usr/bin/env bash
    
    # Helper function to check and install cargo tools
    function ensure_installed() {
        local tool=$1
        local install_cmd=$2
        if ! command -v "$tool" &> /dev/null; then
            echo "Installing $tool..."
            eval "$install_cmd"
        else
            echo "✓ $tool already installed"
        fi
    }
    
    ensure_installed "cargo-udeps" "cargo install cargo-udeps --locked"
    ensure_installed "cargo-semver-checks" "cargo install cargo-semver-checks"
    ensure_installed "taplo" "cargo install taplo-cli"

# Install nightly rust
install-rust-nightly:
    rustup install nightly

# Install required Rust targets
install-targets:
    rustup target add x86_64-unknown-linux-musl aarch64-apple-darwin

# Setup complete development environment
setup: install-deps install-targets install-cargo-tools install-rust-nightly
    @echo "Development environment setup complete!"

# Build native target (lib, tests, examples, etc)
build:
    cargo build --workspace --all-targets

# Build all platforms
build-all: build-mac build-linux
    @echo "All arch builds completed!"

# Build macOS ARM64
build-mac:
    @echo "Building macOS ARM64..."
    cargo build --workspace --target aarch64-apple-darwin

# Build Linux x86_64
build-linux:
    @echo "Building Linux x86_64..."
    cargo build --workspace --target x86_64-unknown-linux-musl

# Test local target arch code
test:
    cargo test --workspace --all-targets

# Lint local target arch code
lint:
    cargo clippy --workspace --all-targets --all-features

# Lint all target arches
lint-all: lint-mac lint-linux
    @echo "All arch lint completed!"

# Lint macOS ARM64
lint-mac:
    @echo "Checking lint on macOS ARM64..."
    cargo clippy --workspace --all-targets --target aarch64-apple-darwin

# Lint Linux x86_64
lint-linux:
    @echo "Checking lint on Linux x86_64..."
    cargo clippy --workspace --all-targets --target x86_64-unknown-linux-musl

# Check for semver issues in the workspace
semver: 
    cargo semver-checks check-release --workspace

# Format code
fmt:
    cargo fmt --all
    taplo fmt

# Check unused dependencies
udeps:
    cargo +nightly udeps --workspace

# Clean build artifacts
clean:
    cargo clean

# Show environment info
info:
    @echo "OS: $OSTYPE"
    @rustc --version
    @cargo --version
    @echo "Installed targets:"
    @rustup target list --installed

ci:
    #!/usr/bin/env bash
    set -euo pipefail

    # Colors
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    NC='\033[0m' # No Color

    # Array to store failures
    declare -a failures=()

    # Helper function for progress indicator
    function progress() {
        echo -e "${BLUE}${BOLD}Running${NC} ${CYAN}${1}${NC}..."
    }

    # Helper function to capture failures
    function run_check() {
        local name=$1
        shift
        progress "$name"
        if ! "$@" > /tmp/check-output 2>&1; then
            failures+=("$name")
            echo -e "  ${RED}${BOLD}FAILED${NC}"
            echo -e "${RED}----------------------------------------"
            cat /tmp/check-output | sed "s/^/${RED}/" # Prefix each line with red color
            echo -e "----------------------------------------${NC}"
        else
            echo -e "  ${GREEN}${BOLD}PASSED${NC}"
        fi
    }

    echo -e "${BOLD}Starting CI checks${NC}\n"

    # Run all checks
    run_check "Rust formatting" cargo fmt --all -- --check
    run_check "TOML formatting" taplo fmt --check
    run_check "Linux clippy" cargo clippy --target x86_64-unknown-linux-musl --all-targets --all-features -- --deny warnings
    run_check "macOS clippy" cargo clippy --target aarch64-apple-darwin --all-targets --all-features -- --deny warnings
    run_check "Linux build" cargo build --target x86_64-unknown-linux-musl --workspace
    run_check "macOS build" cargo build --target aarch64-apple-darwin --workspace
    run_check "Test suite" cargo test --verbose --workspace
    run_check "Unused dependencies" cargo +nightly udeps --workspace
    run_check "Semver compatibility" cargo semver-checks check-release --workspace

    echo -e "\n${BOLD}CI Summary:${NC}"
    if [ ${#failures[@]} -eq 0 ]; then
        echo -e "${GREEN}${BOLD}All checks passed successfully!${NC}"
        exit 0
    else
        echo -e "${RED}${BOLD}The following checks failed:${NC}"
        for failure in "${failures[@]}"; do
            echo -e "${RED}  • ${failure}${NC}"
        done
        exit 1
    fi