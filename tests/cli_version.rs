use std::process::Command;

#[test]
fn cli_supports_version_flag() {
    let binary = env!("CARGO_BIN_EXE_codex-mem9");
    let output = Command::new(binary).arg("--version").output().unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = format!("codex-mem9 v{}", env!("CARGO_PKG_VERSION"));
    assert_eq!(stdout.trim(), expected);
}
