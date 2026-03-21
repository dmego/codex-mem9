use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};

use codex_mem9::{load_runtime_config, run_sync, run_watch};

#[derive(Parser, Debug)]
#[command(name = "codex-mem9")]
#[command(about = "Sync and watch Codex memories into Mem9 with redaction")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Manually sync historical Codex memory markdown into Mem9.
    Sync,
    /// Automatically monitor Codex memory markdown and sync redacted updates.
    Watch,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_runtime_config()?;

    match cli.command {
        Command::Sync => {
            let stats = run_sync(&config).await?;
            println!(
                "synced total={} imported={} skipped={}",
                stats.total, stats.imported, stats.skipped
            );
        }
        Command::Watch => {
            run_watch(&config, Duration::from_secs(config.poll_interval_seconds)).await?;
        }
    }

    Ok(())
}
