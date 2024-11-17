<div align="center">

# learner
*A Rust-powered academic research management system*

[![Library](https://img.shields.io/badge/lib-learner-blue)](https://crates.io/crates/learner)
[![Crates.io](https://img.shields.io/crates/v/learner)](https://crates.io/crates/learner)
[![docs.rs](https://img.shields.io/docsrs/learner)](https://docs.rs/learner)
&nbsp;&nbsp;|&nbsp;&nbsp;
[![CLI](https://img.shields.io/badge/cli-learnerd-blue)](https://crates.io/crates/learnerd)
[![Crates.io](https://img.shields.io/crates/v/learnerd)](https://crates.io/crates/learnerd)
[![CI](https://github.com/autoparallel/learner/actions/workflows/check.yaml/badge.svg)](https://github.com/autoparallel/learner/actions/workflows/check.yaml)
[![codecov](https://codecov.io/gh/autoparallel/learner/branch/main/graph/badge.svg)](https://codecov.io/gh/autoparallel/learner)
[![License](https://img.shields.io/crates/l/learner)](LICENSE)

<img src="assets/header.svg" alt="learner header" width="600px">

</div>

## Features

- Paper Metadata Management
  - Support for arXiv, IACR, and DOI sources
  - Automatic source detection from URLs or identifiers
  - Full metadata extraction including authors and abstracts

- Local Database
  - SQLite-based storage with full-text search
  - Configurable document storage
  - Platform-specific defaults

- CLI Tool (`learnerd`)
  - Paper addition and retrieval
  - Search functionality
  - Document management
  - Daemon support for background operations

## Installation

### Library

```toml
[dependencies]
learner = { version = "*" }  # Uses latest version

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
    let db = Database::open(Database::default_path()).await?;
    
    // Add papers from various sources
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await?;
    paper.save(&db).await?;
    
    // Download associated document
    let storage = Database::default_storage_path();
    paper.download_pdf(&storage).await?;
    
    Ok(())
}
```

### CLI Usage

```bash
# Initialize database
learner init

# Add papers
learner add 2301.07041
learner add "https://arxiv.org/abs/2301.07041"
learner add "10.1145/1327452.1327492"

# Manage documents
learner download arxiv 2301.07041
learner get arxiv 2301.07041
learner search "neural networks"
```

### Daemon Management

`learnerd` can run as a background service for paper monitoring and updates.

#### System Service 
```bash
# Install and start
sudo learnerd daemon install
sudo systemctl enable --now learnerd  # Linux
sudo launchctl load /Library/LaunchDaemons/learnerd.daemon.plist  # macOS

# Remove
sudo learnerd daemon uninstall
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

1. `learner` - Core library
   - Paper metadata extraction and management
   - Database operations and search
   - PDF handling and source-specific clients
   - Error handling and type safety

2. `learnerd` - CLI application
   - Paper and document management interface
   - System daemon capabilities
   - Logging and diagnostics

## Roadmap

### Core Features 
- [x] PDF management
- [x] Content extraction
- [x] Paper removal
- [x] Batch operations
- [ ] Export functionality
- [ ] Enhanced search
- [ ] Custom metadata

### Advanced Features 
- [x] LLM integration
- [ ] Version control and annotations
- [ ] Paper discovery
- [ ] Citation analysis

## Contributing

Contributions welcome! Please open an issue before making major changes.

### CI Workflow

Our automated pipeline ensures:

- Code Quality
  - rustfmt and taplo for consistent formatting
  - clippy for Rust best practices
  - cargo-udeps for dependency management
  - cargo-semver-checks for API compatibility

- Testing
  - Full test suite across workspace and platforms

All checks must pass before merging pull requests.

## Development

This project uses [just](https://github.com/casey/just) as a command runner.

```bash
# Setup
cargo install just
just setup

# Common commands
just test       # run tests
just fmt        # format code
just ci         # run all checks
just build-all  # build all targets
```

> [!TIP]
> Running `just setup` and `just ci` locally is a quick way to get up to speed and see that the repo is working on your system!

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