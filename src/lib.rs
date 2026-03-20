pub mod config;
pub mod importer;
pub mod mem9;
pub mod redact;
pub mod state;

use std::time::Duration;

use anyhow::Result;
use tokio::signal;

pub use config::{RuntimeConfig, load_runtime_config};
pub use importer::SyncStats;

pub async fn run_sync(config: &RuntimeConfig) -> Result<SyncStats> {
    importer::sync_once(config).await
}

pub async fn run_watch(config: &RuntimeConfig, interval: Duration) -> Result<()> {
    loop {
        if wait_for_next_cycle(interval, shutdown_signal()).await {
            break;
        }

        match importer::sync_once(config).await {
            Ok(stats) => {
                println!(
                    "watch tick total={} imported={} skipped={}",
                    stats.total, stats.imported, stats.skipped
                );
            }
            Err(error) => {
                eprintln!("watch tick failed: {error:#}");
            }
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = signal::ctrl_c().await;
    }
}

async fn wait_for_next_cycle<F>(interval: Duration, shutdown: F) -> bool
where
    F: std::future::Future<Output = ()>,
{
    tokio::select! {
        _ = shutdown => true,
        _ = tokio::time::sleep(interval) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::wait_for_next_cycle;

    #[tokio::test]
    async fn wait_for_next_cycle_stops_on_shutdown() {
        let should_stop = wait_for_next_cycle(Duration::from_secs(60), async {}).await;
        assert!(should_stop);
    }

    #[tokio::test]
    async fn wait_for_next_cycle_ticks_when_no_shutdown_arrives() {
        let should_stop =
            wait_for_next_cycle(Duration::from_millis(1), std::future::pending()).await;
        assert!(!should_stop);
    }
}
