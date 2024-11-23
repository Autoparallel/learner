//! macOS-specific daemon implementation using launchd.
//!
//! Provides functions for installing and managing the daemon as a macOS system service.
//! Uses launchd for service management and follows Apple's guidelines for daemon
//! configuration and directory structure.
//!
//! # Service Configuration
//!
//! The daemon is installed as a system-level launchd service with:
//! - Automatic restart on crash
//! - 60-second throttle between restarts
//! - Structured logging to system directories
//! - Standard macOS directory paths

use super::*;

/// Default PID file location following macOS conventions
pub const DEFAULT_PID_FILE: &str = "/Library/Application Support/learnerd/learnerd.pid";

/// Default working directory for daemon operations
pub const DEFAULT_WORKING_DIR: &str = "/Library/Application Support/learnerd";

/// Default log directory following macOS system log conventions
pub const DEFAULT_LOG_DIR: &str = "/Library/Logs/learnerd";

/// Service identifier for launchd integration
pub const SERVICE_NAME: &str = "learnerd.daemon";

/// Property list filename for the launchd service definition
pub const SERVICE_FILE: &str = "learnerd.daemon.plist";

/// Installs the daemon as a launchd service.
///
/// Creates a property list file with appropriate configuration for the daemon:
/// - Service identification and metadata
/// - Executable path and arguments
/// - Working directory and log paths
/// - Restart and crash handling policies
///
/// # Errors
///
/// Returns `LearnerdErrors` if:
/// - Cannot determine current executable path
/// - Fails to write property list file
pub fn install_system_daemon(daemon: &Daemon) -> Result<()> {
  let plist = format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
        <string>start</string>
    </array>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>ThrottleInterval</key>
    <integer>60</integer>
    <key>WorkingDirectory</key>
    <string>{}</string>
    <key>StandardOutPath</key>
    <string>{}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{}/stderr.log</string>
</dict>
</plist>"#,
    SERVICE_NAME,
    std::env::current_exe()?.display(),
    daemon.working_dir.display(),
    daemon.log_dir.display(),
    daemon.log_dir.display(),
  );

  Ok(fs::write(format!("/Library/LaunchDaemons/{}", SERVICE_FILE), plist)?)
}

/// Removes the daemon service configuration.
///
/// # Errors
///
/// Returns `LearnerdErrors` if the property list file cannot be removed.
pub fn uninstall_system_daemon() -> Result<()> {
  Ok(fs::remove_file(format!("/Library/LaunchDaemons/{}", SERVICE_FILE))?)
}

/// Displays post-installation instructions and helpful commands.
///
/// Shows:
/// - Service activation steps
/// - Troubleshooting commands
/// - Service control operations
/// - Important file paths
pub fn daemon_install_prompt(daemon: &Daemon) {
  println!("{} Daemon service installed", style(SUCCESS_PREFIX).green());

  println!("\n{} To activate the service:", style("Next steps").blue());
  println!(
    "   1. Load:     {}",
    style(format!("sudo launchctl load /Library/LaunchDaemons/{}", SERVICE_FILE)).yellow()
  );
  println!("   2. Verify:   {}", style("sudo launchctl list | grep learnerd").yellow());

  println!("\n{} Troubleshooting commands:", style("Debug").blue());
  println!(
    "   View logs:     {}",
    style(format!("tail -f {}/stdout.log", daemon.log_dir.display())).yellow()
  );
  println!(
    "   Check status:  {}",
    style(format!("sudo launchctl print system/{}", SERVICE_NAME)).yellow()
  );
  println!("   List service:  {}", style("sudo launchctl list | grep learnerd").yellow());

  println!("\n{} Service management:", style("Control").blue());
  println!(
    "   Stop:          {}",
    style(format!("sudo launchctl bootout system/{}", SERVICE_NAME)).yellow()
  );
  println!(
    "   Start:         {}",
    style(format!("sudo launchctl bootstrap system /Library/LaunchDaemons/{}", SERVICE_FILE))
      .yellow()
  );
  println!(
    "   Restart:       {}",
    style(format!("sudo launchctl kickstart -k system/{}", SERVICE_NAME)).yellow()
  );

  println!("\n{} Service paths:", style("Configuration").blue());
  println!("   Working dir: {}", style(daemon.working_dir.display()).yellow());
  println!("   PID file:    {}", style(daemon.pid_file.display()).yellow());
  println!("   Log dir:     {}", style(daemon.log_dir.display()).yellow());
}
