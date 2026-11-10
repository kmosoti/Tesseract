use std::fs;
use tesseract_contracts::digest;
use tesseract_knowledge::symbolic::{
    Atom, Claim, Concept, Element, Query, Relation, Scope, Verdict, language,
};
use tesseract_knowledge::{Article, Corpus, KnowledgeBase, LICENSE, Passage};
fn corpus(sentences: &[&str]) -> Corpus {
    Corpus {
        schema: 1,
        license: LICENSE.into(),
        articles: vec![Article {
            page_id: 1,
            title: "Source fixture".into(),
            revision: 2,
            revision_timestamp: "2026-11-09T00:00:00Z".into(),
            retrieved_at: "2026-11-10T00:00:00Z".into(),
            html_sha256: "0".repeat(64),
            equations: vec![],
            passages: sentences
                .iter()
                .map(|s| Passage {
                    section: "Fixture".into(),
                    text: (*s).into(),
                    equations: vec![],
                })
                .collect(),
        }],
    }
}
fn memory(sentences: &[&str]) -> KnowledgeBase {
    KnowledgeBase::read(corpus(sentences)).unwrap()
}
fn evaluate(
    memory: &KnowledgeBase,
    question: &str,
) -> tesseract_knowledge::symbolic::Understanding {
    memory
        .interpretation()
        .unwrap()
        .evaluate(&language::compile(question).unwrap())
        .unwrap()
}
#[test]
fn translation_has_no_answers_and_preserves_scope_and_polarity() {
    let question = "Can veloran alone be used for iridescent signalling?";
    let query = language::compile(question).unwrap();
    let Query::Verify { claim } = query.clone() else {
        panic!("not a proposition query")
    };
    assert!(claim.positive);
    assert!(matches!(
        claim.atom,
        Atom::Relation {
            scope: Scope::Alone,
            ..
        }
    ));
    let yes = memory(&["Veloran alone can be used for iridescent signalling."]);
    let no = memory(&["Veloran alone cannot be used for iridescent signalling."]);
    assert_eq!(query, language::compile(question).unwrap());
    assert_eq!(
        yes.interpretation()
            .unwrap()
            .evaluate(&query)
            .unwrap()
            .verdict,
        Verdict::Supported
    );
    assert_eq!(
        no.interpretation()
            .unwrap()
            .evaluate(&query)
            .unwrap()
            .verdict,
        Verdict::Refuted
    );
    assert_eq!(
        evaluate(&no, "Can veloran be used for iridescent signalling?").verdict,
        Verdict::Unknown
    );
    assert_eq!(
        evaluate(
            &no,
            "Is it false that veloran alone can be used for iridescent signalling?"
        )
        .verdict,
        Verdict::Supported
    );
    for question in [
        "Why does veloran cause collapse?",
        "Can veloran sometimes be used for iridescent signalling?",
        "Can veloran not be used for iridescent signalling?",
        "Is veloran not a phenomenon that causes collapse?",
    ] {
        let query = language::compile(question).unwrap();
        assert!(
            matches!(query, Query::Unsupported { .. }),
            "{question}: {query:?}"
        );
        assert_eq!(
            no.interpretation()
                .unwrap()
                .evaluate(&query)
                .unwrap()
                .verdict,
            Verdict::Unknown
        );
    }
}
#[test]
fn novel_source_atoms_conflict_and_missing_evidence_are_distinct() {
    let unknown = memory(&["Nacre is a mineral."]);
    let yes = memory(&["Veloran can be used for iridescent signalling."]);
    let no = memory(&["Veloran cannot be used for iridescent signalling."]);
    let both = memory(&[
        "Veloran can be used for iridescent signalling.",
        "Veloran cannot be used for iridescent signalling.",
    ]);
    for (model, expected) in [
        (&unknown, Verdict::Unknown),
        (&yes, Verdict::Supported),
        (&no, Verdict::Refuted),
        (&both, Verdict::Conflict),
    ] {
        assert_eq!(
            evaluate(model, "Can veloran be used for iridescent signalling?").verdict,
            expected
        );
    }
    let substituted = memory(&["Veloran can be used for ultraviolet imaging."]);
    assert_eq!(
        evaluate(
            &substituted,
            "Can veloran be used for iridescent signalling?"
        )
        .verdict,
        Verdict::Unknown
    );
    assert_eq!(
        evaluate(&substituted, "Can veloran be used for ultraviolet imaging?").verdict,
        Verdict::Supported
    );
    for model in [&yes, &no, &both, &substituted] {
        for fact in model.interpretation().unwrap().facts() {
            let witness = &fact.source;
            let raw = &model.corpus().articles[witness.article].passages[witness.passage].text;
            assert!(raw.get(witness.start..witness.end).is_some());
            assert_eq!(witness.revision, 2);
        }
    }
}
#[test]
fn qualifiers_and_subordinate_negation_are_not_silently_projected() {
    let model = memory(&[
        "Veloran is not a mineral that emits light.",
        "Veloran can be used for imaging only if cooled.",
        "Veloran may be used for imaging.",
        "Veloran is either blue or green.",
        "If veloran can produce light, but it cannot be used for imaging.",
        "Veloran is sometimes a mineral.",
    ]);
    assert!(model.interpretation().unwrap().facts().is_empty());
    let model =
        memory(&["Veloran can produce iridescent light, but it cannot be used for imaging."]);
    assert_eq!(
        evaluate(&model, "Can veloran be used for imaging?").verdict,
        Verdict::Refuted
    );
    let alone = memory(&[
        "Outcomes cannot be controlled, so veloran alone cannot be used to transmit information.",
    ]);
    assert_eq!(
        evaluate(&alone, "Can veloran alone be used to transmit information?").verdict,
        Verdict::Refuted
    );
    assert_eq!(
        evaluate(&alone, "Can veloran be used to transmit information?").verdict,
        Verdict::Unknown
    );
}
#[test]
fn acquired_linear_closure_enables_only_the_conditional_rule() {
    let source = "Iris principle is a postulate that states that linear combinations of solutions to the amber equation are also solutions of the amber equation.";
    let model = memory(&[source]);
    let question = "Under iris principle, if alpha and beta are solutions of the amber equation, is their linear combination also a solution?";
    let query = language::compile(question).unwrap();
    let result = model.interpretation().unwrap().evaluate(&query).unwrap();
    assert_eq!(result.verdict, Verdict::Supported);
    assert!(result.conditional);
    assert_eq!(result.proof[0].rule, "linear_combination_closure");
    assert_eq!(result.proof[0].premises.len(), 3);
    let empty = memory(&["Iris principle is a postulate."]);
    assert_eq!(
        empty
            .interpretation()
            .unwrap()
            .evaluate(&query)
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let changed = memory(&[
        "Iris principle is a postulate that states that linear combinations of solutions to the cobalt equation are also solutions of the cobalt equation.",
    ]);
    assert_eq!(
        changed
            .interpretation()
            .unwrap()
            .evaluate(&query)
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let mismatch = memory(&[
        "Iris principle is a postulate that states that linear combinations of solutions to the amber equation are also solutions of the cobalt equation.",
    ]);
    assert_eq!(
        mismatch
            .interpretation()
            .unwrap()
            .evaluate(&query)
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let Query::Conditional {
        mut assumptions,
        conclusion,
    } = query
    else {
        panic!("missing conditional")
    };
    assumptions.pop();
    let missing = Query::Conditional {
        assumptions: assumptions.clone(),
        conclusion: conclusion.clone(),
    };
    assert_eq!(
        model
            .interpretation()
            .unwrap()
            .evaluate(&missing)
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let mut negative = assumptions[0].clone();
    negative.positive = false;
    assumptions.push(negative);
    assert_eq!(
        model
            .interpretation()
            .unwrap()
            .evaluate(&Query::Conditional {
                assumptions,
                conclusion
            })
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
}
#[test]
fn no_arbitrary_relation_transitivity_and_aliases_need_source_evidence() {
    let model = memory(&["Veloran describes nacre.", "Nacre describes amber."]);
    let query = Query::Verify {
        claim: Claim {
            atom: Atom::Relation {
                subject: Concept::new("veloran").unwrap(),
                predicate: Relation::Describes,
                object: Concept::new("amber").unwrap(),
                scope: Scope::Unqualified,
            },
            positive: true,
            according_to: None,
        },
    };
    assert_eq!(
        model
            .interpretation()
            .unwrap()
            .evaluate(&query)
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let model = memory(&["Veloran is also known as iris.", "Veloran is a mineral."]);
    let result = evaluate(&model, "Is iris a mineral?");
    assert_eq!(result.verdict, Verdict::Supported);
    assert_eq!(result.proof[0].rule, "alias_rewrite");
    assert_eq!(result.proof[0].premises.len(), 2);
}
#[test]
fn persisted_interpretation_is_versioned_source_bound_and_immutable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.json");
    let model = memory(&["Veloran is the loss of nacre."]);
    model.save(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let result = evaluate(&model, "What is veloran?");
    assert_eq!(
        result,
        evaluate(&KnowledgeBase::open(&path).unwrap(), "What is veloran?")
    );
    assert_eq!(before, fs::read(&path).unwrap());
    let mut raw: serde_json::Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(raw["schema"], 2);
    raw["interpretation"]["facts"][0]["claim"]["positive"] = false.into();
    raw["interpretation_digest"] = digest(&raw["interpretation"]).unwrap().into();
    fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    assert!(KnowledgeBase::open(&path).is_err());
    raw = serde_json::from_slice(&before).unwrap();
    raw["interpretation"]["extractor"] = "future-9".into();
    fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    assert!(KnowledgeBase::open(&path).is_err());
    let legacy = serde_json::json!({"schema":1,"corpus_digest":digest(model.corpus()).unwrap(),"corpus":model.corpus()});
    fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let legacy = KnowledgeBase::open(&path).unwrap();
    assert!(legacy.interpretation().is_err());
    assert!(legacy.ask("What is veloran?").is_ok());
}
#[test]
fn renderer_only_formats_the_given_proof_and_scientific_graph_has_real_atoms() {
    let model = memory(&["Veloran is the loss of nacre."]);
    let mut result = evaluate(&model, "What is veloran?");
    let prose = language::render(&result);
    assert!(prose.contains("LossOf(veloran, nacre)"));
    assert!(!prose.contains("Veloran is the loss of nacre."));
    result.proof.clear();
    result.sources.clear();
    assert!(!language::render(&result).contains("LossOf"));
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../data/quantum-mechanics/corpus.json")).unwrap();
    let science = KnowledgeBase::read(corpus).unwrap();
    assert_eq!(
        evaluate(
            &science,
            "Can entanglement be used for faster-than-light communication?"
        )
        .verdict,
        Verdict::Refuted
    );
    assert_eq!(
        evaluate(
            &science,
            "Can entanglement alone be used to transmit information?"
        )
        .verdict,
        Verdict::Refuted
    );
    let result = evaluate(
        &science,
        "Under quantum superposition, if a and b are solutions of the Schrödinger equation, is their linear combination also a solution?",
    );
    assert_eq!(result.verdict, Verdict::Supported);
    assert!(result.conditional);
    assert!(matches!(
        result.proof[0].conclusion.atom,
        Atom::Solution {
            element: Element::LinearCombination { .. },
            ..
        }
    ));
}

#[test]
fn interrogative_sources_and_unresolved_pronouns_are_not_assertions() {
    let model = memory(&[
        "Veloran can produce light?",
        "It is a mineral.",
        "This phenomenon is a mineral.",
        "Scientists believed the conjecture, so veloran can produce light.",
    ]);
    assert!(model.interpretation().unwrap().facts().is_empty());
    assert_eq!(
        evaluate(&model, "Can veloran produce light?").verdict,
        Verdict::Unknown
    );
}
#[test]
fn description_conflicts_include_alias_equivalent_subjects_and_both_witnesses() {
    let model = memory(&[
        "Veloran is also known as iris.",
        "Veloran is a mineral.",
        "Iris is not a mineral.",
    ]);
    let result = evaluate(&model, "What is veloran?");
    assert_eq!(result.verdict, Verdict::Conflict);
    assert_eq!(
        evaluate(&model, "Is veloran a mineral?").verdict,
        Verdict::Conflict
    );
    assert!(result.proof.iter().any(|p| p.conclusion.positive));
    assert!(result.proof.iter().any(|p| !p.conclusion.positive));
}
#[test]
fn contested_alias_equivalence_blocks_rewriting_in_both_directions_and_longer_paths() {
    let reverse = memory(&[
        "Veloran is also known as iris.",
        "Iris is not also known as veloran.",
        "Veloran is a mineral.",
    ]);
    let result = evaluate(&reverse, "Is iris a mineral?");
    assert_eq!(result.verdict, Verdict::Unknown);
    assert!(result.reason.contains("conflicting evidence"));
    assert_eq!(result.sources.len(), 2);
    assert_eq!(
        evaluate(&reverse, "What is iris?").verdict,
        Verdict::Conflict
    );
    let cycle = memory(&[
        "Veloran is also known as iris.",
        "Iris is also known as nacre.",
        "Nacre is not also known as veloran.",
        "Nacre is a mineral.",
    ]);
    let result = evaluate(&cycle, "Is veloran a mineral?");
    assert_eq!(result.verdict, Verdict::Unknown);
    assert_eq!(result.sources.len(), 3);
}
#[test]
fn opposing_hypotheses_block_both_conclusions_and_required_source_premises() {
    let model = memory(&[
        "Iris principle is a postulate that states that linear combinations of solutions to the amber equation are also solutions of the amber equation.",
    ]);
    let Query::Conditional {assumptions,conclusion}=language::compile("Under iris principle, if a and b are solutions of the amber equation, is their linear combination also a solution?").unwrap() else {panic!("not conditional")};
    let mut opposed = conclusion.clone();
    opposed.positive = false;
    let mut premises = assumptions.clone();
    premises.push(opposed);
    assert_eq!(
        model
            .interpretation()
            .unwrap()
            .evaluate(&Query::Conditional {
                assumptions: premises,
                conclusion: conclusion.clone()
            })
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
    let mut closure = model
        .interpretation()
        .unwrap()
        .facts()
        .iter()
        .find(|f| matches!(f.claim.atom, Atom::ClosedUnderLinearCombination { .. }))
        .unwrap()
        .claim
        .clone();
    closure.positive = false;
    let mut premises = assumptions;
    premises.push(closure);
    assert_eq!(
        model
            .interpretation()
            .unwrap()
            .evaluate(&Query::Conditional {
                assumptions: premises,
                conclusion
            })
            .unwrap()
            .verdict,
        Verdict::Unknown
    );
}
#[test]
fn disconnected_alias_workloads_are_indexed_and_depth_bounds_are_reported() {
    let mut sentences = Vec::new();
    for i in 0..600 {
        sentences.push(format!("Veloran{i} is also known as iris{i}."));
        sentences.push(format!("Iris{i} is a mineral."));
    }
    let references = sentences.iter().map(String::as_str).collect::<Vec<_>>();
    let model = memory(&references);
    assert_eq!(
        evaluate(&model, "Is veloran599 a mineral?").verdict,
        Verdict::Supported
    );
    let mut chain = Vec::new();
    for i in 0..6 {
        chain.push(format!("Iris{i} is also known as iris{}.", i + 1));
    }
    chain.push("Iris6 is a mineral.".into());
    let model = memory(&chain.iter().map(String::as_str).collect::<Vec<_>>());
    let result = evaluate(&model, "Is iris0 a mineral?");
    assert_eq!(result.verdict, Verdict::Unknown);
    assert!(result.reason.contains("cutoff"));
}

#[test]
fn non_relation_queries_do_not_visit_unrelated_alias_components() {
    let mut sentences=vec!["Linear combinations of solutions to the amber equation are also solutions of the amber equation.".to_owned()];
    for i in 0..70 {
        sentences.push(format!("Unused is also known as endpoint{i}."));
    }
    let model = memory(&sentences.iter().map(String::as_str).collect::<Vec<_>>());
    let query = Query::Verify {
        claim: Claim {
            atom: Atom::ClosedUnderLinearCombination {
                equation: Concept::new("amber equation").unwrap(),
            },
            positive: true,
            according_to: None,
        },
    };
    let result = model.interpretation().unwrap().evaluate(&query).unwrap();
    assert_eq!(result.verdict, Verdict::Supported);
    assert_eq!(result.sources.len(), 1);
    assert!(!result.reason.contains("alias"));
}
