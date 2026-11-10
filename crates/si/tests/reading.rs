use std::{fs, process::Command};
use tesseract_knowledge::{Article, Corpus, KnowledgeBase, LICENSE, Passage};
use tesseract_si::Term;
fn fixture() -> Corpus {
    Corpus {schema:1,license:LICENSE.into(),articles:vec![Article {
        page_id:1,title:"Quantum mechanics".into(),revision:2,
        revision_timestamp:"2026-11-09T00:00:00Z".into(),retrieved_at:"2026-11-10T00:00:00Z".into(),html_sha256:"0".repeat(64),
        equations:vec![],
        passages:vec![Passage {section:"Introduction".into(),text:"Quantum mechanics describes matter and light with mathematical states and probabilities.".into(),equations:vec![]}],
    }]}
}
#[test]
fn recall_round_trip_preserves_open_english_and_enforces_context() {
    for question in [
        "What is quantum mechanics?",
        "What does \"ψ\" mean in a quantum state?",
        &"x".repeat(2048),
    ] {
        let term = Term::Recall(question.into());
        let source = term.to_source().unwrap();
        assert_eq!(
            Term::parse(&source).unwrap().to_english().unwrap(),
            question
        );
    }
    for source in [
        "recall not-json",
        "recall \"hello\" invent",
        "recall \"\"",
        "recall \"hello\\nworld\"",
    ] {
        assert!(Term::parse(source).is_err());
    }
    let memory = KnowledgeBase::read(fixture()).unwrap();
    assert!(
        Term::parse("concepts")
            .unwrap()
            .evaluate_reading(&memory)
            .is_err()
    );
    let kernel = tesseract_kernel::Kernel::new(tesseract_kernel::Kind::Complex).unwrap();
    let term = Term::Recall("What is quantum mechanics?".into());
    assert!(
        term.evaluate(&kernel, tesseract_contracts::Budget::default())
            .is_err()
    );
    assert_eq!(
        term.evaluate_reading(&memory).unwrap().answer.evidence[0].excerpt,
        memory.corpus().articles[0].passages[0].text
    );
}
#[test]
fn cli_reads_reopens_and_answers_without_modifying_memory() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = dir.path().join("corpus.json");
    let memory = dir.path().join("memory.json");
    fs::write(&corpus, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    let binary = env!("CARGO_BIN_EXE_tesseract-si");
    let read = Command::new(binary)
        .arg("read")
        .arg(&corpus)
        .arg(&memory)
        .output()
        .unwrap();
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    let before = fs::read(&memory).unwrap();
    let answer = Command::new(binary)
        .arg("ask-knowledge")
        .arg(&memory)
        .arg("What is quantum mechanics?")
        .output()
        .unwrap();
    assert!(answer.status.success());
    let english = String::from_utf8(answer.stdout).unwrap();
    assert!(english.contains("recall \"What is quantum mechanics?\""));
    assert!(english.contains("describes matter and light"));
    assert!(english.contains("oldid=2"));
    let unsupported = Command::new(binary)
        .arg("ask-knowledge")
        .arg(&memory)
        .arg("What is a quantum banana recipe?")
        .output()
        .unwrap();
    assert!(unsupported.status.success());
    assert!(
        String::from_utf8(unsupported.stdout)
            .unwrap()
            .contains("not have enough relevant evidence")
    );
    let symbolic = Command::new(binary)
        .arg("recall")
        .arg(&memory)
        .arg("recall \"What is quantum mechanics?\"")
        .output()
        .unwrap();
    assert!(symbolic.status.success());
    let thought: tesseract_si::ReadingThought = serde_json::from_slice(&symbolic.stdout).unwrap();
    assert_eq!(
        thought.answer.corpus_digest,
        KnowledgeBase::open(&memory).unwrap().stats().corpus_digest
    );
    assert_eq!(before, fs::read(&memory).unwrap());
    assert!(
        !Command::new(binary)
            .arg("read")
            .arg(&corpus)
            .arg(&memory)
            .status()
            .unwrap()
            .success()
    );
}
