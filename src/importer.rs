use std::{collections::VecDeque, fs, path::Path, time::Duration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{
    config::RuntimeConfig,
    mem9::{Mem9Client, StorePayload},
    redact::sanitize_mem9_content,
    state::SyncState,
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
    content: String,
    tags: Vec<String>,
    source: String,
}

pub async fn sync_once(config: &RuntimeConfig) -> Result<SyncStats> {
    let api_key = config
        .api_key
        .clone()
        .unwrap_or_else(|| config.tenant_id.clone());
    let client = Mem9Client::new(config.api_url.clone(), api_key)?;
    let mut state = SyncState::load(&config.state_path)?;
    let items = collect_import_entries(&config.codex_memories_dir)?;
    let mut stats = SyncStats::default();

    for item in items {
        stats.total += 1;
        let fingerprint = fingerprint_for(&item);
        if state.contains(&fingerprint) {
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

        state.mark_imported(fingerprint);
        state.save(&config.state_path)?;
        stats.imported += 1;
        tokio::time::sleep(Duration::from_millis(STORE_DELAY_MS)).await;
    }

    state.save(&config.state_path)?;
    Ok(stats)
}

fn fingerprint_for(item: &ImportItem) -> String {
    let mut hasher = Sha256::new();
    hasher.update(item.source.as_bytes());
    hasher.update(b"\n");
    hasher.update(item.content.as_bytes());
    format!("{:x}", hasher.finalize())
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
        items.extend(collect_from_file(&path)?);
    }
    Ok(items)
}

fn collect_from_file(path: &Path) -> Result<Vec<ImportItem>> {
    let markdown = fs::read_to_string(path)
        .with_context(|| format!("failed to read memory file: {}", path.display()))?;
    let source = build_source(path);
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let mut items = Vec::new();

    match file_name {
        "MEMORY.md" => {
            items.extend(collect_tagged_bullets(
                &markdown,
                "### learnings",
                "Learning",
                &["codex-memory", "learning"],
                &source,
            ));
        }
        "memory_summary.md" => {
            items.extend(collect_tagged_bullets(
                &markdown,
                "## User preferences",
                "User preference",
                &["codex-memory", "user-preference"],
                &source,
            ));
            items.extend(collect_tagged_bullets(
                &markdown,
                "## General Tips",
                "General tip",
                &["codex-memory", "general-tip"],
                &source,
            ));
        }
        _ => {
            items.extend(collect_tagged_bullets(
                &markdown,
                "### learnings",
                "Learning",
                &["codex-memory", "rollout-learning"],
                &source,
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
                items.push(ImportItem {
                    content: sanitize_mem9_content(&format!("{label}: {text}")),
                    tags: tags.iter().map(|tag| tag.to_string()).collect(),
                    source: source.to_string(),
                });
            }
        }
    }

    items
}

fn build_source(path: &Path) -> String {
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

    use super::{build_source, collect_import_entries, collect_tagged_bullets, sync_once};

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
        let path = Path::new("/tmp/memories/rollout_summaries/example.md");
        let first = build_source(path);
        let second = build_source(path);

        assert_eq!(first, second);
        assert_eq!(first, "codex-memory:memories/rollout_summaries/example.md");
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

    #[test]
    fn ignores_nested_headings_when_collecting_bullets() {
        let markdown = "## User preferences\n- Keep concise output\n### Nested note\n- Ignore this nested bullet\n## General Tips\n- Check health first\n";
        let items = collect_tagged_bullets(
            markdown,
            "## User preferences",
            "User preference",
            &["codex-memory", "user-preference"],
            "codex-memory:memories/memory_summary.md",
        );

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("Keep concise output"));
    }
}
