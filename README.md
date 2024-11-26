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
[![License](https://img.shields.io/crates/l/learner)](LICENSE)

<img src="assets/header.svg" alt="learner header" width="600px">

</div>

[Features](#features)
[Installation](#installation)
[Usage](#usage)
[Configuration](#configuration)
[Roadmap](#roadmap)
[Contributing](#contributing)
[Development](#development)
[License](#license)
[Acknowledgements](#acknowledgements)

---
## Features

- Paper Metadata Management
  - Support for arXiv, IACR, and DOI sources
  - Automatic source detection from URLs or identifiers
  - Full metadata extraction including authors and abstracts

- Local Database
  - SQLite-based storage with full-text search
  - Configurable document storage
  - Platform-specific defaults

- Interactive Interfaces
  - Terminal User Interface (TUI) with vim-style navigation
  - Command-line interface (CLI) for scripting and automation
  - Search, filter, and preview functionality
  - Document management and viewing
  - Daemon support for background operations

## Installation

### Library

```toml
[dependencies]
learner = { version = "*" }  # Uses latest version
```

### CLI Tool

```bash
cargo +nightly install learnerd --features tui
```

This installs both the CLI tool and TUI interface, accessible via the `learner` command.

## Usage

### Library Usage

```rust
use learner::{Paper, Database};

#[tokio::main]
async fn main() -> Result> {
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

### Command Line Interface

```bash
# Initialize database
learner init --default-retrievers

# Add papers
learner add 2301.07041
learner add "https://arxiv.org/abs/2301.07041" --pdf
learner add "10.1145/1327452.1327492" --no-pdf

# Search papers
learner search "quantum computing"
learner search "quantum" --author "Feynman" --detailed
learner search "neural" --source arxiv --before 2023

# Remove papers
learner remove "outdated paper"
learner remove "temp" --force --remove-pdf
```

### Terminal User Interface
If you install with
```
cargo install learnerd --features tui
```
you can get access to a Terminal User Interface (TUI). To launch the interactive TUI just do:
```bash
learner
```

TUI navigation:
- `↑`/`k`, `↓`/`j`: Navigate papers
- `←`/`h`, `→`/`l`: Switch panes
- `:`: Enter command mode
- `o`: Open selected PDF
- `q`: Quit

TUI commands:
```bash
:add      # Add a paper
:remove   # Remove paper(s)
:search   # Search papers
```

(TODO:) Search within TUI supports all filters:
```bash
:search "quantum" --author "Feynman"
:search "neural" --source arxiv --before 2023
```

### System Daemon Management

`learnerd` can run as a background service for paper monitoring and updates.
Currently, there are no distinct processes it runs but there is a tracking issue: [issue #83](https://github.com/Autoparallel/learner/issues/83).

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

## Configuration

The `learner` system uses a flexible configuration system that allows customization of paper sources, storage paths, and retrieval behavior.

### Default Locations

- **Config**: 
  - Linux: `~/.config/learner/config.toml`
  - macOS: `~/Library/Application Support/learner/config.toml`
  - Windows: `%APPDATA%\learner\config.toml`

- **Database**:
  - Linux: `~/.local/share/learner/learner.db`
  - macOS: `~/Library/Application Support/learner/learner.db`
  - Windows: `%APPDATA%\learner\learner.db`

- **Papers**:
  - Linux/macOS: `~/Documents/learner/papers`
  - Windows: `Documents\learner\papers`

### Configuration File

The configuration file (`config.toml`) allows you to customize:
```toml
# Base configuration
[config]
database_path = "/custom/path/to/db.sqlite" # Where the datbase itself is stored
storage_path = "/custom/path/to/papers"     # Where the documents are stored
retrievers_path = "/custom/path/to/papers"  # Where configuration for retrievers are stored
```

### Adding Custom Sources

1. Create a source configuration in TOML:
```toml
[sources.new_source]
name = "New Paper Source"
base_url = "https://api.example.com"
pattern = "^PREFIX-\\d+$"  # Regex for identifier validation
endpoint_template = "/api/v1/papers/{identifier}"
headers = { "API-Key" = "your-key" }  # Optional headers

# For JSON responses
response_format = { type = "json" }
field_maps.title = { path = "data.title" }
field_maps.abstract = { path = "data.description" }
field_maps.pdf_url = { 
    path = "data.files.pdf",
    transform = { type = "url", base = "https://cdn.example.com", suffix = ".pdf" }
}

# For XML responses
response_format = { type = "xml" }
field_maps.title = { path = "paper/title" }
field_maps.authors = { path = "paper/authors/author" }
```
Put this TOML configuration file in your `~/.learner/retrievers/` (or equivalent) directory.
Examples can be found in `crates/learner/config/retrievers/`.

### Source Requirements

Custom sources must provide:
1. A unique identifier pattern (regex)
2. An API endpoint that returns paper metadata
3. Field mappings for required metadata:
   - Title
   - Authors
   - Abstract
   - Publication date
   - Optional: PDF URL, DOI

### Supported Response Formats

- **JSON**: 
  - Path-based field extraction
  - Value transformations (dates, URLs)
  - Array handling for authors/references

- **XML**:
  - XPath-style field selection
  - Namespace handling
  - Multiple value aggregation

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

- [ ] Generic LLM integration (similar to the configurable `Retriever` abstraction)
- [ ] RAG system
- [ ] Document version control and annotations
- [ ] Paper discovery and streaming
- [ ] Configurable daemon process (e.g., watch file system, RSS, automated LLM querying)
- [ ] REST API and Daemonize so `learner` can be a plugin with/for other apps (e.g., Raycast, Syncthing)
- [ ] Database improvements (more searchable fields, tags, organization)
- [ ] TUI improvements (organization, flexibility, in-terminal paper reading)
- [ ] Citation analysis and related works.

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
