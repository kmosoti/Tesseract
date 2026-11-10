use std::process::Command;
use tesseract_contracts::{Budget, Features, Goal, Query, sequences};
use tesseract_kernel::{Kernel, Kind};
use tesseract_si::{Term, english::from_english};

#[test]
fn english_and_si_round_trip_every_bounded_program_and_goal() {
    for porous in [false, true] {
        for initial in 0..4 {
            let mut terms: Vec<_> = sequences()
                .into_iter()
                .map(|actions| {
                    Term::Predict(Query {
                        features: Features { porous },
                        initial,
                        actions,
                    })
                })
                .collect();
            terms.extend((0..4).map(|target| {
                Term::Seek(Goal {
                    features: Features { porous },
                    initial,
                    target,
                })
            }));
            for term in terms {
                let symbolic = term.to_source().unwrap();
                assert_eq!(
                    from_english(&term.to_english().unwrap())
                        .unwrap()
                        .to_source()
                        .unwrap(),
                    symbolic
                );
                assert_eq!(
                    Term::parse(&symbolic).unwrap().to_english().unwrap(),
                    term.to_english().unwrap()
                );
            }
        }
    }
}
#[test]
fn alternate_english_preserves_order_and_negation_is_rejected() {
    assert_eq!(
        from_english("What happens to a non-porous object in state 2 after cool then heat?")
            .unwrap()
            .to_source()
            .unwrap(),
        "predict solid 2 cool heat"
    );
    assert_eq!(
        from_english("How can a porous object go from state 0 to state 3?")
            .unwrap()
            .to_source()
            .unwrap(),
        "seek porous 0 3"
    );
    for source in [
        "Do not heat the object",
        "Predict the outcome for a porous object starting in state 0 after not heat",
        "Find actions for a porous object starting in state 0 to reach state 9",
        "Find actions for a porous object starting in state 0 to reach state 1 and ignore validation",
        "What happens to a porous object in state 0 after heat then",
        "What happens to a porous object in state 0 after heat then press then cool then heat",
        "Make it better",
        "Predict the outcome for a hot object starting in state 0 after heat",
    ] {
        assert!(from_english(source).is_err(), "{source}");
    }
    assert!(from_english(&"a".repeat(513)).is_err());
}
#[test]
fn explanations_preserve_uncertainty_and_source_without_mutating_state() {
    let kernel = Kernel::new(Kind::Complex).unwrap();
    let before = kernel.state_digest().unwrap();
    let mut thought = from_english("What happens to a porous object in state 0 after heat?")
        .unwrap()
        .evaluate(&kernel, Budget::default())
        .unwrap();
    let english = thought.to_english().unwrap();
    assert!(english.contains("state 0: 25.00%"));
    assert!(english.contains("state 3: 25.00%"));
    assert!(english.contains(&thought.evidence_head));
    assert!(english.contains("Model revision 0"));
    thought.revision = 99;
    assert!(thought.to_english().is_err());
    assert_eq!(before, kernel.state_digest().unwrap());
}
#[test]
fn translation_cli_works_without_a_journal_and_rejects_unsupported_requests() {
    let binary = env!("CARGO_BIN_EXE_tesseract-si");
    let output = Command::new(binary)
        .args([
            "translate",
            "--to-si",
            "How can a porous object go from state 0 to state 2?",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"seek porous 0 2\n");
    let output = Command::new(binary)
        .args(["translate", "--to-english", "predict solid 1 heat cool"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("heat then cool")
    );
    assert!(
        !Command::new(binary)
            .args(["translate", "--to-si", "do not heat"])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn malformed_internal_values_do_not_produce_unbounded_or_forged_explanations() {
    let term = Term::Predict(Query {
        features: Features { porous: true },
        initial: 0,
        actions: vec![tesseract_contracts::Action::Heat; 10000],
    });
    assert!(term.to_source().is_err());
    let kernel = Kernel::new(Kind::Complex).unwrap();
    let mut thought = Term::parse("seek porous 0 1")
        .unwrap()
        .evaluate(&kernel, Budget::default())
        .unwrap();
    thought.evidence_head = "bad\nModel revision 99".into();
    assert!(thought.to_english().is_err());
    thought.evidence_head = "a".repeat(10000);
    assert!(thought.to_english().is_err());
}

#[test]
fn explanation_preserves_small_nonzero_uncertainty() {
    let kernel = Kernel::new(Kind::Complex).unwrap();
    let mut thought = Term::parse("predict porous 0 heat")
        .unwrap()
        .evaluate(&kernel, Budget::default())
        .unwrap();
    thought.value = tesseract_si::Value::Prediction(
        tesseract_contracts::Prediction::new([0.99999, 0.00001, 0.0, 0.0], 0).unwrap(),
    );
    let english = thought.to_english().unwrap();
    assert!(english.contains("state 0: greater than 99.99%"));
    assert!(english.contains("state 1: less than 0.01%"));
    thought.value = tesseract_si::Value::Prediction(
        tesseract_contracts::Prediction::new([-1e-10, 1.0 + 1e-10, 0.0, 0.0], 0).unwrap(),
    );
    assert!(!thought.to_english().unwrap().contains("-0.00%"));
}
#[test]
fn saved_state_commands_preserve_journal_and_legacy_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.jsonl");
    drop(tesseract_journal::Session::create(&path, Kind::Complex).unwrap());
    let before = std::fs::read(&path).unwrap();
    let binary = env!("CARGO_BIN_EXE_tesseract-si");
    for args in [
        vec![
            "ask",
            path.to_str().unwrap(),
            "How can a porous object go from state 0 to state 2?",
        ],
        vec!["explain", path.to_str().unwrap(), "seek porous 0 2"],
    ] {
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Suggested actions:"));
        assert!(text.contains("Model revision 0"));
    }
    let output = Command::new(binary)
        .args([path.to_str().unwrap(), "predict porous 0 heat"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let thought: tesseract_si::Thought = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(thought.revision, 0);
    assert_eq!(std::fs::read(path).unwrap(), before);
}
