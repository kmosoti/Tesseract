//! Grounded working abstractions and disagreement-driven experiment selection.
//! Derived from durable public observations; never installs numerical competence.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tesseract_contracts::{Action, Error, Experience, Features, Query, Result, digest, sequences};
use tesseract_kernel::{Event, Kernel};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptStatus {
    Candidate,
    SupportedWorking,
    Contested,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    Pooled,
    Porous,
    Initial,
    PorousAndInitial,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CuriosityReason {
    CommitteeDisagreement,
    CoverageFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pattern {
    pub porous: Option<bool>,
    pub initial: Option<u8>,
    pub actions: Vec<Action>,
}
impl Pattern {
    fn matches(&self, e: &Experience) -> bool {
        self.porous.is_none_or(|p| p == e.query.features.porous)
            && self.initial.is_none_or(|i| i == e.query.initial)
            && self.actions == e.query.actions
    }
    fn for_query(query: &Query, structure: usize) -> Self {
        Self {
            porous: if structure & 1 != 0 {
                Some(query.features.porous)
            } else {
                None
            },
            initial: if structure & 2 != 0 {
                Some(query.initial)
            } else {
                None
            },
            actions: query.actions.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub pattern: Pattern,
    pub outcome: u8,
    pub support_objects: usize,
    pub counterexample_objects: usize,
    pub support_refs: Vec<String>,
    pub counterexample_refs: Vec<String>,
    pub cross_condition_support: bool,
    pub status: ConceptStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interpretation {
    pub structure: Grouping,
    pub weight: f64,
    pub probabilities: [f64; 4],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curiosity {
    pub query: Query,
    pub disagreement: f64,
    pub interpretations: Vec<Interpretation>,
    pub reason: CuriosityReason,
    pub evaluated: usize,
    pub observations: usize,
    pub evidence_head: String,
    pub revision: u64,
}
fn valid_digest(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Concept {
    pub fn validate(&self) -> Result<()> {
        if self.pattern.actions.is_empty() || self.pattern.actions.len() > 3 {
            return Err(Error::Budget("concept actions outside bounds".into()));
        }
        Query {
            features: Features {
                porous: self.pattern.porous.unwrap_or(false),
            },
            initial: self.pattern.initial.unwrap_or(0),
            actions: self.pattern.actions.clone(),
        }
        .validate()?;
        if self.outcome >= 4
            || self.support_objects == 0
            || self.support_objects > 4096
            || self.counterexample_objects > 4096
            || self.support_objects + self.counterexample_objects > 4096
            || self.support_refs.len() > 8
            || self.counterexample_refs.len() > 8
            || self.support_refs.is_empty()
            || (self.counterexample_objects == 0) != self.counterexample_refs.is_empty()
            || self.support_refs.iter().collect::<BTreeSet<_>>().len() != self.support_refs.len()
            || self
                .counterexample_refs
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.counterexample_refs.len()
            || self
                .support_refs
                .iter()
                .any(|r| self.counterexample_refs.contains(r))
            || self
                .support_refs
                .iter()
                .chain(&self.counterexample_refs)
                .any(|r| !valid_digest(r))
            || self.id != digest(&(&self.pattern, self.outcome))?
        {
            return Err(Error::Invalid("invalid concept".into()));
        }
        let expected = if self.counterexample_objects > 0 {
            ConceptStatus::Contested
        } else if self.support_objects >= 3 && self.cross_condition_support {
            ConceptStatus::SupportedWorking
        } else {
            ConceptStatus::Candidate
        };
        if self.status != expected {
            return Err(Error::Invalid("inconsistent concept status".into()));
        }
        if self.status == ConceptStatus::SupportedWorking
            && (self.support_objects < 3 || !self.cross_condition_support)
        {
            return Err(Error::Invalid("unsupported working concept".into()));
        }
        Ok(())
    }
}
impl Curiosity {
    pub fn validate(&self) -> Result<()> {
        self.query.validate()?;
        let names = [
            Grouping::Pooled,
            Grouping::Porous,
            Grouping::Initial,
            Grouping::PorousAndInitial,
        ];
        if self.interpretations.len() != 4
            || self.evaluated == 0
            || self.evaluated > 312
            || !valid_digest(&self.evidence_head)
            || !self.disagreement.is_finite()
            || self.disagreement < 0.0
            || self.disagreement > 4f64.ln() + 1e-9
            || self.observations > 4096
        {
            return Err(Error::Invalid("invalid curiosity record".into()));
        }
        if !candidate_schedule(self.observations, self.evaluated).contains(&self.query) {
            return Err(Error::Invalid(
                "query outside recorded search schedule".into(),
            ));
        }
        for (p, name) in self.interpretations.iter().zip(names) {
            tesseract_contracts::Prediction::new(p.probabilities, 0)?;
            if p.structure != name || !p.weight.is_finite() || !(0.0..=1.0).contains(&p.weight) {
                return Err(Error::Invalid("invalid hypothesis weights".into()));
            }
        }
        let weights: Vec<_> = self.interpretations.iter().map(|p| p.weight).collect();
        let predicted: Vec<_> = self
            .interpretations
            .iter()
            .map(|p| p.probabilities)
            .collect();
        if (weights.iter().sum::<f64>() - 1.0).abs() > 1e-9
            || (disagreement(&predicted, &weights) - self.disagreement).abs() > 1e-9
            || (self.disagreement > 1e-12)
                != (self.reason == CuriosityReason::CommitteeDisagreement)
        {
            return Err(Error::Invalid("inconsistent curiosity score".into()));
        }
        Ok(())
    }
}
#[derive(Clone)]
struct Hypothesis {
    rows: BTreeMap<Pattern, [usize; 4]>,
    log_evidence: f64,
}
pub struct Nucleus {
    observations: Vec<Experience>,
    head: String,
    revision: u64,
    models: Vec<Hypothesis>,
    weights: Vec<f64>,
}
fn distribution(counts: [usize; 4]) -> [f64; 4] {
    let total = counts.iter().sum::<usize>() as f64 + 1.0;
    counts.map(|n| (n as f64 + 0.25) / total)
}
fn entropy(p: &[f64; 4]) -> f64 {
    p.iter().filter(|p| **p > 0.0).map(|p| -p * p.ln()).sum()
}
fn disagreement(predictions: &[[f64; 4]], weights: &[f64]) -> f64 {
    let mean =
        std::array::from_fn(|i| predictions.iter().zip(weights).map(|(p, w)| p[i] * w).sum());
    (entropy(&mean)
        - predictions
            .iter()
            .zip(weights)
            .map(|(p, w)| entropy(p) * w)
            .sum::<f64>())
    .max(0.0)
}
fn candidate_schedule(observations: usize, evaluations: usize) -> Vec<Query> {
    let mut all = Vec::new();
    for porous in [false, true] {
        for initial in 0..4 {
            for actions in sequences() {
                all.push(Query {
                    features: Features { porous },
                    initial,
                    actions,
                });
            }
        }
    }
    if evaluations == 312 {
        return all;
    }
    // 131 is coprime to 312. Small budgets traverse the entire query universe
    // as evidence accumulates, rather than permanently starving later contexts.
    (0..evaluations)
        .map(|i| all[((observations + i) * 131) % 312].clone())
        .collect()
}
impl Nucleus {
    pub fn from_kernel(kernel: &Kernel) -> Result<Self> {
        let observations: Vec<_> = kernel
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::Observed(e) => Some(e.clone()),
                _ => None,
            })
            .collect();
        if observations.len() > 4096 {
            return Err(Error::Budget("nucleus evidence exceeds 4096".into()));
        }
        let mut models = vec![
            Hypothesis {
                rows: BTreeMap::new(),
                log_evidence: 0.0
            };
            4
        ];
        for e in &observations {
            e.validate()?;
            for (structure, model) in models.iter_mut().enumerate() {
                let counts = model
                    .rows
                    .entry(Pattern::for_query(&e.query, structure))
                    .or_insert([0; 4]);
                model.log_evidence += distribution(*counts)[e.outcome as usize].ln();
                counts[e.outcome as usize] += 1;
            }
        }
        let maximum = models
            .iter()
            .map(|m| m.log_evidence)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut weights: Vec<_> = models
            .iter()
            .map(|m| (m.log_evidence - maximum).exp())
            .collect();
        let sum: f64 = weights.iter().sum();
        for w in &mut weights {
            *w /= sum;
        }
        Ok(Self {
            observations,
            head: kernel.evidence_head().into(),
            revision: kernel.revision(),
            models,
            weights,
        })
    }
    /// At most eight example references per side; counts include all distinct objects.
    /// Supported means a working generalization on observed conditions, not a proof.
    pub fn concepts(&self, limit: usize) -> Result<Vec<Concept>> {
        if limit == 0 || limit > 128 {
            return Err(Error::Budget("concept limit must be 1..=128".into()));
        }
        let mut concepts = Vec::new();
        for model in &self.models {
            for pattern in model.rows.keys() {
                for outcome in 0..4 {
                    let mut support = BTreeSet::new();
                    let mut counters = BTreeSet::new();
                    let mut initials = BTreeSet::new();
                    let mut features = BTreeSet::new();
                    let mut support_refs = BTreeSet::new();
                    let mut counter_refs = BTreeSet::new();
                    for (ordinal, e) in self
                        .observations
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| pattern.matches(e))
                    {
                        if e.outcome == outcome {
                            support.insert(&e.object);
                            initials.insert(e.query.initial);
                            features.insert(e.query.features.porous);
                            if support_refs.len() < 8 {
                                support_refs.insert(digest(&(ordinal, e))?);
                            }
                        } else {
                            counters.insert(&e.object);
                            if counter_refs.len() < 8 {
                                counter_refs.insert(digest(&(ordinal, e))?);
                            }
                        }
                    }
                    if support.is_empty() {
                        continue;
                    }
                    let cross = (pattern.initial.is_some() || initials.len() >= 2)
                        && (pattern.porous.is_some() || features.len() == 2);
                    let status = if !counters.is_empty() {
                        ConceptStatus::Contested
                    } else if support.len() >= 3 && cross {
                        ConceptStatus::SupportedWorking
                    } else {
                        ConceptStatus::Candidate
                    };
                    concepts.push(Concept {
                        id: digest(&(pattern, outcome))?,
                        pattern: pattern.clone(),
                        outcome,
                        support_objects: support.len(),
                        counterexample_objects: counters.len(),
                        support_refs: support_refs.into_iter().collect(),
                        counterexample_refs: counter_refs.into_iter().collect(),
                        cross_condition_support: cross,
                        status,
                    });
                }
            }
        }
        concepts.sort_by(|a, b| {
            let priority = |c: &Concept| match c.status {
                ConceptStatus::SupportedWorking => 0,
                ConceptStatus::Contested => 1,
                ConceptStatus::Candidate => 2,
            };
            let generality = |c: &Concept| {
                usize::from(c.pattern.porous.is_none()) + usize::from(c.pattern.initial.is_none())
            };
            priority(a).cmp(&priority(b)).then(
                generality(b)
                    .cmp(&generality(a))
                    .then(b.support_objects.cmp(&a.support_objects))
                    .then(a.id.cmp(&b.id)),
            )
        });
        concepts.truncate(limit);
        Ok(concepts)
    }
    pub fn interpretations(&self, query: &Query) -> Result<Vec<Interpretation>> {
        query.validate()?;
        Ok(self
            .models
            .iter()
            .enumerate()
            .map(|(i, m)| Interpretation {
                structure: [
                    Grouping::Pooled,
                    Grouping::Porous,
                    Grouping::Initial,
                    Grouping::PorousAndInitial,
                ][i],
                weight: self.weights[i],
                probabilities: distribution(
                    *m.rows.get(&Pattern::for_query(query, i)).unwrap_or(&[0; 4]),
                ),
            })
            .collect())
    }
    /// Bayesian-weighted predictive disagreement (Jensen-Shannon divergence),
    /// not entropy, expected information gain, or an intrinsic mental state.
    pub fn wonder(&self, evaluations: usize) -> Result<Curiosity> {
        if evaluations == 0 || evaluations > 312 {
            return Err(Error::Budget("wonder evaluations must be 1..=312".into()));
        }
        let candidates = candidate_schedule(self.observations.len(), evaluations);
        let mut best: Option<(f64, usize, Query, Vec<Interpretation>)> = None;
        for query in candidates.into_iter().take(evaluations) {
            let interpretations = self.interpretations(&query)?;
            let score = disagreement(
                &interpretations
                    .iter()
                    .map(|p| p.probabilities)
                    .collect::<Vec<_>>(),
                &self.weights,
            );
            let seen = self
                .observations
                .iter()
                .filter(|e| e.query == query)
                .count();
            if best.as_ref().is_none_or(|(s, n, _, _)| {
                score > *s + 1e-12 || ((score - *s).abs() <= 1e-12 && seen < *n)
            }) {
                best = Some((score, seen, query, interpretations));
            }
        }
        let (score, _, query, interpretations) =
            best.ok_or_else(|| Error::Invalid("empty curiosity search".into()))?;
        Ok(Curiosity {
            query,
            disagreement: score,
            interpretations,
            reason: if score > 1e-12 {
                CuriosityReason::CommitteeDisagreement
            } else {
                CuriosityReason::CoverageFallback
            },
            evaluated: evaluations,
            observations: self.observations.len(),
            evidence_head: self.head.clone(),
            revision: self.revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseract_kernel::{CognitiveKernel, Kind};
    fn observe(kernel: &mut Kernel, object: &str, porous: bool, initial: u8, outcome: u8) {
        kernel
            .observe(Experience {
                object: object.into(),
                query: Query {
                    features: Features { porous },
                    initial,
                    actions: vec![Action::Cool],
                },
                outcome,
            })
            .unwrap();
    }
    #[test]
    fn abstractions_require_cross_condition_support_and_keep_counterexamples() {
        let mut kernel = Kernel::new(Kind::Complex).unwrap();
        for i in 0..3 {
            observe(&mut kernel, &format!("a{i}"), false, 0, 0);
        }
        let concepts = Nucleus::from_kernel(&kernel)
            .unwrap()
            .concepts(128)
            .unwrap();
        assert!(
            !concepts
                .iter()
                .any(|c| c.pattern.initial.is_none() && c.status == ConceptStatus::SupportedWorking)
        );
        observe(&mut kernel, "cross-initial", false, 1, 0);
        let concepts = Nucleus::from_kernel(&kernel)
            .unwrap()
            .concepts(128)
            .unwrap();
        let general = concepts
            .iter()
            .find(|c| {
                c.pattern.initial.is_none() && c.pattern.porous == Some(false) && c.outcome == 0
            })
            .unwrap();
        assert_eq!(general.status, ConceptStatus::SupportedWorking);
        let id = general.id.clone();
        observe(&mut kernel, "counterexample", false, 2, 1);
        let concepts = Nucleus::from_kernel(&kernel)
            .unwrap()
            .concepts(128)
            .unwrap();
        let revised = concepts.iter().find(|c| c.id == id).unwrap();
        assert_eq!(revised.status, ConceptStatus::Contested);
        assert_eq!(revised.counterexample_objects, 1);
        assert!(!revised.counterexample_refs.is_empty());
    }
    #[test]
    fn disagreement_is_not_entropy_and_changes_attention() {
        assert_eq!(disagreement(&[[0.25; 4]; 2], &[0.5, 0.5]), 0.0);
        let score = disagreement(&[[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]], &[0.5, 0.5]);
        assert!((score - 2f64.ln()).abs() < 1e-12);
        let mut kernel = Kernel::new(Kind::Bayesian).unwrap();
        let cold = Nucleus::from_kernel(&kernel).unwrap().wonder(312).unwrap();
        assert_eq!(cold.reason, CuriosityReason::CoverageFallback);
        for i in 0..6 {
            observe(&mut kernel, &format!("seed-{i}"), false, (i % 2) as u8, 0);
        }
        let nucleus = Nucleus::from_kernel(&kernel).unwrap();
        let chosen = nucleus.wonder(312).unwrap();
        assert_eq!(chosen.reason, CuriosityReason::CommitteeDisagreement);
        assert_eq!(chosen.query.actions, vec![Action::Cool]);
        assert!(chosen.query.features.porous || chosen.query.initial >= 2);
        let before_weight = nucleus.weights[0];
        let before_hash = kernel.state_digest().unwrap();
        nucleus.wonder(312).unwrap();
        assert_eq!(before_hash, kernel.state_digest().unwrap());
        observe(&mut kernel, "distinguishing-counterexample", true, 2, 1);
        let updated = Nucleus::from_kernel(&kernel).unwrap();
        assert!(updated.weights[0] < before_weight);
    }
    #[test]
    fn journal_replay_reconstructs_the_same_nucleus_and_next_experiment() {
        use tesseract_journal::Session;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let mut session = Session::create(&path, Kind::Complex).unwrap();
        for i in 0..4 {
            session
                .observe(Experience {
                    object: format!("object-{i}"),
                    query: Query {
                        features: Features { porous: i % 2 == 0 },
                        initial: i,
                        actions: vec![Action::Cool],
                    },
                    outcome: 0,
                })
                .unwrap();
        }
        let nucleus = Nucleus::from_kernel(session.kernel()).unwrap();
        let concepts = serde_json::to_value(nucleus.concepts(16).unwrap()).unwrap();
        let choice = serde_json::to_value(nucleus.wonder(312).unwrap()).unwrap();
        assert!(nucleus.wonder(0).is_err());
        assert!(nucleus.wonder(313).is_err());
        drop(session);
        let reopened = Session::open(path).unwrap();
        let restored = Nucleus::from_kernel(reopened.kernel()).unwrap();
        assert_eq!(
            concepts,
            serde_json::to_value(restored.concepts(16).unwrap()).unwrap()
        );
        assert_eq!(
            choice,
            serde_json::to_value(restored.wonder(312).unwrap()).unwrap()
        );
    }
    #[test]
    fn curious_and_coverage_controllers_have_equal_budgets_on_a_fixed_world() {
        use tesseract_workshop::Workshop;
        let mut reports = Vec::new();
        for curious in [false, true] {
            let mut kernel = Kernel::new(Kind::Bayesian).unwrap();
            let mut world = Workshop::new(114, 0.0).unwrap();
            let all: Vec<_> = [false, true]
                .into_iter()
                .flat_map(|porous| {
                    (0..4).flat_map(move |initial| {
                        sequences().into_iter().map(move |actions| Query {
                            features: Features { porous },
                            initial,
                            actions,
                        })
                    })
                })
                .collect();
            let mut trace = Vec::new();
            for query in all.iter().take(48) {
                let query = if curious {
                    Nucleus::from_kernel(&kernel)
                        .unwrap()
                        .wonder(312)
                        .unwrap()
                        .query
                } else {
                    query.clone()
                };
                trace.push(query.clone());
                kernel
                    .observe(world.interact(query, "nucleus-diagnostic").unwrap())
                    .unwrap();
            }
            let nucleus = Nucleus::from_kernel(&kernel).unwrap();
            let mut evaluation = Workshop::evaluation(114, 0.0).unwrap();
            let mut loss = 0.0;
            for query in &all {
                let p = nucleus.interpretations(query).unwrap();
                let mean =
                    std::array::from_fn(|i| p.iter().map(|p| p.weight * p.probabilities[i]).sum());
                let e = evaluation
                    .interact(query.clone(), "held-out-identity")
                    .unwrap();
                loss += tesseract_contracts::Prediction::new(mean, 0)
                    .unwrap()
                    .brier(e.outcome)
                    .unwrap();
            }
            reports.push((trace, loss / all.len() as f64));
        }
        assert_ne!(reports[0].0, reports[1].0);
        println!(
            "nucleus diagnostic seed114 budget48: coverage_loss={} curious_loss={}",
            reports[0].1, reports[1].1
        );
        // No assertion of superiority: generalization and curiosity value are empirical.
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use tesseract_kernel::{CognitiveKernel, Kind};
    #[test]
    fn small_budget_schedule_eventually_visits_every_query() {
        let mut all = BTreeSet::new();
        for observations in 0..312 {
            let query = candidate_schedule(observations, 1).remove(0);
            all.insert((query.features.index(), query.initial, query.actions));
        }
        assert_eq!(all.len(), 312);
        assert!(candidate_schedule(0, 8).iter().any(|q| q.features.porous));
    }
    #[test]
    fn malformed_provenance_status_and_scores_are_rejected() {
        let mut kernel = Kernel::new(Kind::Bayesian).unwrap();
        kernel
            .observe(Experience {
                object: "fixture".into(),
                query: Query {
                    features: Features { porous: false },
                    initial: 0,
                    actions: vec![Action::Cool],
                },
                outcome: 1,
            })
            .unwrap();
        let nucleus = Nucleus::from_kernel(&kernel).unwrap();
        let mut concept = nucleus.concepts(16).unwrap().remove(0);
        concept.validate().unwrap();
        let first_id = kernel.symbols()["fixture"].evidence[0].clone();
        assert_eq!(concept.support_refs, vec![first_id]);
        let saved = concept.clone();
        concept.support_refs.clear();
        assert!(concept.validate().is_err());
        concept = saved.clone();
        concept.support_refs.push(concept.support_refs[0].clone());
        assert!(concept.validate().is_err());
        concept = saved;
        concept.status = ConceptStatus::SupportedWorking;
        assert!(concept.validate().is_err());
        let empty = Kernel::new(Kind::Bayesian).unwrap();
        let mut choice = Nucleus::from_kernel(&empty).unwrap().wonder(1).unwrap();
        choice.validate().unwrap();
        choice.disagreement = -5e-10;
        assert!(choice.validate().is_err());
        choice.disagreement = 0.0;
        choice.query.initial = 3;
        assert!(choice.validate().is_err());
    }
}
