use std::{
    env, fs,
    fs::File,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result};
use fs2::FileExt;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncState {
    pub imported_fingerprints: IndexSet<String>,
}

pub struct LockedSyncState {
    path: PathBuf,
    _lock_file: File,
    state: SyncState,
}

impl SyncState {
    pub fn migrate_default_state_file_if_needed(path: &Path) -> Result<()> {
        let Some(home_dir) = env::var_os("HOME").map(PathBuf::from) else {
            return Ok(());
        };
        if path != default_state_path(&home_dir) || path.exists() {
            return Ok(());
        }

        migrate_state_file_from_candidates_if_needed(path, &legacy_state_candidates(&home_dir))
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read state file: {}", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse state file: {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create state directory: {}", parent.display())
            })?;
        }

        let raw = serde_json::to_string_pretty(self)?;
        let tmp_path = path.with_extension(format!("tmp.{}", process::id()));
        fs::write(&tmp_path, format!("{raw}\n")).with_context(|| {
            format!(
                "failed to write temporary state file: {}",
                tmp_path.display()
            )
        })?;
        fs::rename(&tmp_path, path).with_context(|| {
            format!(
                "failed to replace state file atomically: {}",
                path.display()
            )
        })
    }

    pub fn mark_imported(&mut self, fingerprint: String) {
        self.imported_fingerprints.insert(fingerprint);
        while self.imported_fingerprints.len() > 50_000 {
            self.imported_fingerprints.shift_remove_index(0);
        }
    }

    pub fn contains(&self, fingerprint: &str) -> bool {
        self.imported_fingerprints.contains(fingerprint)
    }

    pub fn imported_fingerprints(&self) -> &IndexSet<String> {
        &self.imported_fingerprints
    }

    pub fn replace_imported_fingerprints(&mut self, imported_fingerprints: IndexSet<String>) {
        self.imported_fingerprints = imported_fingerprints;
    }

    pub fn load_locked(path: &Path) -> Result<LockedSyncState> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create state directory: {}", parent.display())
            })?;
        }

        let lock_path = path.with_extension("lock");
        let lock_file = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open state lock file: {}", lock_path.display()))?;
        lock_file
            .lock_exclusive()
            .with_context(|| format!("failed to lock state file: {}", lock_path.display()))?;

        let state = Self::load(path)?;
        Ok(LockedSyncState {
            path: path.to_path_buf(),
            _lock_file: lock_file,
            state,
        })
    }
}

fn default_state_path(home_dir: &Path) -> PathBuf {
    home_dir
        .join(".codex")
        .join("codex-mem9")
        .join("state.json")
}

fn legacy_state_candidates(home_dir: &Path) -> Vec<PathBuf> {
    vec![
        home_dir
            .join("Library")
            .join("Application Support")
            .join("ai.dmego.codex-mem9")
            .join("state.json"),
        home_dir
            .join(".local")
            .join("share")
            .join("codex-mem9")
            .join("state.json"),
    ]
}

fn migrate_state_file_from_candidates_if_needed(
    path: &Path,
    legacy_candidates: &[PathBuf],
) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    for legacy_path in legacy_candidates {
        if !legacy_path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create state directory: {}", parent.display())
            })?;
        }
        fs::copy(legacy_path, path).with_context(|| {
            format!(
                "failed to migrate state file from {} to {}",
                legacy_path.display(),
                path.display()
            )
        })?;
        return Ok(());
    }

    Ok(())
}

impl LockedSyncState {
    pub fn contains(&self, fingerprint: &str) -> bool {
        self.state.contains(fingerprint)
    }

    pub fn imported_fingerprints(&self) -> &IndexSet<String> {
        self.state.imported_fingerprints()
    }

    pub fn replace_imported_fingerprints(&mut self, imported_fingerprints: IndexSet<String>) {
        self.state
            .replace_imported_fingerprints(imported_fingerprints);
    }

    pub fn mark_imported(&mut self, fingerprint: String) {
        self.state.mark_imported(fingerprint);
    }

    pub fn save(&self) -> Result<()> {
        self.state.save(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, process};

    use tempfile::tempdir;

    use super::SyncState;

    #[test]
    fn save_and_load_round_trip_without_leaving_temp_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut state = SyncState::default();
        state.mark_imported("alpha".to_string());
        state.save(&path).unwrap();

        let loaded = SyncState::load(&path).unwrap();
        assert!(loaded.contains("alpha"));
        assert!(
            !path
                .with_extension(format!("tmp.{}", process::id()))
                .exists()
        );
    }

    #[test]
    fn evicts_oldest_fingerprint_when_capacity_is_exceeded() {
        let mut state = SyncState::default();
        for idx in 0..50_001 {
            state.mark_imported(format!("item-{idx}"));
        }

        assert!(!state.contains("item-0"));
        assert!(state.contains("item-50000"));
    }

    #[test]
    fn migrates_state_from_legacy_default_location() {
        let dir = tempdir().unwrap();
        let current_path = dir
            .path()
            .join(".codex")
            .join("codex-mem9")
            .join("state.json");
        let legacy_path = dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("ai.dmego.codex-mem9")
            .join("state.json");
        fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        fs::write(
            &legacy_path,
            "{\n  \"imported_fingerprints\": [\"legacy\"]\n}\n",
        )
        .unwrap();

        super::migrate_state_file_from_candidates_if_needed(&current_path, &[legacy_path]).unwrap();

        let migrated = SyncState::load(&current_path).unwrap();
        assert!(migrated.contains("legacy"));
    }
}
