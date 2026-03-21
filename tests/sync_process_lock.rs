use std::{
    fs,
    process::{Command, Stdio},
    time::Duration,
};

use httpmock::prelude::*;
use tempfile::tempdir;

#[test]
fn sync_command_serializes_state_updates_across_processes() {
    let server = MockServer::start();
    let store_mock = server.mock(|when, then| {
        when.method(POST).path("/v1alpha2/mem9s/memories");
        then.status(202)
            .header("content-type", "application/json")
            .body(r#"{"status":"accepted"}"#)
            .delay(Duration::from_millis(200));
    });

    let dir = tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let memories_dir = dir.path().join("memories");
    let state_path = home_dir
        .join(".codex")
        .join("codex-mem9")
        .join("state.json");
    let codex_config_path = home_dir.join(".codex").join("config.toml");

    fs::create_dir_all(&memories_dir).unwrap();
    fs::create_dir_all(codex_config_path.parent().unwrap()).unwrap();
    fs::write(
        memories_dir.join("MEMORY.md"),
        "### learnings\n- Shared fingerprint\n",
    )
    .unwrap();
    fs::write(
        &codex_config_path,
        format!(
            "[codex_mem9]\ntenant_id = \"tenant\"\napi_url = \"{}\"\ncodex_memories_dir = \"{}\"\nstate_path = \"{}\"\n",
            server.base_url(),
            memories_dir.display(),
            state_path.display()
        ),
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_codex-mem9");
    let first = Command::new(binary)
        .arg("sync")
        .env("HOME", &home_dir)
        .env_remove("MEM9_TENANT_ID")
        .env_remove("MEM9_API_URL")
        .env_remove("MEM9_API_KEY")
        .env_remove("CODEX_MEMORIES_DIR")
        .env_remove("CODEX_MEM9_STATE_PATH")
        .env_remove("CODEX_MEM9_POLL_INTERVAL_SECONDS")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let second = Command::new(binary)
        .arg("sync")
        .env("HOME", &home_dir)
        .env_remove("MEM9_TENANT_ID")
        .env_remove("MEM9_API_URL")
        .env_remove("MEM9_API_KEY")
        .env_remove("CODEX_MEMORIES_DIR")
        .env_remove("CODEX_MEM9_STATE_PATH")
        .env_remove("CODEX_MEM9_POLL_INTERVAL_SECONDS")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let first_output = first.wait_with_output().unwrap();
    let second_output = second.wait_with_output().unwrap();

    assert!(
        first_output.status.success(),
        "{}",
        String::from_utf8_lossy(&first_output.stderr)
    );
    assert!(
        second_output.status.success(),
        "{}",
        String::from_utf8_lossy(&second_output.stderr)
    );
    store_mock.assert_hits(1);
}
