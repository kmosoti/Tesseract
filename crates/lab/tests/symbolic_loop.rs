//! Exploratory closed loop over typed public symbols, not a language benchmark.
use tesseract_contracts::{Budget, Error, Features, Goal};
use tesseract_journal::Session;
use tesseract_kernel::{CognitiveKernel, Kernel, Kind, ValidationPolicy};
use tesseract_workshop::{Workshop, dataset};

#[test]
fn complex_symbolic_loop_observes_and_resumes_without_prompts() {
    let data = dataset(100, 0.0).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("loop.jsonl");
    let mut session = Session::create(&journal, Kind::Complex).unwrap();
    for experience in &data.training {
        session.observe(experience.clone()).unwrap();
    }
    let candidate = session
        .kernel()
        .develop(
            Budget {
                evaluations: 4,
                max_processes: 64,
            },
            100,
        )
        .unwrap();
    let proof = session
        .kernel()
        .validate(candidate, &data.validation, ValidationPolicy::default())
        .unwrap();
    session.commit(proof).unwrap();
    let baseline = Kernel::new(Kind::Complex).unwrap();
    let mut world = Workshop::evaluation(100, 0.0).unwrap();
    let budget = Budget {
        evaluations: 39,
        max_processes: 1,
    };
    let mut different_from_untrained = 0;
    let mut observed = Vec::new();
    let mut revision = 1;
    for epoch in 0..2 {
        for initial in 0..4 {
            for target in 0..4 {
                let goal = Goal {
                    features: Features { porous: epoch == 1 },
                    initial,
                    target,
                };
                let before = session.kernel().state_digest().unwrap();
                let proposed = session.kernel().propose(&goal, budget).unwrap();
                let untrained = baseline.propose(&goal, budget).unwrap();
                assert_eq!(before, session.kernel().state_digest().unwrap());
                different_from_untrained += usize::from(proposed[0].query != untrained[0].query);
                let experience = world
                    .interact(proposed[0].query.clone(), "symbolic-loop")
                    .unwrap();
                observed.push((experience.object.clone(), experience.outcome));
                session.observe(experience).unwrap();
            }
        }
        let candidate = session
            .kernel()
            .develop(
                Budget {
                    evaluations: 4,
                    max_processes: 64,
                },
                101 + epoch,
            )
            .unwrap();
        assert_eq!(
            candidate.fit_report().training_examples,
            data.training.len() + observed.len()
        );
        match session
            .kernel()
            .validate(candidate, &data.validation, ValidationPolicy::default())
        {
            Ok(proof) => revision = session.commit(proof).unwrap(),
            Err(Error::Rejected(reason))
                if reason.starts_with("gain ")
                    || reason == "unrelated retained competence regressed"
                    || reason == "zero-loss parent cannot support a relative improvement claim" => {
            }
            Err(error) => panic!("unexpected validation failure: {error}"),
        }
        let state = session.kernel().state_digest().unwrap();
        drop(session);
        session = Session::open(&journal).unwrap();
        assert_eq!(session.kernel().state_digest().unwrap(), state);
        assert_eq!(session.kernel().kind(), Kind::Complex);
        assert_eq!(session.kernel().revision(), revision);
        for (object, outcome) in &observed {
            assert_eq!(session.kernel().symbols()[object].latest_outcome, *outcome);
        }
    }
    assert!(
        different_from_untrained > 0,
        "learned state never affected a proposal"
    );
}
