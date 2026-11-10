//! SI v0: bounded symbolic terms evaluated against persistent cognitive state.
pub mod english;
use serde::{Deserialize, Serialize};
use tesseract_contracts::{
    Action, Budget, Error, Experiment, Features, Goal, Prediction, Query, Result,
};
use tesseract_kernel::{CognitiveKernel, Kernel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Term {
    Predict(Query),
    Seek(Goal),
    Concepts,
    Wonder,
    /// Open-text question over explicitly supplied scientific reading memory.
    Recall(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Prediction(Prediction),
    Program(Experiment),
    Concepts(Vec<tesseract_nucleus::Concept>),
    Curiosity(tesseract_nucleus::Curiosity),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub term: Term,
    pub value: Value,
    pub evidence_head: String,
    pub revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingThought {
    pub term: Term,
    pub answer: tesseract_knowledge::Answer,
}
impl Term {
    /// Canonical surface syntax has no free-form instructions or implicit context.
    /// `predict porous 0 heat press` or `seek solid 1 3`.
    pub fn parse(source: &str) -> Result<Self> {
        if source.len() <= 8192
            && let Some(question) = source.trim().strip_prefix("recall ")
        {
            let question: String = serde_json::from_str(question)?;
            tesseract_knowledge::validate_question(&question)?;
            return Ok(Self::Recall(question));
        }
        if source.len() > 256 {
            return Err(Error::Budget("SI term exceeds 256 bytes".into()));
        }
        match source.trim() {
            "concepts" => return Ok(Self::Concepts),
            "wonder" => return Ok(Self::Wonder),
            _ => {}
        }
        let words: Vec<_> = source.split_whitespace().collect();
        if words.len() < 4 || words.len() > 6 {
            return Err(Error::Invalid("SI arity".into()));
        }
        let features = Features {
            porous: match words[1] {
                "porous" => true,
                "solid" => false,
                _ => return Err(Error::Invalid("SI feature".into())),
            },
        };
        let state = |s: &str| -> Result<u8> {
            let n = s
                .parse::<u8>()
                .map_err(|_| Error::Invalid("SI outcome".into()))?;
            if n < 4 {
                Ok(n)
            } else {
                Err(Error::Invalid("SI outcome".into()))
            }
        };
        let initial = state(words[2])?;
        match words[0] {
            "seek" if words.len() == 4 => Ok(Self::Seek(Goal {
                features,
                initial,
                target: state(words[3])?,
            })),
            "predict" => {
                let actions = words[3..]
                    .iter()
                    .map(|a| match *a {
                        "heat" => Ok(Action::Heat),
                        "press" => Ok(Action::Press),
                        "cool" => Ok(Action::Cool),
                        _ => Err(Error::Invalid("SI action".into())),
                    })
                    .collect::<Result<Vec<_>>>()?;
                let query = Query {
                    features,
                    initial,
                    actions,
                };
                query.validate()?;
                Ok(Self::Predict(query))
            }
            _ => Err(Error::Invalid("SI operator/arity".into())),
        }
    }
    pub fn evaluate(&self, kernel: &Kernel, budget: Budget) -> Result<Thought> {
        budget.validate()?;
        let value = match self {
            Self::Recall(_) => {
                return Err(Error::Invalid(
                    "recall requires reading memory, not a workshop kernel".into(),
                ));
            }
            Self::Concepts => {
                Value::Concepts(tesseract_nucleus::Nucleus::from_kernel(kernel)?.concepts(16)?)
            }
            Self::Wonder => Value::Curiosity(
                tesseract_nucleus::Nucleus::from_kernel(kernel)?
                    .wonder(budget.evaluations.min(312))?,
            ),
            Self::Predict(query) => Value::Prediction(kernel.predict(query)?),
            Self::Seek(goal) => Value::Program(
                kernel
                    .propose(goal, budget)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Invalid("empty SI search".into()))?,
            ),
        };
        Ok(Thought {
            term: self.clone(),
            value,
            evidence_head: kernel.evidence_head().into(),
            revision: kernel.revision(),
        })
    }
    pub fn evaluate_reading(
        &self,
        memory: &tesseract_knowledge::KnowledgeBase,
    ) -> Result<ReadingThought> {
        let Self::Recall(question) = self else {
            return Err(Error::Invalid("reading memory accepts recall terms".into()));
        };
        Ok(ReadingThought {
            term: self.clone(),
            answer: memory.ask(question)?,
        })
    }
}

/// Ask Z3 to search for a counterexample to maximality among the supplied
/// alternatives. Scores are the exact IEEE binary values, encoded as rationals.
/// This certifies ranking, not learned truth or floating-point channel validity.
pub fn certify_maximum(selected: f64, alternatives: &[f64]) -> Result<()> {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    if alternatives.is_empty()
        || alternatives.len() > 128
        || !selected.is_finite()
        || alternatives.iter().any(|x| !x.is_finite())
        || !(0.0..=1.0).contains(&selected)
        || alternatives.iter().any(|x| !(0.0..=1.0).contains(x))
        || !alternatives.contains(&selected)
    {
        return Err(Error::Invalid("invalid SMT scores".into()));
    }
    fn rational(x: f64) -> String {
        if x == 0.0 {
            return "0".into();
        }
        let bits = x.to_bits();
        let exponent = ((bits >> 52) & 2047) as i32;
        let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
        let shift = if exponent == 0 {
            -1074
        } else {
            exponent - 1023 - 52
        };
        let power = if shift == 0 {
            "1".to_owned()
        } else {
            format!(
                "(* {})",
                std::iter::repeat_n("2", shift.unsigned_abs() as usize)
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        let value = if shift >= 0 {
            format!("(* {mantissa} {power})")
        } else {
            format!("(/ {mantissa} {power})")
        };
        if x < 0.0 {
            format!("(- {value})")
        } else {
            value
        }
    }
    let clauses = alternatives
        .iter()
        .map(|x| format!("(> {} {})", rational(*x), rational(selected)))
        .collect::<Vec<_>>()
        .join(" ");
    let script = format!(
        "(set-option :timeout 5000)\n(set-logic QF_NRA)\n(assert (or {clauses}))\n(check-sat)\n"
    );
    let mut child = Command::new("z3")
        .args(["-in", "-T:10"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| Error::Invalid("SMT stdin".into()))?
        .write_all(script.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() || output.stdout != b"unsat\n" {
        return Err(Error::Rejected(format!(
            "SMT ranking not certified: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use tesseract_kernel::Kind;
    #[test]
    fn language_is_bounded_and_queries_are_read_only() {
        let kernel = Kernel::new(Kind::Complex).unwrap();
        let before = kernel.state_digest().unwrap();
        for source in ["predict porous 0 heat press", "seek solid 1 3"] {
            Term::parse(source)
                .unwrap()
                .evaluate(&kernel, Budget::default())
                .unwrap();
        }
        assert_eq!(kernel.state_digest().unwrap(), before);
        for source in [
            "predict porous 0 heat press cool heat",
            "seek solid 4 1",
            "seek solid 1 4",
            "invent solid 0 1",
            "predict unknown 0 heat",
        ] {
            assert!(Term::parse(source).is_err(), "{source}");
        }
    }
    #[test]
    fn smt_rejects_a_suboptimal_selection_and_invalid_inputs() {
        certify_maximum(0.75, &[0.25, 0.75]).unwrap();
        assert!(certify_maximum(0.25, &[0.25, 0.75]).is_err());
        assert!(certify_maximum(f64::NAN, &[0.0]).is_err());
        assert!(certify_maximum(0.0, &[]).is_err());
        assert!(certify_maximum(1.0, &[0.25, 0.75]).is_err());
        certify_maximum(f64::from_bits(1), &[0.0, f64::from_bits(1)]).unwrap();
    }
}
