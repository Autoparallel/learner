//! Linux-specific daemon implementation using systemd.
//!
//! Provides functions for installing and managing the daemon as a Linux system service.
//! Uses systemd for service management and follows FHS conventions for daemon
//! directory structure.
//!
//! # Service Configuration
//!
//! The daemon is installed as a systemd service with:
//! - Network dependency management
//! - Automatic restart on failure
//! - Journal integration for logging
//! - Standard Linux directory paths

use super::*;

/// Default PID file location following FHS conventions
pub const DEFAULT_PID_FILE: &str = "/var/run/learnerd.pid";

/// Default working directory for daemon operations
pub const DEFAULT_WORKING_DIR: &str = "/var/lib/learnerd";

/// Default log directory following system log conventions
pub const DEFAULT_LOG_DIR: &str = "/var/log/learnerd";

/// Installs the daemon as a systemd service.
///
/// Creates a service unit file and installs the binary:
/// - Sets up service dependencies and metadata
/// - Configures process management and logging
/// - Installs binary to /usr/local/bin if running from cargo
/// - Reloads systemd configuration
///
/// # Errors
///
/// Returns `LearnerdErrors` if:
/// - Binary installation fails
/// - Service file creation fails
/// - Systemd reload fails
pub fn install_system_daemon(_daemon: &Daemon) -> Result<()> {
  let service = String::from(
    r#"[Unit]
Description=Academic Paper Management Daemon
After=network.target
Documentation=https://github.com/autoparallel/learner

[Service]
Type=simple
User=root
Group=root
ExecStart=/usr/local/bin/learnerd daemon start
Restart=on-failure
RestartSec=60
RemainAfterExit=yes

# Logging configuration
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
"#,
  );

  // Install the binary to /usr/local/bin if it's not there
  if let Ok(current_exe) = std::env::current_exe() {
    if current_exe.to_str().unwrap_or("").contains(".cargo") {
      std::process::Command::new("cp")
        .args([current_exe.to_str().unwrap(), "/usr/local/bin/learnerd"])
        .output()?;
      std::process::Command::new("chmod").args(["755", "/usr/local/bin/learnerd"]).output()?;
    }
  }

  fs::write("/etc/systemd/system/learnerd.service", service)?;

  // Reload systemd
  std::process::Command::new("systemctl").args(["daemon-reload"]).output()?;
  Ok(())
}

/// Removes the daemon service configuration.
///
/// # Errors
///
/// Returns `LearnerdErrors` if service file removal fails.
pub fn uninstall_system_daemon() -> Result<()> {
  Ok(fs::remove_file("/etc/systemd/system/learnerd.service")?)
}

/// Displays post-installation instructions and helpful commands.
///
/// Shows:
/// - Service activation sequence
/// - Debugging commands
/// - Log access instructions
/// - Important systemd paths
pub fn daemon_install_prompt(daemon: &Daemon) {
  println!("{} Daemon service installed", style(SUCCESS).green());

  println!("\n{} To activate the service:", style("Next steps").blue());
  println!("   1. Reload:   {}", style("sudo systemctl daemon-reload").yellow());
  println!("   2. Enable:   {}", style("sudo systemctl enable learnerd").yellow());
  println!("   3. Start:    {}", style("sudo systemctl start learnerd").yellow());
  println!("   4. Verify:   {}", style("sudo systemctl status learnerd").yellow());

  println!("\n{} Troubleshooting commands:", style("Debug").blue());
  println!("   View logs:     {}", style("sudo journalctl -u learnerd -f").yellow());
  println!(
    "   Check paths:   {}",
    style("sudo systemctl show learnerd -p ExecStart,PIDFile,RuntimeDirectory").yellow()
  );
  println!("   Check status:  {}", style("sudo systemctl status learnerd --no-pager -l").yellow());

  println!("\n{} Service paths:", style("Configuration").blue());
  println!("   Working dir: {}", style(daemon.working_dir.display()).yellow());
  println!("   PID file:    {}", style(daemon.pid_file.display()).yellow());
  println!("   Log dir:     {}", style(daemon.log_dir.display()).yellow());
}
