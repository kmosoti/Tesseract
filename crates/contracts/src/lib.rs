//! Public, representation-independent messages. No simulator or matrix types.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const SCHEMA: u32 = 1;
pub const OUTCOMES: usize = 4;
pub const MAX_SEQUENCE: usize = 3;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("resource budget exceeded: {0}")]
    Budget(String),
    #[error("revision no longer matches its parent or evidence head")]
    Stale,
    #[error("automatic validation rejected candidate: {0}")]
    Rejected(String),
    #[error("corrupt persistence: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Heat,
    Press,
    Cool,
}
impl Action {
    pub const ALL: [Self; 3] = [Self::Heat, Self::Press, Self::Cool];
    pub fn index(self) -> usize {
        match self {
            Self::Heat => 0,
            Self::Press => 1,
            Self::Cool => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Features {
    pub porous: bool,
}
impl Features {
    pub fn index(self) -> usize {
        usize::from(self.porous)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub features: Features,
    pub initial: u8,
    pub actions: Vec<Action>,
}
impl Query {
    pub fn validate(&self) -> Result<()> {
        if self.initial as usize >= OUTCOMES
            || self.actions.is_empty()
            || self.actions.len() > MAX_SEQUENCE
        {
            return Err(Error::Invalid(
                "initial outcome or action sequence outside public bounds".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Experience {
    pub object: String,
    pub query: Query,
    pub outcome: u8,
}
impl Experience {
    pub fn validate(&self) -> Result<()> {
        self.query.validate()?;
        if self.object.is_empty() || self.object.len() > 128 || self.outcome as usize >= OUTCOMES {
            return Err(Error::Invalid("invalid object identity or outcome".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prediction {
    pub probabilities: [f64; OUTCOMES],
    pub entropy: f64,
    pub revision: u64,
}
impl Prediction {
    pub fn new(probabilities: [f64; OUTCOMES], revision: u64) -> Result<Self> {
        if probabilities
            .iter()
            .any(|p| !p.is_finite() || *p < -1e-9 || *p > 1.0 + 1e-9)
            || (probabilities.iter().sum::<f64>() - 1.0).abs() > 1e-9
        {
            return Err(Error::Invalid(
                "prediction is not a probability distribution".into(),
            ));
        }
        let entropy = probabilities
            .iter()
            .filter(|p| **p > 0.0)
            .map(|p| -p * p.ln())
            .sum();
        Ok(Self {
            probabilities,
            entropy,
            revision,
        })
    }
    pub fn brier(&self, outcome: u8) -> Result<f64> {
        if outcome as usize >= OUTCOMES {
            return Err(Error::Invalid("invalid scoring outcome".into()));
        }
        Ok(self
            .probabilities
            .iter()
            .enumerate()
            .map(|(i, p)| (p - f64::from(i == outcome as usize)).powi(2))
            .sum())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Budget {
    pub evaluations: usize,
    pub max_processes: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            evaluations: 192,
            max_processes: 64,
        }
    }
}
impl Budget {
    pub fn validate(self) -> Result<()> {
        if self.evaluations == 0
            || self.evaluations > 16_384
            || self.max_processes == 0
            || self.max_processes > 128
        {
            return Err(Error::Budget(
                "evaluations must be 1..=16384 and processes 1..=128".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub features: Features,
    pub initial: u8,
    pub target: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub query: Query,
    pub expected_success: f64,
    pub information: f64,
    pub process: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Process {
    pub id: String,
    pub query: Query,
    pub predicted_outcome: u8,
    pub support: usize,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationReceipt {
    pub evidence_id: String,
    pub evidence_head: String,
    pub ordinal: usize,
}

/// Deterministic for the typed structures in this workspace. No unordered maps.
pub fn digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

pub fn sequences() -> Vec<Vec<Action>> {
    let mut all = Vec::new();
    let mut level = vec![Vec::new()];
    for _ in 0..MAX_SEQUENCE {
        let mut next = Vec::new();
        for prefix in &level {
            for action in Action::ALL {
                let mut s = prefix.clone();
                s.push(action);
                next.push(s);
            }
        }
        all.extend(next.clone());
        level = next;
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn messages_reject_bad_probabilities_and_unbounded_sequences() {
        assert!(Prediction::new([f64::NAN, 0.0, 0.0, 1.0], 0).is_err());
        assert!(Prediction::new([0.3; 4], 0).is_err());
        assert!(
            Query {
                features: Features { porous: false },
                initial: 0,
                actions: vec![Action::Heat; 4]
            }
            .validate()
            .is_err()
        );
        assert_eq!(sequences().len(), 39);
        assert_eq!(
            Prediction::new([0.25; 4], 0).unwrap().brier(0).unwrap(),
            0.75
        );
    }
}
