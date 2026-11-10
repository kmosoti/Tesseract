//! Learning from self-executed symbolic programs, with matched representation controls.
use tesseract_contracts::{Action, Budget, Error, Features, Query, sequences};
use tesseract_journal::Session;
use tesseract_kernel::{CognitiveKernel, Kind, ValidationPolicy};
use tesseract_si::{Term, Value, certify_maximum};
use tesseract_workshop::{Workshop, dataset};

fn run(kind: Kind, consolidate: bool) -> (f64, usize) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("si.jsonl");
    let mut session = Session::create(&path, kind).unwrap();
    let mut world = Workshop::new(101, 0.0).unwrap();
    let held_out = dataset(101, 0.0).unwrap();
    let budget = Budget {
        evaluations: 39,
        max_processes: 64,
    };
    let mut commits = 0;
    for epoch in 0..3 {
        // Fixed exploration prevents an uninformed greedy policy from starving actions.
        for porous in [false, true] {
            for initial in 0..4 {
                for action in Action::ALL {
                    let query = Query {
                        features: Features { porous },
                        initial,
                        actions: vec![action],
                    };
                    session
                        .observe(world.interact(query, "si-explore").unwrap())
                        .unwrap();
                }
            }
        }
        for porous in [false, true] {
            for initial in 0..4 {
                for target in 0..4 {
                    let feature = if porous { "porous" } else { "solid" };
                    let term = Term::parse(&format!("seek {feature} {initial} {target}")).unwrap();
                    let before = session.kernel().state_digest().unwrap();
                    let thought = term.evaluate(session.kernel(), budget).unwrap();
                    let Value::Program(program) = thought.value else {
                        panic!("seek must produce program")
                    };
                    assert_eq!(before, session.kernel().state_digest().unwrap());
                    if initial == 0 && target == 0 {
                        let alternatives = sequences()
                            .into_iter()
                            .map(|actions| {
                                session
                                    .kernel()
                                    .predict(&Query {
                                        features: Features { porous },
                                        initial,
                                        actions,
                                    })
                                    .unwrap()
                                    .probabilities[target as usize]
                            })
                            .collect::<Vec<_>>();
                        certify_maximum(program.expected_success, &alternatives).unwrap();
                    }
                    let public = world.interact(program.query, "si-feedback").unwrap();
                    session.observe(public).unwrap();
                }
            }
        }
        if consolidate {
            let candidate = session
                .kernel()
                .develop(
                    Budget {
                        evaluations: 4,
                        max_processes: 64,
                    },
                    200 + epoch,
                )
                .unwrap();
            match session.kernel().validate(
                candidate,
                &held_out.validation,
                ValidationPolicy::default(),
            ) {
                Ok(proof) => {
                    session.commit(proof).unwrap();
                    commits += 1;
                }
                Err(Error::Rejected(reason))
                    if reason.starts_with("gain ")
                        || reason == "unrelated retained competence regressed" => {}
                Err(error) => panic!("invalid learning cycle: {error}"),
            }
        }
        let before = session.kernel().state_digest().unwrap();
        drop(session);
        session = Session::open(&path).unwrap();
        assert_eq!(session.kernel().state_digest().unwrap(), before);
    }
    let loss = session.kernel().model().loss(&held_out.evaluation).unwrap();
    (loss, commits)
}
#[test]
fn si_learns_from_its_actions_and_retains_competence_across_rounds() {
    let (frozen, _) = run(Kind::Complex, false);
    let (complex, commits) = run(Kind::Complex, true);
    let (real, real_commits) = run(Kind::Real, true);
    println!(
        "SI diagnostic seed=101 epochs=3: frozen={frozen}, complex={complex} ({commits} commits), real={real} ({real_commits} commits)"
    );
    assert!(commits >= 2, "require repeated accepted consolidation");
    assert!(real_commits >= 2);
    assert!(
        complex < frozen * 0.5,
        "persistent learning must beat the frozen control"
    );
    assert!(real < frozen * 0.5);
}
