//! Deterministic English adapter for the bounded SI vocabulary.
use crate::{Term, Thought, Value};
use tesseract_contracts::{Action, Error, Result};

fn invalid() -> Error {
    Error::Invalid("unsupported English; use 'Predict the outcome for a porous object starting in state 0 after heat then press' or 'Find actions for a nonporous object starting in state 1 to reach state 3'".into())
}
fn feature(word: &str) -> Result<&'static str> {
    match word {
        "porous" => Ok("porous"),
        "nonporous" | "solid" => Ok("solid"),
        _ => Err(invalid()),
    }
}
fn actions(words: &[&str]) -> Result<String> {
    let mut result = Vec::new();
    for (i, word) in words.iter().enumerate() {
        if i % 2 == 1 {
            if *word != "then" {
                return Err(invalid());
            }
        } else if ["heat", "press", "cool"].contains(word) {
            result.push(*word);
        } else {
            return Err(invalid());
        }
    }
    if words.len().is_multiple_of(2) || result.len() > 3 {
        return Err(invalid());
    }
    Ok(result.join(" "))
}
/// Translate supported English into validated SI. Unknown words are never dropped.
pub fn from_english(source: &str) -> Result<Term> {
    if source.len() > 512 {
        return Err(Error::Budget("English input exceeds 512 bytes".into()));
    }
    if !source.is_ascii() {
        return Err(invalid());
    }
    let normalized = source
        .trim()
        .trim_end_matches(['.', '?'])
        .to_ascii_lowercase()
        .replace("non-porous", "nonporous");
    let words: Vec<_> = normalized.split_whitespace().collect();
    let symbolic = match words.as_slice() {
        ["what", "concepts", "have", "you", "formed"] => "concepts".into(),
        ["what", "should", "you", "investigate", "next"] => "wonder".into(),
        [
            "predict",
            "the",
            "outcome",
            "for",
            "a",
            kind,
            "object",
            "starting",
            "in",
            "state",
            initial,
            "after",
            rest @ ..,
        ] => format!("predict {} {initial} {}", feature(kind)?, actions(rest)?),
        [
            "what",
            "happens",
            "to",
            "a",
            kind,
            "object",
            "in",
            "state",
            initial,
            "after",
            rest @ ..,
        ] => format!("predict {} {initial} {}", feature(kind)?, actions(rest)?),
        [
            "find",
            "actions",
            "for",
            "a",
            kind,
            "object",
            "starting",
            "in",
            "state",
            initial,
            "to",
            "reach",
            "state",
            target,
        ] => format!("seek {} {initial} {target}", feature(kind)?),
        [
            "how",
            "can",
            "a",
            kind,
            "object",
            "go",
            "from",
            "state",
            initial,
            "to",
            "state",
            target,
        ] => format!("seek {} {initial} {target}", feature(kind)?),
        _ => return Err(invalid()),
    };
    Term::parse(&symbolic)
}
fn action_name(action: Action) -> &'static str {
    match action {
        Action::Heat => "heat",
        Action::Press => "press",
        Action::Cool => "cool",
    }
}
fn percentage(probability: f64) -> String {
    let p = probability.clamp(0.0, 1.0);
    if p == 0.0 {
        "0.00%".into()
    } else if p < 0.0001 {
        "less than 0.01%".into()
    } else if p < 1.0 && p > 0.9999 {
        "greater than 99.99%".into()
    } else {
        format!("{:.2}%", p * 100.0)
    }
}
impl Term {
    /// Canonical SI source, validated even for externally constructed terms.
    pub fn to_source(&self) -> Result<String> {
        if let Self::Predict(query) = self {
            query.validate()?;
        }
        let source = match self {
            Self::Reason(query) => {
                query.validate()?;
                format!("reason {}", serde_json::to_string(query)?)
            }
            Self::Recall(question) => {
                tesseract_knowledge::validate_question(question)?;
                format!("recall {}", serde_json::to_string(question)?)
            }
            Self::Concepts => "concepts".into(),
            Self::Wonder => "wonder".into(),
            Self::Predict(query) => format!(
                "predict {} {} {}",
                if query.features.porous {
                    "porous"
                } else {
                    "solid"
                },
                query.initial,
                query
                    .actions
                    .iter()
                    .map(|a| action_name(*a))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Seek(goal) => format!(
                "seek {} {} {}",
                if goal.features.porous {
                    "porous"
                } else {
                    "solid"
                },
                goal.initial,
                goal.target
            ),
        };
        Term::parse(&source)?;
        Ok(source)
    }
    pub fn to_english(&self) -> Result<String> {
        self.to_source()?;
        Ok(match self {
            Self::Reason(query) => tesseract_knowledge::symbolic::language::display_query(query),
            Self::Recall(question) => question.clone(),
            Self::Concepts => "What concepts have you formed?".into(),
            Self::Wonder => "What should you investigate next?".into(),
            Self::Predict(query) => format!(
                "Predict the outcome for a {} object starting in state {} after {}.",
                if query.features.porous {
                    "porous"
                } else {
                    "nonporous"
                },
                query.initial,
                query
                    .actions
                    .iter()
                    .map(|a| action_name(*a))
                    .collect::<Vec<_>>()
                    .join(" then ")
            ),
            Self::Seek(goal) => format!(
                "Find actions for a {} object starting in state {} to reach state {}.",
                if goal.features.porous {
                    "porous"
                } else {
                    "nonporous"
                },
                goal.initial,
                goal.target
            ),
        })
    }
}
impl Thought {
    /// Reports model estimates as estimates and retains evidence/revision references.
    pub fn to_english(&self) -> Result<String> {
        if self.evidence_head.len() != 64
            || !self.evidence_head.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Invalid("invalid thought evidence head".into()));
        }
        let request = self.term.to_english()?;
        let result = match (&self.term, &self.value) {
            (Term::Concepts, Value::Concepts(concepts)) => {
                if concepts.len() > 16 {
                    return Err(Error::Budget("too many displayed concepts".into()));
                }
                let mut lines = Vec::new();
                for concept in concepts {
                    concept.validate()?;
                    let feature = match concept.pattern.porous {
                        Some(true) => "porous",
                        Some(false) => "nonporous",
                        None => "either porous or nonporous",
                    };
                    let initial = concept.pattern.initial.map_or_else(
                        || "any initial state".into(),
                        |i| format!("initial state {i}"),
                    );
                    let status = match concept.status {
                        tesseract_nucleus::ConceptStatus::Candidate => "candidate",
                        tesseract_nucleus::ConceptStatus::SupportedWorking => {
                            "supported working hypothesis"
                        }
                        tesseract_nucleus::ConceptStatus::Contested => "contested",
                    };
                    lines.push(format!("{}: For an object that is {feature}, at {initial}, {} predicts state {}. Support: {} objects; counterexamples: {} objects; {status}.", &concept.id[..12],concept.pattern.actions.iter().map(|a|action_name(*a)).collect::<Vec<_>>().join(" then "),concept.outcome,concept.support_objects,concept.counterexample_objects));
                }
                if lines.is_empty() {
                    "No working concepts yet; observations are needed.".into()
                } else {
                    format!(
                        "Working hypotheses, not validated laws:\n{}",
                        lines.join("\n")
                    )
                }
            }
            (Term::Wonder, Value::Curiosity(choice)) => {
                choice.validate()?;
                if choice.evidence_head != self.evidence_head || choice.revision != self.revision {
                    return Err(Error::Invalid("curiosity source mismatch".into()));
                }
                let reason = match choice.reason {
                    tesseract_nucleus::CuriosityReason::CommitteeDisagreement => {
                        "competing predictions disagree"
                    }
                    tesseract_nucleus::CuriosityReason::CoverageFallback => {
                        "coverage fallback because no disagreement was detected"
                    }
                };
                format!(
                    "Suggested experiment: {}\nWeighted predictive disagreement: {:.6e} nats; {reason}; searched {} of 312 queries. This proposes a test, not a verified outcome.",
                    Term::Predict(choice.query.clone()).to_english()?,
                    choice.disagreement,
                    choice.evaluated
                )
            }
            (Term::Predict(_), Value::Prediction(prediction)) => {
                let checked = tesseract_contracts::Prediction::new(
                    prediction.probabilities,
                    prediction.revision,
                )?;
                if checked.revision != self.revision {
                    return Err(Error::Invalid("thought revision mismatch".into()));
                }
                format!(
                    "Estimated outcome probabilities: {}.",
                    checked
                        .probabilities
                        .iter()
                        .enumerate()
                        .map(|(i, p)| format!("state {i}: {}", percentage(*p)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            (Term::Seek(goal), Value::Program(program)) => {
                program.query.validate()?;
                if program.query.features != goal.features
                    || program.query.initial != goal.initial
                    || !program.expected_success.is_finite()
                    || !(0.0..=1.0).contains(&program.expected_success)
                {
                    return Err(Error::Invalid("invalid thought program".into()));
                }
                format!(
                    "Suggested actions: {}. Estimated chance of state {}: {}.",
                    program
                        .query
                        .actions
                        .iter()
                        .map(|a| action_name(*a))
                        .collect::<Vec<_>>()
                        .join(" then "),
                    goal.target,
                    percentage(program.expected_success)
                )
            }
            _ => return Err(Error::Invalid("thought term/value mismatch".into())),
        };
        Ok(format!(
            "{request}\n{result}\nModel revision {}; evidence head {}.",
            self.revision, self.evidence_head
        ))
    }
}
