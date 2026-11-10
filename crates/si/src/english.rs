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
impl Term {
    /// Canonical SI source, validated even for externally constructed terms.
    pub fn to_source(&self) -> Result<String> {
        if let Self::Predict(query) = self {
            query.validate()?;
        }
        let source = match self {
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
                        .map(|(i, p)| format!("state {i}: {:.2}%", p * 100.0))
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
                    "Suggested actions: {}. Estimated chance of state {}: {:.2}%.",
                    program
                        .query
                        .actions
                        .iter()
                        .map(|a| action_name(*a))
                        .collect::<Vec<_>>()
                        .join(" then "),
                    goal.target,
                    program.expected_success * 100.0
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
