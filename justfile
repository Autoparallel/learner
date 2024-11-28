default:
    @just --list

[private]
warn := "\\033[33m"
error := "\\033[31m"
info := "\\033[34m"
success := "\\033[32m"
reset := "\\033[0m"
bold := "\\033[1m"

# Print formatted headers without shell scripts
[private]
header msg:
    @printf "{{info}}{{bold}}==> {{msg}}{{reset}}\n"

# Install required system dependencies
install-deps:
    @just header "Installing system dependencies"
    # macOS
    if command -v brew > /dev/null; then \
        brew install filosottile/musl-cross/musl-cross mingw-w64; \
    fi
    # Linux
    if command -v apt-get > /dev/null; then \
        sudo apt-get update && sudo apt-get install -y musl-tools mingw-w64; \
    elif command -v dnf > /dev/null; then \
        sudo dnf install -y musl-gcc mingw64-gcc; \
    elif command -v pacman > /dev/null; then \
        sudo pacman -Sy musl mingw-w64-gcc; \
    fi

# Install cargo tools
install-cargo-tools:
    @just header "Installing Cargo tools"
    # cargo-udeps
    if ! command -v cargo-udeps > /dev/null; then \
        printf "{{info}}Installing cargo-udeps...{{reset}}\n" && \
        cargo install cargo-udeps --locked; \
    else \
        printf "{{success}}✓ cargo-udeps already installed{{reset}}\n"; \
    fi
    # cargo-semver-checks
    if ! command -v cargo-semver-checks > /dev/null; then \
        printf "{{info}}Installing cargo-semver-checks...{{reset}}\n" && \
        cargo install cargo-semver-checks; \
    else \
        printf "{{success}}✓ cargo-semver-checks already installed{{reset}}\n"; \
    fi
    # taplo
    if ! command -v taplo > /dev/null; then \
        printf "{{info}}Installing taplo...{{reset}}\n" && \
        cargo install taplo-cli; \
    else \
        printf "{{success}}✓ taplo already installed{{reset}}\n"; \
    fi

# Install nightly rust
install-rust-nightly:
    @just header "Installing Rust nightly"
    rustup install nightly

# Install required Rust targets
install-targets:
    @just header "Installing Rust targets"
    rustup target add x86_64-unknown-linux-musl aarch64-apple-darwin x86_64-pc-windows-gnu

# Setup complete development environment
setup: install-deps install-targets install-cargo-tools install-rust-nightly
    @printf "{{success}}{{bold}}Development environment setup complete!{{reset}}\n"

# Build with local OS target
build:
    @just header "Building workspace"
    cargo build --workspace --all-targets

# Build with MacOS and Linux targets
build-all: build-mac build-linux build-windows
    @printf "{{success}}{{bold}}All arch builds completed!{{reset}}\n"

# Build just the target
build-mac:
    @just header "Building macOS ARM64"
    cargo build --workspace --target aarch64-apple-darwin

# Build the Linux target
build-linux:
    @just header "Building Linux x86_64"
    cargo build --workspace --target x86_64-unknown-linux-musl

# Build the Windows target
build-windows:
    @just header "Building Windows x86_64"
    cargo build --workspace --target x86_64-pc-windows-gnu

# Run the tests on your local OS
test:
    @just header "Running main test suite"
    cargo test --workspace --all-targets --all-features
    @just header "Running doc tests"
    cargo test --workspace --doc

# Run clippy for the workspace on your local OS
lint:
    @just header "Running clippy"
    cargo clippy --workspace --all-targets --all-features

# Run clippy for the workspace on MacOS and Linux targets
lint-all: lint-mac lint-linux lint-windows
    @printf "{{success}}{{bold}}All arch lint completed!{{reset}}\n"

# Run clippy for the MacOS target
lint-mac:
    @just header "Checking lint on macOS ARM64"
    cargo clippy --workspace --all-targets --target aarch64-apple-darwin

# Run clippy for the Linux target
lint-linux:
    @just header "Checking lint on Linux x86_64"
    cargo clippy --workspace --all-targets --target x86_64-unknown-linux-musl

# Run clippy for the Windows target
lint-windows:
    @just header "Checking lint on Windows x86_64"
    cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu

# Check for semantic versioning for workspace crates
semver:
    @just header "Checking semver compatibility"
    cargo semver-checks check-release --workspace

# Run format for the workspace
fmt:
    @just header "Formatting code"
    cargo fmt --all
    taplo fmt

# Check for unused dependencies in the workspace
udeps:
    @just header "Checking unused dependencies"
    cargo +nightly udeps --workspace

# Run cargo clean to remove build artifacts
clean:
    @just header "Cleaning build artifacts"
    cargo clean

# Install learnerd binary with TUI feature in debug mode
debug:
    @just header "Installing learnerd in debug mode"
    cargo install --path crates/learnerd --features tui --debug

# Show your relevant environment information
info:
    @just header "Environment Information"
    @printf "{{info}}OS:{{reset}} %s\n" "$(uname -s)"
    @printf "{{info}}Rust:{{reset}} %s\n" "$(rustc --version)"
    @printf "{{info}}Cargo:{{reset}} %s\n" "$(cargo --version)"
    @printf "{{info}}Installed targets:{{reset}}\n"
    @rustup target list --installed | sed 's/^/  /'

# Run all possible CI checks (cannot test a non-local OS target!)
ci:
    @printf "{{bold}}Starting CI checks{{reset}}\n\n"
    @ERROR=0; \
    just run-single-check "Rust formatting" "cargo fmt --all -- --check" || ERROR=1; \
    just run-single-check "TOML formatting" "taplo fmt --check" || ERROR=1; \
    just run-single-check "Linux build" "cargo build --target x86_64-unknown-linux-musl --workspace" || ERROR=1; \
    just run-single-check "macOS build" "cargo build --target aarch64-apple-darwin --workspace" || ERROR=1; \
    just run-single-check "Windows build" "cargo build --target x86_64-pc-windows-gnu --workspace" || ERROR=1; \
    just run-single-check "Linux clippy" "cargo clippy --target x86_64-unknown-linux-musl --all-targets --all-features -- --deny warnings" || ERROR=1; \
    just run-single-check "macOS clippy" "cargo clippy --target aarch64-apple-darwin --all-targets --all-features -- --deny warnings" || ERROR=1; \
    just run-single-check "Windows clippy" "cargo clippy --target x86_64-pc-windows-gnu --all-targets --all-features -- --deny warnings" || ERROR=1; \
    just run-single-check "Test suite" "cargo test --verbose --workspace" || ERROR=1; \
    just run-single-check "Unused dependencies" "cargo +nightly udeps --workspace" || ERROR=1; \
    just run-single-check "Semver compatibility" "cargo semver-checks check-release --workspace" || ERROR=1; \
    printf "\n{{bold}}CI Summary:{{reset}}\n"; \
    if [ $ERROR -eq 0 ]; then \
        printf "{{success}}{{bold}}All checks passed successfully!{{reset}}\n"; \
    else \
        printf "{{error}}{{bold}}Some checks failed. See output above for details.{{reset}}\n"; \
        exit 1; \
    fi

# Run a single check and return status (0 = pass, 1 = fail)
[private]
run-single-check name command:
    #!/usr/bin/env sh
    printf "{{info}}{{bold}}Running{{reset}} {{info}}%s{{reset}}...\n" "{{name}}"
    if {{command}} > /tmp/check-output 2>&1; then
        printf "  {{success}}{{bold}}PASSED{{reset}}\n"
        exit 0
    else
        printf "  {{error}}{{bold}}FAILED{{reset}}\n"
        printf "{{error}}----------------------------------------\n"
        while IFS= read -r line; do
            printf "{{error}}%s{{reset}}\n" "$line"
        done < /tmp/check-output
        printf "{{error}}----------------------------------------{{reset}}\n"
        exit 1
    fi

# Success summary (called if all checks pass)
[private]
_ci-summary-success:
    @printf "\n{{bold}}CI Summary:{{reset}}\n"
    @printf "{{success}}{{bold}}All checks passed successfully!{{reset}}\n"

# Failure summary (called if any check fails)
[private]
_ci-summary-failure:
    @printf "\n{{bold}}CI Summary:{{reset}}\n"
    @printf "{{error}}{{bold}}Some checks failed. See output above for details.{{reset}}\n"
    @exit 1