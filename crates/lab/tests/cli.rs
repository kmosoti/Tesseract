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

#[test]
fn nucleus_exploration_is_bounded_persistent_and_exclusive() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("nucleus.jsonl");
    let binary = env!("CARGO_BIN_EXE_tesseract");
    let invalid = Command::new(binary)
        .args(["explore", "--journal"])
        .arg(&journal)
        .args(["--steps", "0"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!journal.exists());
    let run = Command::new(binary)
        .args(["explore", "--journal"])
        .arg(&journal)
        .args(["--steps", "4"])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(result["steps"], 4);
    assert_eq!(result["trace"].as_array().unwrap().len(), 4);
    assert_eq!(result["replay_matches"], true);
    let before = std::fs::read(&journal).unwrap();
    let duplicate = Command::new(binary)
        .args(["explore", "--journal"])
        .arg(&journal)
        .args(["--steps", "4"])
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert_eq!(before, std::fs::read(&journal).unwrap());
}
