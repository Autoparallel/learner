<div align="center">

# learner
*A Rust-powered academic research management system*

[![Library](https://img.shields.io/badge/lib-learner-blue)](https://crates.io/crates/learner)
[![Crates.io](https://img.shields.io/crates/v/learner?color=orange)](https://crates.io/crates/learner)
[![docs.rs](https://img.shields.io/docsrs/learner)](https://docs.rs/learner)
[![Downloads](https://img.shields.io/crates/d/learner.svg)](https://crates.io/crates/learner)
&nbsp;&nbsp;|&nbsp;&nbsp;
[![CLI](https://img.shields.io/badge/cli-learnerd-blue)](https://crates.io/crates/learnerd)
[![Crates.io](https://img.shields.io/crates/v/learnerd?color=orange)](https://crates.io/crates/learnerd)
[![Downloads](https://img.shields.io/crates/d/learnerd.svg)](https://crates.io/crates/learnerd)

[![CI](https://github.com/autoparallel/learner/actions/workflows/check.yaml/badge.svg)](https://github.com/autoparallel/learner/actions/workflows/check.yaml)
[![License](https://img.shields.io/crates/l/learner)](LICENSE)

<img src="assets/header.svg" alt="learner header" width="600px">

</div>

## Features

- Academic Paper Management
  - Extract metadata from multiple sources (arXiv, IACR, DOI)
  - Support for both URLs and direct identifiers
  - Automatic source detection
  - Full paper metadata including authors, abstracts, and publication dates

- Local Database Management
  - SQLite-based storage for offline access
  - Full-text search capabilities
  - Case-insensitive title search
  - Duplicate detection and handling
  - Platform-specific default locations
  - PDF management with configurable storage location

- Command Line Interface (`learnerd`)
  - Interactive database management
  - Paper addition and retrieval
  - Search functionality
  - PDF downloading and management
  - Beautiful, colored output
  - Detailed logging options

## Installation

### Library

Add this to your `Cargo.toml`:

```toml
[dependencies]
learner = "0.2"  # Core library
```

### CLI Tool

```bash
cargo install learnerd
```
which will install a binary you can reference with the command `learner`.

## Usage

### Library Usage

```rust
use learner::{Paper, Database};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database with default paths
    let db = Database::open(Database::default_path()).await?;
    
    // Add papers from various sources
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await?;
    paper.save(&db).await?;
    
    // Download PDF if available
    let pdf_dir = Database::default_pdf_path();
    paper.download_pdf(pdf_dir).await?;
    
    // Add papers from other sources
    let paper = Paper::new("10.1145/1327452.1327492").await?;  // From DOI
    let paper = Paper::new("2023/123").await?;                 // From IACR
    
    Ok(())
}
```

### CLI Usage

```bash
# Initialize a new database (interactive)
learner init

# Add a paper (auto-detects source)
learner add 2301.07041
learner add "https://arxiv.org/abs/2301.07041"
learner add "10.1145/1327452.1327492"

# Skip PDF download
learner add 2301.07041 --no-pdf

# Download PDF for existing paper
learner download arxiv 2301.07041

# Retrieve paper details
learner get arxiv 2301.07041

# Search papers
learner search "neural networks"

# Verbose output for debugging
learner -v add 2301.07041

# Clean up database (with confirmation)
learner clean
```

### Daemon Management

`learnerd` can run as a background service for paper monitoring and updates.

#### System Service Installation
**Linux (`sytemd`):**
```bash
# Install and start
sudo learnerd daemon install
sudo systemctl daemon-reload
sudo systemctl enable --now learnerd

# Manage
sudo systemctl status learnerd     # Check status
sudo journalctl -u learnerd -f     # View logs
sudo systemctl restart learnerd    # Restart service

# Remove
sudo systemctl disable --now learnerd
sudo learnerd daemon uninstall
```

**MacOS (`launchd`):**
```bash
# Install and start
sudo learner daemon install
sudo launchctl load /Library/LaunchDaemons/learnerd.daemon.plist

# Manage
sudo launchctl list | grep learnerd           # Check status
tail -f /Library/Logs/learnerd/learnerd.log   # View logs
sudo launchctl kickstart -k system/learnerd.daemon  # Restart

# Remove
sudo launchctl bootout system/learnerd.daemon
sudo learner daemon uninstall
```

#### Logs
- Linux: /var/log/learnerd/
- macOS: /Library/Logs/learnerd/

Files: `learnerd.log` (main, rotated daily), `stdout.log`, `stderr.log`

#### Troubleshooting

- **Permission Errors:** Check ownership of log directories
- **Won't Start:** Check system logs and remove stale PID file if present
- **Installation:** Run commands as root/sudo

## Project Structure

The project consists of two main components:

1. `learner` - Core library providing:
   - Paper metadata extraction
   - Database management
   - PDF download capabilities
   - Source-specific clients (arXiv, IACR, DOI)
   - Error handling

2. `learnerd` - CLI application offering:
   - User-friendly interface
   - PDF management
   - Interactive confirmations
   - Colored output
   - Logging and debugging capabilities

## Roadmap

### Phase 1: Core Improvements 
- [x] PDF management
- [ ] PDF content extraction
- [ ] DB/Paper removal functionality
- [ ] Batch paper operations
- [ ] Export capabilities
- [ ] Enhanced search features
- [ ] Custom metadata fields

### Phase 2: Advanced Features 
- [ ] LLM-powered paper analysis
- [ ] PDF daemon for paper versioning and annotations
- [ ] Automated paper discovery
- [ ] Citation graph analysis
- [ ] Web interface

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. Before making major changes, please open an issue first to discuss what you would like to change.

### Continuous Integration
The project maintains code quality through automated CI workflows:

- Code Formatting
  - rustfmt: Enforces consistent Rust code formatting
  - taplo: Ensures TOML files (like Cargo.toml) follow consistent style

- Code Quality
  - clippy: Rust's official linter for catching common mistakes and enforcing best practices
  - cargo-udeps: Identifies unused dependencies to keep the project lean


- Testing
  - Runs the full test suite across all workspace members
  - [ ] TODO: Check cross-platform 

- Release Safety
  - cargo-semver-checks: Verifies that version bumps follow semantic versioning rules
  - Prevents accidental breaking changes in minor/patch releases

All CI checks must pass before merging pull requests, maintaining consistent quality across contributions.

## Development

This project uses [just](https://github.com/casey/just) as a command runner.

```bash
# First time setup
cargo install just
just setup  # installs dependencies, targets, and required tools
```

### Common Commands

```bash
just         # show all available commands
just ci      # run all checks (fmt, lint, test, build)
just test    # run test suite
just fmt     # format code
```

### Platform Builds

```bash
just build-all        # build all targets
just build-linux      # linux (x86_64-musl)
just build-mac        # macOS (arm64)
```

All commands support standard Cargo flags:
```bash
just test -- --release  # run tests in release mode
just build -- --quiet   # quiet build output
```

> [!TIP]
> Running `just ci` locally ensures your code will pass CI checks!

### System Requirements

The setup command will attempt to install required system dependencies, but if you need to install them manually:

#### Linux (Debian/Ubuntu)
```bash
sudo apt-get install pkg-config libssl-dev
```

#### macOS
```bash
brew install openssl@3
export OPENSSL_DIR=$(brew --prefix openssl@3)  # Add to your shell profile
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [arXiv API](https://arxiv.org/help/api/index) for paper metadata
- [IACR](https://eprint.iacr.org/) for cryptography papers
- [CrossRef](https://www.crossref.org/) for DOI resolution
- [SQLite](https://www.sqlite.org/) for local database support

---

<div align="center">
Made for making learning sh*t less annoying.
</div>