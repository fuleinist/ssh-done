mod config;
mod daemon;
mod notify;
mod util;
mod wrap;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ssh-done",
    version,
    about = "Desktop notification when remote SSH commands finish",
    long_about = "ssh-done tells you when SSH sessions and wrapped commands finish.\n\
                  Tiny, local, no cloud, nothing installed on the remote host."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Send a test notification immediately
    Notify {
        /// Notification title
        title: String,
        /// Notification body
        body: String,
    },
    /// Run a command and notify when it finishes
    Wrap {
        /// Skip the notification if the command finishes faster than this (seconds)
        #[arg(long)]
        min_duration: Option<u64>,
        /// Command to run (put after `--`)
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Watch the process table for SSH sessions and notify when they end
    Daemon {
        /// Ignore sessions shorter than this (seconds); overrides config
        #[arg(long)]
        min_duration: Option<u64>,
        /// Process-table poll interval in seconds; overrides config
        #[arg(long)]
        poll_interval: Option<u64>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::Config::load(None);

    match cli.command {
        Command::Notify { title, body } => {
            notify::send(&title, &body, cfg.sound);
        }
        Command::Wrap {
            min_duration,
            command,
        } => {
            let code = wrap::run(&command, min_duration, cfg.sound)?;
            std::process::exit(code);
        }
        Command::Daemon {
            min_duration,
            poll_interval,
        } => {
            let min = min_duration.unwrap_or(cfg.min_duration_secs);
            let poll = poll_interval.unwrap_or(cfg.poll_interval_secs);
            daemon::run(min, poll, cfg.sound)?;
        }
    }
    Ok(())
}
