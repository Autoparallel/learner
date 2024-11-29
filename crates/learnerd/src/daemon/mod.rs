//! Daemon implementation for the learnerd service.
//!
//! This module provides functionality for running learnerd as a system service, with support
//! for both systemd (Linux) and launchd (macOS) environments. The daemon handles background
//! tasks such as paper monitoring, metadata updates, and system integration.
//!
//! # Architecture
//!
//! The daemon implementation follows a platform-agnostic core with platform-specific adapters:
//! - Core daemon functionality is implemented in this module
//! - Platform-specific service management is handled in submodules:
//!   - [`linux`] module for systemd integration
//!   - [`macos`] module for launchd integration
//!
//! # Features
//!
//! - Process lifecycle management (start/stop/restart)
//! - System service installation and removal
//! - Structured logging with rotation
//! - Graceful shutdown handling
//! - Platform-specific service integration
//!
//! # Examples
//!
//! Basic daemon management through the CLI:
//!
//! ```bash
//! # Install and start the daemon
//! learnerd daemon install
//!
//! # Use launchd/systemd to manage daemon
//! # ...
//!
//! # Remove associated daemon files
//! learnerd daemon uninstall
//! ```
//!
//! # Platform-Specific Details
//!
//! ## Linux (systemd)
//!
//! The daemon integrates with systemd using a unit file at `/etc/systemd/system/learnerd.service`.
//! Key paths:
//! - PID file: `/var/run/learnerd.pid`
//! - Working directory: `/var/lib/learnerd`
//! - Logs: `/var/log/learnerd`
//!
//! Service management:
//! ```bash
//! sudo systemctl start learnerd
//! sudo systemctl status learnerd
//! sudo journalctl -u learnerd -f
//! ```
//!
//! ## macOS (launchd)
//!
//! The daemon integrates with launchd using a plist at
//! `/Library/LaunchDaemons/learnerd.daemon.plist`. Key paths:
//! - PID file: `/Library/Application Support/learnerd/learnerd.pid`
//! - Working directory: `/Library/Application Support/learnerd`
//! - Logs: `/Library/Logs/learnerd`
//!
//! Service management:
//! ```bash
//! sudo launchctl load /Library/LaunchDaemons/learnerd.daemon.plist
//! sudo launchctl list | grep learnerd
//! ```
//!
//! # Implementation Notes
//!
//! The daemon implementation follows several best practices:
//!
//! 1. Structured logging using the `tracing` ecosystem:
//!    - File-based logging with daily rotation
//!    - System journal integration
//!    - Contextual metadata (thread IDs, source location)
//!
//! 2. Graceful shutdown handling:
//!    - SIGTERM signal handling on Unix systems
//!    - Proper cleanup of PID files and resources
//!
//! 3. Error handling:
//!    - Custom error types via [`LearnerdErrors`]
//!    - Detailed error context and chain
//!    - Platform-specific error mapping
//!
//! # Future Improvements
//!
//! - [ ] Implement Windows service support
//! - [ ] Add configurable monitoring intervals
//! - [ ] Support for plugins/extensions
//! - [ ] Health check endpoint
//! - [ ] Metrics collection
//!
//! # See Also
//!
//! - [`Database`] - Core database functionality
//! - [`Paper`] - Paper metadata management
//! - [`Source`] - Paper source implementations
//! - [systemd documentation](https://www.freedesktop.org/software/systemd/man/systemd.service.html)
//! - [launchd documentation](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)

use std::{fs, path::PathBuf};

use nix::{
  sys::signal::{self, Signal},
  unistd::Pid,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use tracing_appender::rolling;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::*;

#[cfg(target_os = "linux")] pub mod linux;
#[cfg(target_os = "linux")] pub use linux::*;
#[cfg(target_os = "macos")] pub mod macos;
#[cfg(target_os = "macos")] pub use macos::*;

/// Commands available for daemon management through the CLI.
#[derive(Subcommand, Clone, Copy)]
pub enum DaemonCommands {
  /// Start the daemon process.
  ///
  /// This command will:
  /// 1. Create required directories
  /// 2. Initialize logging
  /// 3. Start the main daemon process
  /// 4. Create PID file
  Start,
  /// Stop a running daemon process.
  ///
  /// This command will:
  /// 1. Read the PID file
  /// 2. Send SIGTERM to the process
  /// 3. Clean up the PID file
  Stop,
  /// Restart the daemon process.
  ///
  /// Equivalent to running `stop` followed by `start` with a 1-second delay
  /// between operations to ensure clean shutdown.
  Restart,
  /// Install the daemon as a system service.
  ///
  /// This command will:
  /// 1. Create service definition file
  /// 2. Register with system service manager
  /// 3. Configure logging and directories
  Install,
  /// Remove the daemon from system services.
  ///
  /// This command will:
  /// 1. Stop the service if running
  /// 2. Remove service definition file
  /// 3. Unregister from service manager
  Uninstall,
  /// Display current daemon status.
  ///
  /// Shows:
  /// - Running status and PID
  /// - Log file locations
  /// - Service registration status
  Status,
}

/// Configuration for the daemon service.
///
/// # Platform-Specific Defaults
///
/// ## Linux
/// ```text
/// pid_file: "/var/run/learnerd.pid"
/// working_dir: "/var/lib/learnerd"
/// log_dir: "/var/log/learnerd"
/// ```
///
/// ## macOS
/// ```text
/// pid_file: "/Library/Application Support/learnerd/learnerd.pid"
/// working_dir: "/Library/Application Support/learnerd"
/// log_dir: "/Library/Logs/learnerd"
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Daemon {
  /// Path to store the PID file.
  ///
  /// This file contains the process ID of the running daemon and is used
  /// for process management and status checks.
  pub pid_file:    PathBuf,
  /// Working directory for the daemon.
  ///
  /// This directory holds runtime data and temporary files. It should be
  /// persistent across daemon restarts.
  pub working_dir: PathBuf,
  /// Directory for log files.
  ///
  /// Contains:
  /// - Daily rotating log files
  /// - stdout/stderr capture
  /// - Debug logs
  pub log_dir:     PathBuf,
}

impl Default for Daemon {
  fn default() -> Self {
    Self {
      pid_file:    PathBuf::from(DEFAULT_PID_FILE),
      working_dir: PathBuf::from(DEFAULT_WORKING_DIR),
      log_dir:     PathBuf::from(DEFAULT_LOG_DIR),
    }
  }
}

impl Daemon {
  /// Creates a new daemon instance with platform-specific default configuration.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use learnerd::daemon::Daemon;
  ///
  /// let daemon = Daemon::new();
  /// ```
  pub fn new() -> Self { Self::default() }

  /// Starts the daemon process and initializes logging.
  ///
  /// Sets up daily log rotation and dual logging to both files and system journal.
  /// Creates required directories if they don't exist.
  ///
  /// # Errors
  ///
  /// Returns `LearnerdErrors` if:
  /// - Directory creation fails
  /// - Log initialization fails
  /// - Daemon process fails to start
  pub fn start(&self) -> Result<()> {
    // Ensure directories exist
    fs::create_dir_all(&self.working_dir)?;
    fs::create_dir_all(&self.log_dir)?;

    // Configure file logging
    let file_appender = rolling::RollingFileAppender::builder()
      .rotation(rolling::Rotation::DAILY)
      .filename_prefix("learnerd")
      .filename_suffix("log")
      .build(&self.log_dir)?;

    // Create a file layer for file logging
    let file_layer = tracing_subscriber::fmt::layer()
      .with_writer(file_appender)
      .with_ansi(false)
      .with_thread_ids(true)
      .with_target(true)
      .with_file(true)
      .with_line_number(true);

    // Create a stdout layer for systemd/journal capture
    let stdout_layer = tracing_subscriber::fmt::layer().with_ansi(false).with_target(true);

    // Initialize both layers
    tracing_subscriber::registry()
      .with(file_layer)
      .with(stdout_layer)
      .with(EnvFilter::new("debug"))
      .init();

    info!("Starting learnerd daemon");
    debug!("Using config: {:?}", self);

    info!("Daemon started successfully");
    self.run()
  }

  // TODO (autoparallel): this is actually never really able to be used at the moment.
  /// Attempts to stop a running daemon process.
  ///
  /// Sends SIGTERM to the process identified by PID file and performs cleanup.
  ///
  /// # Errors
  ///
  /// Returns `LearnerdErrors` if:
  /// - PID file is missing or invalid
  /// - Process termination fails
  /// - Cleanup fails
  ///
  /// # Note
  ///
  /// Currently has limited functionality - see TODO in implementation.
  pub fn stop(&self) -> Result<()> {
    if let Ok(pid) = fs::read_to_string(&self.pid_file) {
      let pid: i32 = pid.trim().parse().map_err(|e: std::num::ParseIntError| {
        LearnerdError::Daemon(format!("pid.trim().parse() gave error: {}", e))
      })?;

      #[cfg(unix)]
      {
        // Send SIGTERM to the process
        if let Err(e) = signal::kill(Pid::from_raw(pid), Signal::SIGTERM) {
          error!("Failed to send SIGTERM to process: {}", e);
          return Err(LearnerdError::Daemon(format!("Failed to stop daemon: {}", e)));
        }
      }

      if let Err(e) = fs::remove_file(&self.pid_file) {
        error!("Failed to remove PID file: {}", e);
      }

      Ok(())
    } else {
      error!("PID file not found");
      Err(LearnerdError::Daemon("Daemon not running".to_string()))
    }
  }

  // TODO (autoparallel): this is actually never really able to be used at the moment.
  /// Restarts the daemon process with a 1-second delay between stop and start.
  ///
  /// # Errors
  ///
  /// Returns `LearnerdErrors` if either stop or start operations fail.
  pub fn restart(&self) -> Result<()> {
    self.stop()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    self.start()
  }

  /// Installs the daemon as a system service using platform-specific mechanisms.
  ///
  /// # Platform-specific behavior
  ///
  /// - Linux: Creates systemd unit file
  /// - macOS: Creates launchd plist
  ///
  /// # Errors
  ///
  /// Returns `LearnerdErrors` if service installation fails.
  pub fn install(&self) -> Result<()> { install_system_daemon(self) }

  /// Removes the daemon from system services.
  ///
  /// # Errors
  ///
  /// Returns `LearnerdErrors` if service removal fails.
  pub fn uninstall(&self) -> Result<()> { uninstall_system_daemon() }

  /// Main daemon loop that handles background tasks.
  ///
  /// Currently implements a basic heartbeat for monitoring.
  fn run(&self) -> Result<()> {
    info!("Daemon running");

    // TODO: Implement actual daemon functionality
    loop {
      std::thread::sleep(std::time::Duration::from_secs(5));
      debug!("Daemon heartbeat");
    }
  }
}

#[cfg(test)]
mod tests {

  use tempfile::tempdir;

  use super::*;

  fn setup_test_daemon() -> (Daemon, tempfile::TempDir) {
    let test_dir = tempdir().expect("Failed to create temp directory");
    let daemon = Daemon {
      pid_file:    test_dir.path().join("test.pid"),
      working_dir: test_dir.path().join("work"),
      log_dir:     test_dir.path().join("logs"),
    };
    (daemon, test_dir)
  }

  #[test]
  fn test_daemon_directory_creation() {
    let (daemon, _temp) = setup_test_daemon();
    let daemon_clone = daemon.clone();
    // Start should create directories
    let _handle = std::thread::spawn(move || daemon.start());
    std::thread::sleep(std::time::Duration::from_secs(1));

    assert!(daemon_clone.working_dir.exists(), "Working directory should be created");
    assert!(daemon_clone.log_dir.exists(), "Log directory should be created");
  }
}
