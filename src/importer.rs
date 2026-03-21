use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{
    config::RuntimeConfig,
    mem9::{Mem9Client, StorePayload},
    redact::sanitize_mem9_content,
    state::{LockedSyncState, SyncState},
};

const STORE_DELAY_MS: u64 = 100;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct SyncStats {
    pub total: usize,
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
struct ImportItem {
    raw_content: String,
    content: String,
    tags: Vec<String>,
    source: String,
    legacy_sources: Vec<String>,
}

pub async fn sync_once(config: &RuntimeConfig) -> Result<SyncStats> {
    let api_key = config
        .api_key
        .clone()
        .unwrap_or_else(|| config.tenant_id.clone());
    let client = Mem9Client::new(config.api_url.clone(), api_key)?;
    let items = collect_import_entries(&config.codex_memories_dir)?;
    SyncState::migrate_default_state_file_if_needed(&config.state_path)?;
    let mut state = SyncState::load_locked(&config.state_path)?;
    if migrate_locked_state_fingerprints(&mut state, &items) {
        state.save()?;
    }
    let mut stats = SyncStats::default();

    for item in items {
        stats.total += 1;
        let canonical_fingerprint = canonical_fingerprint_for(&item);
        if state.contains(&canonical_fingerprint) {
            stats.skipped += 1;
            continue;
        }
        let store_result = client
            .store(&StorePayload {
                content: item.content.clone(),
                tags: item.tags.clone(),
                source: item.source.clone(),
            })
            .await;

        if let Err(error) = store_result {
            eprintln!("failed to store memory item: {error:#}");
            continue;
        }

        state.mark_imported(canonical_fingerprint);
        state.save()?;
        stats.imported += 1;
        tokio::time::sleep(Duration::from_millis(STORE_DELAY_MS)).await;
    }

    state.save()?;
    Ok(stats)
}

fn fingerprint_value(source: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(b"\n");
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonical_fingerprint_for(item: &ImportItem) -> String {
    fingerprint_value(&item.source, &item.raw_content)
}

fn legacy_fingerprint_candidates(item: &ImportItem) -> Vec<String> {
    let canonical_fingerprint = canonical_fingerprint_for(item);
    let mut fingerprints = Vec::new();
    let sanitized_fingerprint = fingerprint_value(&item.source, &item.content);
    if sanitized_fingerprint != canonical_fingerprint {
        fingerprints.push(sanitized_fingerprint);
    }
    for legacy_source in &item.legacy_sources {
        let legacy_fingerprint = fingerprint_value(legacy_source, &item.content);
        if legacy_fingerprint != canonical_fingerprint
            && !fingerprints.contains(&legacy_fingerprint)
        {
            fingerprints.push(legacy_fingerprint);
        }
    }
    fingerprints
}

#[cfg(test)]
fn migrate_state_fingerprints(state: &mut SyncState, items: &[ImportItem]) -> bool {
    let (updated_fingerprints, changed) =
        migrate_fingerprints(state.imported_fingerprints(), items);
    if changed {
        state.replace_imported_fingerprints(updated_fingerprints);
    }
    changed
}

fn migrate_locked_state_fingerprints(state: &mut LockedSyncState, items: &[ImportItem]) -> bool {
    let (updated_fingerprints, changed) =
        migrate_fingerprints(state.imported_fingerprints(), items);
    if changed {
        state.replace_imported_fingerprints(updated_fingerprints);
    }
    changed
}

fn migrate_fingerprints(
    existing_fingerprints: &IndexSet<String>,
    items: &[ImportItem],
) -> (IndexSet<String>, bool) {
    let mut alias_to_canonical = HashMap::<String, Vec<String>>::new();
    for item in items {
        let canonical_fingerprint = canonical_fingerprint_for(item);
        for alias in legacy_fingerprint_candidates(item) {
            alias_to_canonical
                .entry(alias)
                .or_default()
                .push(canonical_fingerprint.clone());
        }
    }

    let mut updated_fingerprints = IndexSet::new();
    let mut changed = false;

    for fingerprint in existing_fingerprints {
        match alias_to_canonical.get(fingerprint) {
            Some(canonicals) if canonicals.len() == 1 => {
                changed = true;
                updated_fingerprints.insert(canonicals[0].clone());
            }
            Some(_) => {
                changed = true;
            }
            None => {
                updated_fingerprints.insert(fingerprint.clone());
            }
        }
    }

    if !changed && updated_fingerprints != *existing_fingerprints {
        changed = true;
    }

    (updated_fingerprints, changed)
}

fn collect_import_entries(root: &Path) -> Result<Vec<ImportItem>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(vec![]);
    }

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.file_name().and_then(|v| v.to_str()) == Some("raw_memories.md") {
            continue;
        }
        if path.extension().and_then(|v| v.to_str()) == Some("md") {
            files.push(path.to_path_buf());
        }
    }

    files.sort();
    let mut items = Vec::new();
    for path in files {
        items.extend(collect_from_file(root, &path)?);
    }
    Ok(items)
}

fn collect_from_file(root: &Path, path: &Path) -> Result<Vec<ImportItem>> {
    let markdown = fs::read_to_string(path)
        .with_context(|| format!("failed to read memory file: {}", path.display()))?;
    let source = build_source(root, path);
    let legacy_source = build_legacy_source(path);
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let legacy_sources = if legacy_source == source {
        vec![]
    } else {
        vec![legacy_source]
    };
    let mut items = Vec::new();

    match file_name {
        "MEMORY.md" => {
            items.extend(collect_tagged_bullets(
                &markdown,
                "### learnings",
                "Learning",
                &["codex-memory", "learning"],
                &source,
                &legacy_sources,
            ));
        }
        "memory_summary.md" => {
            items.extend(collect_tagged_bullets(
                &markdown,
                "## User preferences",
                "User preference",
                &["codex-memory", "user-preference"],
                &source,
                &legacy_sources,
            ));
            items.extend(collect_tagged_bullets(
                &markdown,
                "## General Tips",
                "General tip",
                &["codex-memory", "general-tip"],
                &source,
                &legacy_sources,
            ));
        }
        _ => {
            items.extend(collect_tagged_bullets(
                &markdown,
                "### learnings",
                "Learning",
                &["codex-memory", "rollout-learning"],
                &source,
                &legacy_sources,
            ));
        }
    }

    Ok(items)
}

fn collect_tagged_bullets(
    markdown: &str,
    heading: &str,
    label: &str,
    tags: &[&str],
    source: &str,
    legacy_sources: &[String],
) -> Vec<ImportItem> {
    let mut items = Vec::new();
    let mut current_heading = String::new();
    let heading_level = heading.chars().take_while(|ch| *ch == '#').count();

    for raw in markdown.lines() {
        let line = raw.trim_end();
        let line_heading_level = line.chars().take_while(|ch| *ch == '#').count();
        if line_heading_level > 0 && line.as_bytes().get(line_heading_level) == Some(&b' ') {
            if line_heading_level == heading_level {
                current_heading = line.trim().to_string();
            } else if current_heading.eq_ignore_ascii_case(heading) {
                current_heading.clear();
            }
            continue;
        }
        if current_heading.eq_ignore_ascii_case(heading) && line.starts_with("- ") {
            let text = line.trim_start_matches("- ").trim();
            if !text.is_empty() {
                let raw_content = format!("{label}: {text}");
                items.push(ImportItem {
                    raw_content: raw_content.clone(),
                    content: sanitize_mem9_content(&raw_content),
                    tags: tags.iter().map(|tag| tag.to_string()).collect(),
                    source: source.to_string(),
                    legacy_sources: legacy_sources.to_vec(),
                });
            }
        }
    }

    items
}

fn build_source(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    format!("codex-memory:{relative}")
}

fn build_legacy_source(path: &Path) -> String {
    let mut parts = VecDeque::new();
    let mut cursor = Some(path);
    while let Some(current) = cursor {
        if let Some(name) = current.file_name().and_then(|v| v.to_str()) {
            parts.push_front(name.to_string());
            if name == "memories" {
                break;
            }
        }
        cursor = current.parent();
    }
    format!(
        "codex-memory:{}",
        parts.into_iter().collect::<Vec<_>>().join("/")
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use httpmock::prelude::*;
    use tempfile::tempdir;

    use crate::config::RuntimeConfig;

    use super::{
        ImportItem, build_legacy_source, build_source, canonical_fingerprint_for,
        collect_import_entries, collect_tagged_bullets, migrate_state_fingerprints, sync_once,
    };
    use crate::{redact::sanitize_mem9_content, state::SyncState};

    #[test]
    fn collects_high_signal_entries() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("memories");
        fs::create_dir_all(root.join("rollout_summaries")).unwrap();
        fs::write(
            root.join("MEMORY.md"),
            "### learnings\n- Re-check `~/.codex/sessions/x.jsonl`\n",
        )
        .unwrap();
        fs::write(root.join("memory_summary.md"), "## User preferences\n- Prefer concise Chinese output\n## General Tips\n- Check localhost:8081 first\n").unwrap();
        fs::write(
            root.join("rollout_summaries").join("one.md"),
            "### learnings\n- The service listens on 192.168.64.1:5000\n",
        )
        .unwrap();

        let items = collect_import_entries(&root).unwrap();
        assert_eq!(items.len(), 4);
        assert!(
            items
                .iter()
                .any(|item| item.content.contains("Codex session rollout JSONL file"))
        );
        assert!(
            items
                .iter()
                .any(|item| item.content.contains("loopback address"))
                || items
                    .iter()
                    .any(|item| item.content.contains("related address"))
        );
        assert!(
            items
                .iter()
                .any(|item| item.content.contains("private network address (port 5000)"))
        );
    }

    #[test]
    fn build_source_is_stable_for_the_same_path() {
        let root = Path::new("/tmp/memories");
        let path = Path::new("/tmp/memories/rollout_summaries/example.md");
        let first = build_source(root, path);
        let second = build_source(root, path);

        assert_eq!(first, second);
        assert_eq!(first, "codex-memory:rollout_summaries/example.md");
    }

    #[test]
    fn build_source_does_not_leak_absolute_paths_for_custom_root_names() {
        let root = Path::new("/tmp/custom-root-name");
        let path = Path::new("/tmp/custom-root-name/subdir/example.md");
        let source = build_source(root, path);

        assert_eq!(source, "codex-memory:subdir/example.md");
        assert!(!source.contains("/tmp/custom-root-name"));
    }

    #[test]
    fn build_legacy_source_matches_v012_format() {
        let path = Path::new("/tmp/memories/rollout_summaries/example.md");

        assert_eq!(
            build_legacy_source(path),
            "codex-memory:memories/rollout_summaries/example.md"
        );
    }

    #[test]
    fn migrates_unique_legacy_fingerprint_to_canonical_state_entry() {
        let source = "codex-memory:rollout_summaries/example.md".to_string();
        let legacy_source = "codex-memory:memories/rollout_summaries/example.md".to_string();
        let raw_content = "Learning: Read /Users/example/project/file-a.txt".to_string();
        let sanitized_content = sanitize_mem9_content(&raw_content);
        let item = ImportItem {
            raw_content: raw_content.clone(),
            content: sanitized_content.clone(),
            tags: vec![],
            source: source.clone(),
            legacy_sources: vec![legacy_source.clone()],
        };
        let mut state = SyncState::default();
        state.mark_imported(super::fingerprint_value(&legacy_source, &sanitized_content));

        assert!(migrate_state_fingerprints(&mut state, &[item]));
        assert!(state.contains(&canonical_fingerprint_for(&ImportItem {
            raw_content,
            content: sanitized_content,
            tags: vec![],
            source,
            legacy_sources: vec![legacy_source],
        })));
    }

    #[test]
    fn canonical_fingerprint_avoids_redaction_collision_for_distinct_items() {
        let source = "codex-memory:rollout_summaries/example.md".to_string();
        let first_raw = "Learning: Read /Users/example/project/file-a.txt".to_string();
        let second_raw = "Learning: Read /Users/example/project/file-b.txt".to_string();
        let first = ImportItem {
            raw_content: first_raw.clone(),
            content: sanitize_mem9_content(&first_raw),
            tags: vec![],
            source: source.clone(),
            legacy_sources: vec![],
        };
        let second = ImportItem {
            raw_content: second_raw.clone(),
            content: sanitize_mem9_content(&second_raw),
            tags: vec![],
            source,
            legacy_sources: vec![],
        };
        let mut state = SyncState::default();
        state.mark_imported(canonical_fingerprint_for(&first));

        assert_eq!(first.content, second.content);
        assert!(!state.contains(&canonical_fingerprint_for(&second)));
    }

    #[test]
    fn drops_ambiguous_legacy_fingerprint_aliases_to_avoid_collision() {
        let source = "codex-memory:rollout_summaries/example.md".to_string();
        let legacy_source = "codex-memory:memories/rollout_summaries/example.md".to_string();
        let first_raw = "Learning: Read /Users/example/project/file-a.txt".to_string();
        let second_raw = "Learning: Read /Users/example/project/file-b.txt".to_string();
        let first = ImportItem {
            raw_content: first_raw.clone(),
            content: sanitize_mem9_content(&first_raw),
            tags: vec![],
            source: source.clone(),
            legacy_sources: vec![legacy_source.clone()],
        };
        let second = ImportItem {
            raw_content: second_raw.clone(),
            content: sanitize_mem9_content(&second_raw),
            tags: vec![],
            source,
            legacy_sources: vec![legacy_source.clone()],
        };
        let mut state = SyncState::default();
        state.mark_imported(super::fingerprint_value(&legacy_source, &first.content));

        assert!(migrate_state_fingerprints(
            &mut state,
            &[first.clone(), second.clone()]
        ));
        assert!(!state.contains(&super::fingerprint_value(&legacy_source, &first.content)));
        assert!(!state.contains(&canonical_fingerprint_for(&first)));
        assert!(!state.contains(&canonical_fingerprint_for(&second)));
    }

    #[tokio::test]
    async fn sync_once_continues_when_store_request_fails() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1alpha2/mem9s/memories");
            then.status(500);
        });

        let dir = tempdir().unwrap();
        let memories_dir = dir.path().join("memories");
        fs::create_dir_all(&memories_dir).unwrap();
        fs::write(
            memories_dir.join("MEMORY.md"),
            "### learnings\n- One durable fact\n",
        )
        .unwrap();

        let config = RuntimeConfig {
            api_url: server.base_url(),
            api_key: Some("api-key".to_string()),
            tenant_id: "tenant".to_string(),
            codex_memories_dir: memories_dir,
            state_path: dir.path().join("state.json"),
            poll_interval_seconds: 1,
        };

        let stats = sync_once(&config).await.unwrap();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.imported, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sync_once_serializes_state_updates_across_tasks() {
        let server = MockServer::start();
        let store_mock = server.mock(|when, then| {
            when.method(POST).path("/v1alpha2/mem9s/memories");
            then.status(202)
                .header("content-type", "application/json")
                .body(r#"{"status":"accepted"}"#)
                .delay(std::time::Duration::from_millis(200));
        });

        let dir = tempdir().unwrap();
        let memories_dir = dir.path().join("memories");
        fs::create_dir_all(&memories_dir).unwrap();
        fs::write(
            memories_dir.join("MEMORY.md"),
            "### learnings\n- Shared fingerprint\n",
        )
        .unwrap();

        let config = RuntimeConfig {
            api_url: server.base_url(),
            api_key: Some("api-key".to_string()),
            tenant_id: "tenant".to_string(),
            codex_memories_dir: memories_dir,
            state_path: dir.path().join("state.json"),
            poll_interval_seconds: 1,
        };

        let first_config = config.clone();
        let second_config = config.clone();
        let first = tokio::spawn(async move { sync_once(&first_config).await });
        let second = tokio::spawn(async move { sync_once(&second_config).await });
        let first = first.await.unwrap().unwrap();
        let second = second.await.unwrap().unwrap();

        assert_eq!(first.imported + second.imported, 1);
        assert_eq!(first.total, 1);
        assert_eq!(second.total, 1);
        store_mock.assert_hits(1);
    }

    #[test]
    fn ignores_nested_headings_when_collecting_bullets() {
        let markdown = "## User preferences\n- Keep concise output\n### Nested note\n- Ignore this nested bullet\n## General Tips\n- Check health first\n";
        let items = collect_tagged_bullets(
            markdown,
            "## User preferences",
            "User preference",
            &["codex-memory", "user-preference"],
            "codex-memory:memories/memory_summary.md",
            &[],
        );

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("Keep concise output"));
    }
}
