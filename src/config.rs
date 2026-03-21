use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
#[cfg(test)]
use once_cell::sync::Lazy;
use serde::{Deserialize, de::DeserializeOwned};

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

#[derive(Debug, Default, Deserialize)]
struct CodexConfig {
    #[serde(default)]
    codex_mem9: FileConfig,
}

pub fn load_runtime_config() -> Result<RuntimeConfig> {
    let home_dir = home_dir();
    let codex_config_path = env_var("CODEX_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".codex").join("config.toml"));
    let codex_config = load_optional_toml::<CodexConfig>(&codex_config_path, "Codex config file")?;
    let file_config = codex_config.codex_mem9;

    let tenant_id = env_var("MEM9_TENANT_ID")
        .or(file_config.tenant_id)
        .with_context(missing_tenant_id_message)?;

    let api_key = env_var("MEM9_API_KEY").or(file_config.api_key);

    let api_url = env_var("MEM9_API_URL")
        .or(file_config.api_url)
        .unwrap_or_else(|| DEFAULT_API_URL.to_string());

    let codex_memories_dir = env_var("CODEX_MEMORIES_DIR")
        .map(PathBuf::from)
        .or(file_config.codex_memories_dir)
        .unwrap_or_else(|| home_dir.join(".codex").join("memories"));

    let state_path = env_var("CODEX_MEM9_STATE_PATH")
        .map(PathBuf::from)
        .or(file_config.state_path)
        .unwrap_or_else(|| {
            home_dir
                .join(".codex")
                .join("codex-mem9")
                .join("state.json")
        });

    let poll_interval_seconds = env_var("CODEX_MEM9_POLL_INTERVAL_SECONDS")
        .and_then(|value| value.parse::<u64>().ok())
        .or(file_config.poll_interval_seconds)
        .filter(|value| *value > 0)
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

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn home_dir() -> PathBuf {
    env_var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}

fn load_optional_toml<T>(path: &Path, label: &str) -> Result<T>
where
    T: Default + DeserializeOwned,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read {label}: {}", path.display()))?;
    toml::from_str::<T>(&raw)
        .with_context(|| format!("failed to parse {label}: {}", path.display()))
}

fn missing_tenant_id_message() -> String {
    "missing MEM9_TENANT_ID; run the mem9-setup skill first. If you use brew services, add [codex_mem9].tenant_id to ~/.codex/config.toml. Interactive Codex sessions can also read MEM9_TENANT_ID from the default shell environment."
        .to_string()
}

#[cfg(test)]
static ENV_LOCK: Lazy<std::sync::Mutex<()>> = Lazy::new(|| std::sync::Mutex::new(()));

#[cfg(test)]
mod tests {
    use std::{env, fs, panic::AssertUnwindSafe};

    use tempfile::tempdir;

    use super::{DEFAULT_POLL_INTERVAL_SECONDS, load_runtime_config};

    fn with_env<F>(pairs: &[(&str, Option<&str>)], test: F)
    where
        F: FnOnce(),
    {
        let _guard = super::ENV_LOCK.lock().unwrap();
        let mut keys = vec![
            "HOME",
            "MEM9_TENANT_ID",
            "MEM9_API_URL",
            "MEM9_API_KEY",
            "CODEX_CONFIG_PATH",
            "CODEX_MEMORIES_DIR",
            "CODEX_MEM9_STATE_PATH",
            "CODEX_MEM9_POLL_INTERVAL_SECONDS",
        ];
        for (key, _) in pairs {
            if !keys.contains(key) {
                keys.push(key);
            }
        }

        let previous = keys
            .iter()
            .map(|key| ((*key).to_string(), env::var(key).ok()))
            .collect::<Vec<_>>();

        for (key, value) in pairs {
            unsafe {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }

        let result = std::panic::catch_unwind(AssertUnwindSafe(test));

        for (key, value) in previous {
            unsafe {
                match value {
                    Some(value) => env::set_var(&key, value),
                    None => env::remove_var(&key),
                }
            }
        }

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    #[test]
    fn reads_tenant_from_codex_config_when_environment_is_missing() {
        let home = tempdir().unwrap();
        with_env(
            &[
                ("HOME", Some(home.path().to_str().unwrap())),
                ("MEM9_TENANT_ID", None),
                ("MEM9_API_URL", None),
                ("MEM9_API_KEY", None),
                ("CODEX_CONFIG_PATH", None),
                ("CODEX_MEMORIES_DIR", None),
                ("CODEX_MEM9_STATE_PATH", None),
                ("CODEX_MEM9_POLL_INTERVAL_SECONDS", None),
            ],
            || {
                let codex_config_path = home.path().join(".codex").join("config.toml");
                fs::create_dir_all(codex_config_path.parent().unwrap()).unwrap();
                fs::write(
                    &codex_config_path,
                    "[codex_mem9]\ntenant_id = \"codex-tenant\"\napi_url = \"https://codex.example\"\n",
                )
                .unwrap();

                let config = load_runtime_config().unwrap();
                assert_eq!(config.tenant_id, "codex-tenant");
                assert_eq!(config.api_url, "https://codex.example");
            },
        );
    }

    #[test]
    fn prefers_environment_before_codex_config() {
        let home = tempdir().unwrap();
        with_env(
            &[
                ("HOME", Some(home.path().to_str().unwrap())),
                ("MEM9_TENANT_ID", Some("env-tenant")),
                ("MEM9_API_URL", Some("https://env.example")),
                ("MEM9_API_KEY", None),
                ("CODEX_CONFIG_PATH", None),
                ("CODEX_MEMORIES_DIR", None),
                ("CODEX_MEM9_STATE_PATH", None),
                ("CODEX_MEM9_POLL_INTERVAL_SECONDS", None),
            ],
            || {
                let codex_config_path = home.path().join(".codex").join("config.toml");
                fs::create_dir_all(codex_config_path.parent().unwrap()).unwrap();
                fs::write(
                    &codex_config_path,
                    "[codex_mem9]\ntenant_id = \"codex-tenant\"\napi_url = \"https://codex.example\"\n",
                )
                .unwrap();

                let config = load_runtime_config().unwrap();
                assert_eq!(config.tenant_id, "env-tenant");
                assert_eq!(config.api_url, "https://env.example");
            },
        );
    }

    #[test]
    fn missing_tenant_error_points_to_mem9_setup_and_codex_config() {
        let home = tempdir().unwrap();
        with_env(
            &[
                ("HOME", Some(home.path().to_str().unwrap())),
                ("MEM9_TENANT_ID", None),
                ("MEM9_API_URL", None),
                ("MEM9_API_KEY", None),
                ("CODEX_CONFIG_PATH", None),
                ("CODEX_MEMORIES_DIR", None),
                ("CODEX_MEM9_STATE_PATH", None),
                ("CODEX_MEM9_POLL_INTERVAL_SECONDS", None),
            ],
            || {
                let error = load_runtime_config().unwrap_err().to_string();
                assert!(error.contains("mem9-setup"));
                assert!(error.contains("~/.codex/config.toml"));
                assert!(error.contains("[codex_mem9].tenant_id"));
                assert!(error.contains("brew services"));
            },
        );
    }

    #[test]
    fn applies_shared_codex_config_to_optional_runtime_fields() {
        let home = tempdir().unwrap();
        with_env(
            &[
                ("HOME", Some(home.path().to_str().unwrap())),
                ("MEM9_TENANT_ID", None),
                ("MEM9_API_URL", None),
                ("MEM9_API_KEY", None),
                ("CODEX_CONFIG_PATH", None),
                ("CODEX_MEMORIES_DIR", None),
                ("CODEX_MEM9_STATE_PATH", None),
                ("CODEX_MEM9_POLL_INTERVAL_SECONDS", None),
            ],
            || {
                let codex_config_path = home.path().join(".codex").join("config.toml");
                let custom_memories = home.path().join("shared-memories");
                let custom_state = home.path().join("shared-state.json");
                fs::create_dir_all(codex_config_path.parent().unwrap()).unwrap();
                fs::write(
                    &codex_config_path,
                    format!(
                        "[codex_mem9]\ntenant_id = \"codex-tenant\"\napi_url = \"https://codex.example\"\ncodex_memories_dir = \"{}\"\nstate_path = \"{}\"\npoll_interval_seconds = 23\n",
                        custom_memories.display(),
                        custom_state.display()
                    ),
                )
                .unwrap();

                let config = load_runtime_config().unwrap();
                assert_eq!(config.codex_memories_dir, custom_memories);
                assert_eq!(config.state_path, custom_state);
                assert_eq!(config.poll_interval_seconds, 23);
            },
        );
    }

    #[test]
    fn zero_poll_interval_falls_back_to_default() {
        let home = tempdir().unwrap();
        with_env(
            &[
                ("HOME", Some(home.path().to_str().unwrap())),
                ("MEM9_TENANT_ID", None),
                ("MEM9_API_URL", None),
                ("MEM9_API_KEY", None),
                ("CODEX_CONFIG_PATH", None),
                ("CODEX_MEMORIES_DIR", None),
                ("CODEX_MEM9_STATE_PATH", None),
                ("CODEX_MEM9_POLL_INTERVAL_SECONDS", None),
            ],
            || {
                let codex_config_path = home.path().join(".codex").join("config.toml");
                fs::create_dir_all(codex_config_path.parent().unwrap()).unwrap();
                fs::write(
                    &codex_config_path,
                    "[codex_mem9]\ntenant_id = \"codex-tenant\"\npoll_interval_seconds = 0\n",
                )
                .unwrap();

                let config = load_runtime_config().unwrap();
                assert_eq!(config.poll_interval_seconds, DEFAULT_POLL_INTERVAL_SECONDS);
            },
        );
    }

    #[test]
    fn default_state_path_uses_codex_directory_without_personal_identifier() {
        let home = tempdir().unwrap();
        with_env(
            &[
                ("HOME", Some(home.path().to_str().unwrap())),
                ("MEM9_TENANT_ID", Some("tenant")),
                ("MEM9_API_URL", None),
                ("MEM9_API_KEY", None),
                ("CODEX_CONFIG_PATH", None),
                ("CODEX_MEMORIES_DIR", None),
                ("CODEX_MEM9_STATE_PATH", None),
                ("CODEX_MEM9_POLL_INTERVAL_SECONDS", None),
            ],
            || {
                let config = load_runtime_config().unwrap();
                let expected_state_path = home
                    .path()
                    .join(".codex")
                    .join("codex-mem9")
                    .join("state.json");

                assert_eq!(config.state_path, expected_state_path);
            },
        );
    }
}
