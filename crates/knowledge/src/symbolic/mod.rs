//! Source-grounded, deliberately incomplete symbolic interpretation.
//! Question compilation and realization live in `language`; deductions live in `engine`.
mod engine;
pub mod language;
mod reader;
use crate::Corpus;
use serde::{Deserialize, Serialize};
use tesseract_contracts::{Error, Result, digest};

pub const EXTRACTOR: &str = "conservative-frames-1";
pub const MAX_FACTS: usize = 4096;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Concept(pub String);
impl Concept {
    pub fn new(text: &str) -> Result<Self> {
        let text = text
            .trim()
            .trim_matches(['.', '?'])
            .to_lowercase()
            .replace('ö', "o");
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let text = text
            .strip_prefix("the ")
            .or_else(|| text.strip_prefix("a "))
            .or_else(|| text.strip_prefix("an "))
            .unwrap_or(&text)
            .to_owned();
        if text.is_empty()
            || text.len() > 128
            || text.split_whitespace().count() > 12
            || text
                .chars()
                .any(|c| c.is_control() || matches!(c, '[' | ']' | '{' | '}' | ';' | '?' | '!'))
        {
            return Err(Error::Invalid("concept must be a short noun phrase".into()));
        }
        Ok(Self(text))
    }
    fn validate(&self) -> Result<()> {
        if Self::new(&self.0)? != *self {
            return Err(Error::Invalid("noncanonical concept".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    ClassifiedAs,
    Describes,
    LossOf,
    CanProduce,
    UsableFor,
    UsableTo,
    Alias,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Unqualified,
    Alone,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Element {
    Named { name: Concept },
    LinearCombination { left: Concept, right: Concept },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Atom {
    Relation {
        subject: Concept,
        predicate: Relation,
        object: Concept,
        scope: Scope,
    },
    ClosedUnderLinearCombination {
        equation: Concept,
    },
    Solution {
        element: Element,
        equation: Concept,
    },
}
impl Atom {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Relation {
                subject, object, ..
            } => {
                subject.validate()?;
                object.validate()?;
            }
            Self::ClosedUnderLinearCombination { equation } => equation.validate()?,
            Self::Solution { element, equation } => {
                equation.validate()?;
                match element {
                    Element::Named { name } => name.validate()?,
                    Element::LinearCombination { left, right } => {
                        left.validate()?;
                        right.validate()?;
                    }
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub atom: Atom,
    pub positive: bool,
    /// Reported statements remain attributed, not promoted to global assertions.
    pub according_to: Option<Concept>,
}
impl Claim {
    fn validate(&self) -> Result<()> {
        self.atom.validate()?;
        if let Some(context) = &self.according_to {
            context.validate()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Describe {
        entity: Concept,
    },
    Verify {
        claim: Claim,
    },
    Conditional {
        assumptions: Vec<Claim>,
        conclusion: Claim,
    },
    Unsupported {
        text: String,
    },
}
impl Query {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Describe { entity } => entity.validate(),
            Self::Verify { claim } => claim.validate(),
            Self::Conditional {
                assumptions,
                conclusion,
            } => {
                if assumptions.is_empty() || assumptions.len() > 8 {
                    return Err(Error::Budget("conditional needs 1–8 assumptions".into()));
                }
                for a in assumptions {
                    a.validate()?;
                }
                conclusion.validate()
            }
            Self::Unsupported { text } => crate::validate_question(text),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Witness {
    pub article: usize,
    pub passage: usize,
    pub start: usize,
    pub end: usize,
    pub revision: u64,
    pub url: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fact {
    pub id: usize,
    pub claim: Claim,
    pub extraction_rule: String,
    pub source: Witness,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Interpretation {
    extractor: String,
    corpus_digest: String,
    facts: Vec<Fact>,
    sentences: usize,
    sentences_with_atoms: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterpretationStats {
    pub extractor: String,
    pub facts: usize,
    pub sentences: usize,
    pub sentences_with_atoms: usize,
    pub sentences_without_atoms: usize,
    pub graph_digest: String,
}
impl Interpretation {
    pub(crate) fn derive(corpus: &Corpus) -> Result<Self> {
        reader::interpret(corpus)
    }
    pub fn digest(&self) -> Result<String> {
        digest(self)
    }
    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }
    pub fn stats(&self) -> Result<InterpretationStats> {
        Ok(InterpretationStats {
            extractor: self.extractor.clone(),
            facts: self.facts.len(),
            sentences: self.sentences,
            sentences_with_atoms: self.sentences_with_atoms,
            sentences_without_atoms: self.sentences - self.sentences_with_atoms,
            graph_digest: self.digest()?,
        })
    }
    pub fn evaluate(&self, query: &Query) -> Result<Understanding> {
        query.validate()?;
        engine::evaluate(self, query)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Supported,
    Refuted,
    Conflict,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Premise {
    Source { fact: usize },
    Assumption { index: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofStep {
    pub rule: String,
    pub conclusion: Claim,
    pub premises: Vec<Premise>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Understanding {
    pub query: Query,
    pub verdict: Verdict,
    pub conditional: bool,
    pub proof: Vec<ProofStep>,
    pub sources: Vec<Fact>,
    pub graph_digest: String,
    pub corpus_digest: String,
    pub reason: String,
    pub display_truncated: bool,
}
