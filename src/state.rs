use std::{fs, path::Path};

use anyhow::{Context, Result};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncState {
    pub imported_fingerprints: IndexSet<String>,
}

impl SyncState {
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
        let tmp_path = path.with_extension("tmp");
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
}

#[cfg(test)]
mod tests {
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
        assert!(!path.with_extension("tmp").exists());
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
}
