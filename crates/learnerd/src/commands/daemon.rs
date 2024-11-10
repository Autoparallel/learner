use super::*;

pub async fn daemon(cli: Cli, cmd: DaemonCommands) -> Result<()> {
  let daemon = daemon::Daemon::new();

  match cmd {
    DaemonCommands::Start => {
      println!("{} Starting daemon...", style(ROCKET).cyan());
      match daemon.start() {
        Ok(_) => println!("{} Daemon started successfully", style(SUCCESS).green()),
        Err(e) => {
          println!("{} Failed to start daemon: {}", style(WARNING).yellow(), style(&e).red());
          return Err(e);
        },
      }
    },
    DaemonCommands::Stop => {
      println!("{} Stopping daemon...", style(WARNING).yellow());
      match daemon.stop() {
        Ok(_) => println!("{} Daemon stopped", style(SUCCESS).green()),
        Err(e) => {
          println!("{} Failed to stop daemon: {}", style(WARNING).yellow(), style(&e).red());
          return Err(e);
        },
      }
    },
    DaemonCommands::Restart => {
      println!("{} Restarting daemon...", style(ROCKET).cyan());
      match daemon.restart() {
        Ok(_) => println!("{} Daemon restarted successfully", style(SUCCESS).green()),
        Err(e) => {
          println!("{} Failed to restart daemon: {}", style(WARNING).yellow(), style(&e).red());
          return Err(e);
        },
      }
    },
    DaemonCommands::Install => {
      println!("{} Installing daemon service...", style(ROCKET).cyan());
      match daemon.install() {
        Ok(_) => {
          println!("{} Daemon service installed", style(SUCCESS).green());
          daemon_install_prompt(&daemon);
        },
        Err(e) => {
          println!("{} Failed to install daemon: {}", style(WARNING).yellow(), style(&e).red());
          return Err(e);
        },
      }
    },
    DaemonCommands::Uninstall => {
      println!("{} Removing daemon service...", style(WARNING).yellow());
      match daemon.uninstall() {
        Ok(_) => {
          println!("{} Daemon service removed", style(SUCCESS).green());

          #[cfg(target_os = "linux")]
          println!(
            "\n{} Run {} to apply changes",
            style("Next step:").blue(),
            style("sudo systemctl daemon-reload").yellow()
          );
        },
        Err(e) => {
          println!("{} Failed to uninstall daemon: {}", style(WARNING).yellow(), style(&e).red());
          return Err(e);
        },
      }
    },
    DaemonCommands::Status => {
      if let Ok(pid) = std::fs::read_to_string(&daemon.pid_file) {
        let pid = pid.trim();
        println!("{} Daemon is running with PID: {}", style(SUCCESS).green(), style(pid).yellow());

        // Show log file location
        println!("\n{} Log files:", style("📄").cyan());
        println!("   Main log: {}", style(daemon.log_dir.join("learnerd.log").display()).yellow());
        println!("   Stdout: {}", style(daemon.log_dir.join("stdout.log").display()).yellow());
        println!("   Stderr: {}", style(daemon.log_dir.join("stderr.log").display()).yellow());

        // Show service status if installed
        #[cfg(target_os = "linux")]
        println!(
          "\n{} For detailed status, run: {}",
          style("Tip:").blue(),
          style("sudo systemctl status learnerd").yellow()
        );

        #[cfg(target_os = "macos")]
        println!(
          "\n{} For detailed status, run: {}",
          style("Tip:").blue(),
          style("sudo launchctl list | grep learnerd").yellow()
        );
      } else {
        println!("{} Daemon is not running", style(WARNING).yellow());
      }
    },
  }
  Ok(())
}
