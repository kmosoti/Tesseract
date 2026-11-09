use std::process::Command;

#[test]
fn cli_demo_restarts_and_replays_with_exclusive_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("events.jsonl");
    let binary = env!("CARGO_BIN_EXE_tesseract");
    let demo = Command::new(binary)
        .args(["demo", "--journal"])
        .arg(&journal)
        .output()
        .unwrap();
    assert!(
        demo.status.success(),
        "{}",
        String::from_utf8_lossy(&demo.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&demo.stdout).unwrap();
    assert_eq!(result["replay_matches"], true);
    assert!(result["processes"].as_u64().unwrap() > 0);
    let replay = Command::new(binary)
        .arg("replay")
        .arg(&journal)
        .arg("--checkpoint")
        .arg(journal.with_extension("checkpoint.json"))
        .output()
        .unwrap();
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let duplicate = Command::new(binary)
        .args(["demo", "--journal"])
        .arg(&journal)
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
}

#[test]
fn cli_rejects_unbounded_workers_before_starting_experiments() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tesseract"))
        .args(["compare", "--workers", "5", "--out"])
        .arg(dir.path().join("invalid.json"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("workers must be 1..=4"));
    assert!(!dir.path().join("invalid.json").exists());
}
