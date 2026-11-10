use std::fs;
use tesseract_contracts::digest;
use tesseract_knowledge::{AnswerStatus, Article, Corpus, KnowledgeBase, LICENSE, Passage};

fn corpus(text: &str) -> Corpus {
    Corpus {
        schema: 1,
        license: LICENSE.into(),
        articles: vec![Article {
            page_id: 1,
            title: "Quantum mechanics".into(),
            revision: 2,
            revision_timestamp: "2026-11-09T00:00:00Z".into(),
            retrieved_at: "2026-11-10T00:00:00Z".into(),
            html_sha256: "0".repeat(64),
            equations: vec![],
            passages: vec![Passage {
                section: "Introduction".into(),
                text: text.into(),
                equations: vec![],
            }],
        }],
    }
}
#[test]
fn answers_are_source_bound_persistent_and_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reading.json");
    let memory = KnowledgeBase::read(corpus(
        "Quantum mechanics describes matter and light using quantum states and probabilities.",
    ))
    .unwrap();
    memory.save(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let question = "What is quantum mechanics?";
    let answer = memory.ask(question).unwrap();
    assert_eq!(answer.status, AnswerStatus::EvidenceFound);
    for evidence in &answer.evidence {
        let source = &memory.corpus().articles[0];
        assert_eq!(source.passages[evidence.passage].text, evidence.excerpt);
        assert_eq!(
            digest(&source.passages[evidence.passage]).unwrap(),
            evidence.passage_digest
        );
        assert_eq!(source.url(), evidence.url);
    }
    assert!(memory.save(&path).is_err());
    let reopened = KnowledgeBase::open(&path).unwrap();
    assert_eq!(answer, reopened.ask(question).unwrap());
    assert_eq!(memory.stats(), reopened.stats());
    assert_eq!(before, fs::read(&path).unwrap());
}
#[test]
fn answer_content_comes_from_reading_not_a_question_template() {
    let question = "What is quantum mechanics?";
    let a = KnowledgeBase::read(corpus(
        "First source fixture: quantum mechanics describes a mathematical framework.",
    ))
    .unwrap();
    let b = KnowledgeBase::read(corpus(
        "Second source fixture: quantum mechanics predicts observation probabilities.",
    ))
    .unwrap();
    assert!(
        a.ask(question).unwrap().evidence[0]
            .excerpt
            .starts_with("First source")
    );
    assert!(
        b.ask(question).unwrap().evidence[0]
            .excerpt
            .starts_with("Second source")
    );
    assert_ne!(a.stats().corpus_digest, b.stats().corpus_digest);
    for query in [
        "How much will my electricity bill cost next month?",
        "What is a quantum banana recipe?",
        "What is the best recipe for banana bread?",
    ] {
        let answer = a.ask(query).unwrap();
        assert_eq!(answer.status, AnswerStatus::InsufficientEvidence, "{query}");
        assert!(answer.evidence.is_empty());
    }
}
#[test]
fn corruption_and_boundary_inputs_fail_explicitly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    let memory = KnowledgeBase::read(corpus(
        "Quantum mechanics predicts observation probabilities.",
    ))
    .unwrap();
    memory.save(&path).unwrap();
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    snapshot["corpus"]["articles"][0]["passages"][0]["text"] = "Altered source text".into();
    fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    assert!(KnowledgeBase::open(&path).is_err());
    for bad in [
        "".to_owned(),
        "a".repeat(2049),
        "Do\nthis".to_owned(),
        "hi\0there".to_owned(),
    ] {
        assert!(memory.ask(&bad).is_err());
    }
    assert!(memory.ask("What is ψ in quantum mechanics?").is_ok());
    let mut duplicate = corpus("Quantum mechanics.");
    duplicate.articles.push(duplicate.articles[0].clone());
    assert!(KnowledgeBase::read(duplicate).is_err());
    let mut oversized = corpus("Quantum mechanics.");
    oversized.articles[0].passages[0].text = "x".repeat(16_385);
    assert!(KnowledgeBase::read(oversized).is_err());
    let mut unknown = corpus("Quantum mechanics.");
    unknown.schema = 999;
    assert!(KnowledgeBase::read(unknown).is_err());
    let file = fs::File::create(&path).unwrap();
    file.set_len(tesseract_knowledge::MAX_BYTES + 1).unwrap();
    assert!(KnowledgeBase::open(&path).is_err());
}

#[test]
fn source_mathematics_is_cited_without_becoming_invented_algebra() {
    let mut source = corpus("Quantum mechanics uses a normalization condition [equation:0].");
    let math = "<math><mo>|</mo><mi>α</mi><msup><mo>|</mo><mn>2</mn></msup></math>";
    source.articles[0].equations.push(math.into());
    source.articles[0].passages[0].equations.push(0);
    let memory = KnowledgeBase::read(source.clone()).unwrap();
    let answer = memory.ask("What is quantum mechanics?").unwrap();
    assert_eq!(answer.evidence[0].equations[0].mathml, math);
    assert!(answer.to_english().contains("source MathML"));
    assert!(!answer.to_english().contains("(|)"));
    let mut duplicate = source.clone();
    duplicate.articles[0].passages[0].equations = vec![0, 0];
    assert!(KnowledgeBase::read(duplicate).is_err());
    let mut missing = source.clone();
    missing.articles[0].passages[0].equations.clear();
    assert!(KnowledgeBase::read(missing).is_err());
    source.articles[0].passages[0].equations = vec![1];
    assert!(KnowledgeBase::read(source).is_err());
}

#[test]
fn supplied_corpus_definitions_and_no_communication_evidence_are_retrievable() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../data/quantum-mechanics/corpus.json")).unwrap();
    let memory = KnowledgeBase::read(corpus).unwrap();
    for (question, topic) in [
        ("What is quantum mechanics?", "Quantum mechanics"),
        ("What is quantum superposition?", "Quantum superposition"),
        ("What is the Born rule?", "Born rule"),
    ] {
        let answer = memory.ask(question).unwrap();
        assert_eq!(answer.evidence[0].article, topic);
    }
    let answer = memory
        .ask("Can quantum entanglement send messages faster than light?")
        .unwrap();
    assert!(answer.evidence.iter().any(|e| {
        e.excerpt
            .contains("cannot be used for faster-than-light communication")
    }));
}

#[test]
fn vocabulary_scattered_across_unrelated_passages_is_not_answer_evidence() {
    let mut source =
        corpus("Apples appear in this independent source fixture for one unrelated topic.");
    for text in [
        "Planets appear in this independent source fixture for another unrelated topic.",
        "Minerals appear in this independent source fixture for a separate unrelated topic.",
        "Vehicles appear in this independent source fixture for a fourth unrelated topic.",
    ] {
        source.articles[0].passages.push(Passage {
            section: "Fixture".into(),
            text: text.into(),
            equations: vec![],
        });
    }
    let memory = KnowledgeBase::read(source).unwrap();
    let answer = memory
        .ask("What about apples planets minerals vehicles?")
        .unwrap();
    assert_eq!(answer.matched_term_fraction, 1.0);
    assert_eq!(answer.status, AnswerStatus::InsufficientEvidence);
    assert!(answer.evidence.is_empty());
}
