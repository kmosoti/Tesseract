use tesseract_contracts::{Action, Budget, Error, Experience, Features, Goal, Query};
use tesseract_kernel::{CognitiveKernel, Kernel, Kind, ValidationPolicy};

#[test]
fn all_representations_transfer_without_query_mutation_and_reject_stale_proofs() {
    let data = tesseract_workshop::dataset(100, 0.0).unwrap();
    for kind in Kind::ALL {
        let mut k = Kernel::new(kind).unwrap();
        for e in &data.training {
            k.observe(e.clone()).unwrap();
        }
        let candidate = k
            .develop(
                Budget {
                    evaluations: 4,
                    max_processes: 64,
                },
                100,
            )
            .unwrap();
        assert!(candidate.fit_report().objective_evaluations <= 4);
        let proof = k
            .validate(candidate, &data.validation, ValidationPolicy::default())
            .unwrap();
        assert!(proof.report().relative_gain > 0.5);
        k.commit(proof).unwrap();
        assert!(!k.processes().is_empty());
        let before = k.state_digest().unwrap();
        let loss: f64 = data
            .evaluation
            .iter()
            .map(|e| k.predict(&e.query).unwrap().brier(e.outcome).unwrap())
            .sum::<f64>()
            / data.evaluation.len() as f64;
        assert!(loss < 0.3, "{kind:?}: {loss}");
        let proposals = k
            .propose(
                &Goal {
                    features: Features { porous: false },
                    initial: 0,
                    target: 1,
                },
                Budget {
                    evaluations: 6,
                    max_processes: 4,
                },
            )
            .unwrap();
        assert!(proposals.len() <= 4);
        assert_eq!(before, k.state_digest().unwrap());
        let c = k
            .develop(
                Budget {
                    evaluations: 4,
                    max_processes: 64,
                },
                100,
            )
            .unwrap();
        // A proof can be accepted at a zero-improvement policy for this stale-parent test.
        let proof = k
            .validate(
                c,
                &data.validation,
                ValidationPolicy {
                    min_relative_gain: 0.0,
                    max_retention_regression: 0.01,
                },
            )
            .unwrap();
        let mut later = data.training[0].clone();
        later.object = "new-intervening-object".into();
        k.observe(later).unwrap();
        assert!(matches!(k.commit(proof), Err(Error::Stale)));
    }
}

#[test]
fn staged_context_learning_retains_prior_competence_and_rollback_keeps_it() {
    let data = tesseract_workshop::dataset(3, 0.0).unwrap();
    let mut k = Kernel::new(Kind::Bayesian).unwrap();
    for e in data.training.iter().filter(|e| !e.query.features.porous) {
        k.observe(e.clone()).unwrap();
    }
    let validation: Vec<_> = data
        .validation
        .iter()
        .filter(|e| !e.query.features.porous)
        .cloned()
        .collect();
    let c = k.develop(Budget::default(), 3).unwrap();
    let p = k
        .validate(c, &validation, ValidationPolicy::default())
        .unwrap();
    let first = k.commit(p).unwrap();
    let q = data
        .evaluation
        .iter()
        .find(|e| !e.query.features.porous)
        .unwrap()
        .query
        .clone();
    let prior = k.predict(&q).unwrap().probabilities;
    for e in data.training.iter().filter(|e| e.query.features.porous) {
        k.observe(e.clone()).unwrap();
    }
    let validation: Vec<_> = data
        .validation
        .iter()
        .filter(|e| e.query.features.porous)
        .cloned()
        .collect();
    let c = k.develop(Budget::default(), 3).unwrap();
    let p = k
        .validate(c, &validation, ValidationPolicy::default())
        .unwrap();
    k.commit(p).unwrap();
    assert_eq!(k.predict(&q).unwrap().probabilities, prior);
    k.rollback(first).unwrap();
    assert_eq!(k.predict(&q).unwrap().probabilities, prior);
    let seen = k.symbols().len();
    assert!(seen > 0);
    assert!(k.rollback(99).is_err());
}

#[test]
fn contradictory_evidence_can_change_beliefs_without_corrupting_competence() {
    let data = tesseract_workshop::dataset(4, 0.0).unwrap();
    let mut k = Kernel::new(Kind::Bayesian).unwrap();
    for e in &data.training {
        k.observe(e.clone()).unwrap();
    }
    let c = k.develop(Budget::default(), 4).unwrap();
    let p = k
        .validate(c, &data.validation, ValidationPolicy::default())
        .unwrap();
    k.commit(p).unwrap();
    let mut contradictory = data.training[0].clone();
    let object = contradictory.object.clone();
    contradictory.outcome = (contradictory.outcome + 1) % 4;
    let original = k.predict(&contradictory.query).unwrap();
    for _ in 0..64 {
        k.observe(contradictory.clone()).unwrap();
    }
    assert_eq!(k.symbols()[&object].latest_outcome, contradictory.outcome);
    assert_eq!(k.predict(&contradictory.query).unwrap(), original);
    let candidate = k.develop(Budget::default(), 4).unwrap();
    assert!(matches!(
        k.validate(candidate, &data.validation, ValidationPolicy::default()),
        Err(Error::Rejected(_))
    ));
    let overlapping: Vec<_> = data.training.iter().take(4).cloned().collect();
    let candidate = k.develop(Budget::default(), 4).unwrap();
    assert!(
        k.validate(candidate, &overlapping, ValidationPolicy::default())
            .is_err()
    );
}

#[test]
fn malformed_input_and_budget_fail_without_state_change() {
    let mut k = Kernel::new(Kind::Bayesian).unwrap();
    let before = k.state_digest().unwrap();
    let bad = Experience {
        object: "bad".into(),
        query: Query {
            features: Features { porous: false },
            initial: 7,
            actions: vec![Action::Heat],
        },
        outcome: 0,
    };
    assert!(k.observe(bad).is_err());
    assert_eq!(before, k.state_digest().unwrap());
    assert!(
        k.develop(
            Budget {
                evaluations: usize::MAX,
                max_processes: 1
            },
            0
        )
        .is_err()
    );
    assert!(
        k.propose(
            &Goal {
                features: Features { porous: false },
                initial: 0,
                target: 9
            },
            Budget::default()
        )
        .is_err()
    );
}

#[test]
fn proofs_are_bound_to_kind_and_actual_parent_content() {
    let data = tesseract_workshop::dataset(5, 0.0).unwrap();
    let mut a = Kernel::new(Kind::Bayesian).unwrap();
    let mut real = Kernel::new(Kind::Real).unwrap();
    let mut b = Kernel::new(Kind::Bayesian).unwrap();
    for e in &data.training {
        a.observe(e.clone()).unwrap();
        b.observe(e.clone()).unwrap();
        real.observe(e.clone()).unwrap();
    }
    let candidate = a.develop(Budget::default(), 5).unwrap();
    let proof = a
        .validate(candidate, &data.validation, ValidationPolicy::default())
        .unwrap();
    assert!(matches!(real.commit(proof), Err(Error::Stale)));
    assert_eq!(real.revision(), 0);
    for (kernel, limit) in [(&mut a, 64), (&mut b, 8)] {
        let c = kernel
            .develop(
                Budget {
                    evaluations: 1,
                    max_processes: limit,
                },
                5,
            )
            .unwrap();
        let p = kernel
            .validate(c, &data.validation, ValidationPolicy::default())
            .unwrap();
        kernel.commit(p).unwrap();
    }
    assert_eq!(a.revision(), b.revision());
    assert_eq!(a.evidence_head(), b.evidence_head());
    let c = a.develop(Budget::default(), 5).unwrap();
    let p = a
        .validate(
            c,
            &data.validation,
            ValidationPolicy {
                min_relative_gain: 0.0,
                max_retention_regression: 0.01,
            },
        )
        .unwrap();
    assert!(matches!(b.commit(p), Err(Error::Stale)));
}

#[test]
fn forged_process_provenance_and_retention_contamination_are_rejected() {
    let data = tesseract_workshop::dataset(6, 0.0).unwrap();
    let mut k = Kernel::new(Kind::Bayesian).unwrap();
    for e in &data.training {
        k.observe(e.clone()).unwrap();
    }
    let candidate = k.develop(Budget::default(), 6).unwrap();
    let mut wire = serde_json::to_value(&candidate).unwrap();
    wire["processes"][0]["evidence"] = serde_json::json!(["fabricated-evidence"]);
    let forged = serde_json::from_value(wire).unwrap();
    assert!(
        k.validate(forged, &data.validation, ValidationPolicy::default())
            .is_err()
    );
    let proof = k
        .validate(candidate, &data.validation, ValidationPolicy::default())
        .unwrap();
    k.commit(proof).unwrap();
    let before = k.state_digest().unwrap();
    assert!(k.observe(data.validation[0].clone()).is_err());
    assert_eq!(k.state_digest().unwrap(), before);
    let invalid = tesseract_kernel::Model::Density {
        complex: false,
        channels: Vec::new(),
    };
    assert!(invalid.predict(&data.evaluation[0].query, 0).is_err());
}

#[test]
fn acquired_macros_causally_improve_tightly_budgeted_proposal_search() {
    let data = tesseract_workshop::dataset(8, 0.0).unwrap();
    let mut k = Kernel::new(Kind::Bayesian).unwrap();
    for e in &data.training {
        k.observe(e.clone()).unwrap();
    }
    let c = k.develop(Budget::default(), 8).unwrap();
    let p = k
        .validate(c, &data.validation, ValidationPolicy::default())
        .unwrap();
    k.commit(p).unwrap();
    let mut world = tesseract_workshop::Workshop::new(8, 0.0).unwrap();
    let mut learned = 0;
    let mut ablated = 0;
    for porous in [false, true] {
        for initial in 0..4 {
            for target in 0..4 {
                let goal = Goal {
                    features: Features { porous },
                    initial,
                    target,
                };
                let budget = Budget {
                    evaluations: 1,
                    max_processes: 1,
                };
                let a = k.propose(&goal, budget).unwrap();
                let b = k.propose_without_processes(&goal, budget).unwrap();
                learned += usize::from(
                    world
                        .interact(a[0].query.clone(), "learned")
                        .unwrap()
                        .outcome
                        == target,
                );
                ablated += usize::from(
                    world
                        .interact(b[0].query.clone(), "ablated")
                        .unwrap()
                        .outcome
                        == target,
                );
            }
        }
    }
    assert!(learned > ablated, "learned {learned}, ablated {ablated}");
}
