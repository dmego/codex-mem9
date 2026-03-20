use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;

const DEFAULT_API_URL: &str = "https://api.mem9.ai";
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 15;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub api_url: String,
    pub api_key: Option<String>,
    pub tenant_id: String,
    pub codex_memories_dir: PathBuf,
    pub state_path: PathBuf,
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    api_url: Option<String>,
    api_key: Option<String>,
    tenant_id: Option<String>,
    codex_memories_dir: Option<PathBuf>,
    state_path: Option<PathBuf>,
    poll_interval_seconds: Option<u64>,
}

pub fn load_runtime_config() -> Result<RuntimeConfig> {
    let project_dirs = ProjectDirs::from("ai", "dmego", "code-mem9")
        .context("failed to locate the local configuration directory for code-mem9")?;
    let default_config_path = project_dirs.config_dir().join("config.toml");
    let default_state_path = project_dirs.data_dir().join("state.json");

    let file_config = if default_config_path.exists() {
        let raw = fs::read_to_string(&default_config_path).with_context(|| {
            format!(
                "failed to read config file: {}",
                default_config_path.display()
            )
        })?;
        toml::from_str::<FileConfig>(&raw).with_context(|| {
            format!(
                "failed to parse config file: {}",
                default_config_path.display()
            )
        })?
    } else {
        FileConfig::default()
    };

    let tenant_id = env::var("MEM9_TENANT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or(file_config.tenant_id)
        .context(
            "missing MEM9_TENANT_ID; provide it through the local environment or config file",
        )?;

    let api_key = env::var("MEM9_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or(file_config.api_key);

    let api_url = env::var("MEM9_API_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or(file_config.api_url)
        .unwrap_or_else(|| DEFAULT_API_URL.to_string());

    let codex_memories_dir = env::var("CODEX_MEMORIES_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or(file_config.codex_memories_dir)
        .unwrap_or_else(|| {
            PathBuf::from(env::var("HOME").unwrap_or_else(|_| "~".to_string()))
                .join(".codex")
                .join("memories")
        });

    let state_path = env::var("CODEX_MEM9_STATE_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or(file_config.state_path)
        .unwrap_or(default_state_path);

    let poll_interval_seconds = env::var("CODEX_MEM9_POLL_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .or(file_config.poll_interval_seconds)
        .unwrap_or(DEFAULT_POLL_INTERVAL_SECONDS);

    Ok(RuntimeConfig {
        api_url,
        api_key,
        tenant_id,
        codex_memories_dir,
        state_path,
        poll_interval_seconds,
    })
}
