//! Working evidence is immediate; reusable competence changes only by validation.
pub mod density;
mod model;
pub use model::{FitReport, Kind, Model};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tesseract_contracts::{
    Budget, Error, Experience, Experiment, Goal, ObservationReceipt, Prediction, Process, Query,
    Result, digest, sequences,
};

const MAX_EVIDENCE: usize = 4096;
const MAX_VALIDATION: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Evidence {
    id: String,
    experience: Experience,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub identity: String,
    pub latest_outcome: u8,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRevision {
    parent: u64,
    parent_state: String,
    evidence_head: String,
    budget: Budget,
    model: Model,
    processes: Vec<Process>,
    fit: FitReport,
}
impl CandidateRevision {
    pub fn fit_report(&self) -> &FitReport {
        &self.fit
    }
    pub fn processes(&self) -> &[Process] {
        &self.processes
    }
    pub fn fingerprint(&self) -> Result<String> {
        digest(self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValidationPolicy {
    pub min_relative_gain: f64,
    pub max_retention_regression: f64,
}
impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            min_relative_gain: 0.01,
            max_retention_regression: 0.01,
        }
    }
}
impl ValidationPolicy {
    fn validate(self) -> Result<()> {
        if !self.min_relative_gain.is_finite()
            || !(0.0..=1.0).contains(&self.min_relative_gain)
            || !self.max_retention_regression.is_finite()
            || !(0.0..=1.0).contains(&self.max_retention_regression)
        {
            return Err(Error::Invalid("invalid validation policy".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub candidate_hash: String,
    pub validation_hash: String,
    pub examples: usize,
    pub previous_loss: f64,
    pub candidate_loss: f64,
    pub relative_gain: f64,
    pub retention_regression: f64,
}

/// Only Kernel::validate can construct this proof; it cannot be deserialized.
#[derive(Debug)]
pub struct ValidatedRevision {
    candidate: CandidateRevision,
    validation: Vec<Experience>,
    policy: ValidationPolicy,
    report: ValidationReport,
}
impl ValidatedRevision {
    pub fn report(&self) -> &ValidationReport {
        &self.report
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RevisionState {
    model: Model,
    processes: Vec<Process>,
    retention: Vec<Experience>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Observed(Experience),
    Committed {
        candidate: Box<CandidateRevision>,
        validation: Vec<Experience>,
        policy: ValidationPolicy,
        report: ValidationReport,
    },
    RolledBack {
        from: u64,
        to: u64,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Kernel {
    kind: Kind,
    active: u64,
    next_revision: u64,
    evidence_head: String,
    evidence: Vec<Evidence>,
    symbols: BTreeMap<String, Symbol>,
    revisions: BTreeMap<u64, RevisionState>,
    events: Vec<Event>,
}

pub trait CognitiveKernel {
    fn observe(&mut self, event: Experience) -> Result<ObservationReceipt>;
    fn predict(&self, query: &Query) -> Result<Prediction>;
    fn propose(&self, goal: &Goal, budget: Budget) -> Result<Vec<Experiment>>;
    fn develop(&self, budget: Budget, seed: u64) -> Result<CandidateRevision>;
    fn commit(&mut self, candidate: ValidatedRevision) -> Result<u64>;
}

impl Kernel {
    pub fn new(kind: Kind) -> Result<Self> {
        let state = RevisionState {
            model: Model::new(kind)?,
            processes: Vec::new(),
            retention: Vec::new(),
        };
        Ok(Self {
            kind,
            active: 0,
            next_revision: 1,
            evidence_head: digest(&"tesseract-evidence-v1")?,
            evidence: Vec::new(),
            symbols: BTreeMap::new(),
            revisions: BTreeMap::from([(0, state)]),
            events: Vec::new(),
        })
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn revision(&self) -> u64 {
        self.active
    }
    pub fn evidence_head(&self) -> &str {
        &self.evidence_head
    }
    pub fn symbols(&self) -> &BTreeMap<String, Symbol> {
        &self.symbols
    }
    pub fn processes(&self) -> &[Process] {
        &self.revisions[&self.active].processes
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn model(&self) -> &Model {
        &self.revisions[&self.active].model
    }
    pub fn state_digest(&self) -> Result<String> {
        digest(self)
    }
    pub fn validate(
        &self,
        candidate: CandidateRevision,
        validation: &[Experience],
        policy: ValidationPolicy,
    ) -> Result<ValidatedRevision> {
        policy.validate()?;
        if candidate.parent != self.active
            || candidate.evidence_head != self.evidence_head
            || candidate.parent_state != digest(&self.revisions[&self.active])?
        {
            return Err(Error::Stale);
        }
        if validation.len() < 4 || validation.len() > MAX_VALIDATION {
            return Err(Error::Invalid(
                "validation requires 4..=1024 public episodes".into(),
            ));
        }
        for e in validation {
            e.validate()?;
            if self.symbols.contains_key(&e.object) {
                return Err(Error::Rejected(
                    "validation object overlaps training identities".into(),
                ));
            }
        }
        if candidate.model.kind() != self.kind || candidate.processes.len() > 128 {
            return Err(Error::Rejected(
                "candidate kind or complexity violates contract".into(),
            ));
        }
        candidate.model.validate()?;
        candidate.budget.validate()?;
        if candidate.processes != self.discover_processes(candidate.budget)? {
            return Err(Error::Rejected(
                "process provenance does not match public experience".into(),
            ));
        }
        let training: Vec<_> = self.evidence.iter().map(|e| e.experience.clone()).collect();
        if candidate.fit.training_examples != training.len()
            || candidate.fit.parameter_scalars != candidate.model.parameter_scalars()
            || candidate.fit.objective_evaluations == 0
            || candidate.fit.objective_evaluations > candidate.budget.evaluations
            || !candidate.fit.objective.is_finite()
            || (candidate.fit.objective - candidate.model.loss(&training)?).abs() > 1e-9
        {
            return Err(Error::Rejected(
                "fit report does not match candidate and evidence".into(),
            ));
        }
        let previous_loss = self.model().loss(validation)?;
        let candidate_loss = candidate.model.loss(validation)?;
        if previous_loss <= 1e-12 {
            return Err(Error::Rejected(
                "zero-loss parent cannot support a relative improvement claim".into(),
            ));
        }
        let relative_gain = (previous_loss - candidate_loss) / previous_loss;
        if relative_gain + 1e-12 < policy.min_relative_gain {
            return Err(Error::Rejected(format!(
                "gain {relative_gain:.6} below promotion threshold"
            )));
        }
        let retention = &self.revisions[&self.active].retention;
        let retention_regression = if retention.is_empty() {
            0.0
        } else {
            candidate.model.loss(retention)? - self.model().loss(retention)?
        };
        if retention_regression > policy.max_retention_regression + 1e-12 {
            return Err(Error::Rejected(
                "unrelated retained competence regressed".into(),
            ));
        }
        let report = ValidationReport {
            candidate_hash: candidate.fingerprint()?,
            validation_hash: digest(validation)?,
            examples: validation.len(),
            previous_loss,
            candidate_loss,
            relative_gain,
            retention_regression,
        };
        Ok(ValidatedRevision {
            candidate,
            validation: validation.to_vec(),
            policy,
            report,
        })
    }
    pub fn rollback(&mut self, target: u64) -> Result<()> {
        if !self.revisions.contains_key(&target) {
            return Err(Error::Invalid("unknown rollback target".into()));
        }
        let from = self.active;
        self.active = target;
        self.events.push(Event::RolledBack { from, to: target });
        Ok(())
    }
    /// Proposal-search ablation: same model and budget, no acquired macros.
    pub fn propose_without_processes(
        &self,
        goal: &Goal,
        budget: Budget,
    ) -> Result<Vec<Experiment>> {
        self.propose_internal(goal, budget, false)
    }
    fn propose_internal(
        &self,
        goal: &Goal,
        budget: Budget,
        use_processes: bool,
    ) -> Result<Vec<Experiment>> {
        budget.validate()?;
        if goal.initial >= 4 || goal.target >= 4 {
            return Err(Error::Invalid("invalid goal outcome".into()));
        }
        let mut candidates = Vec::new();
        if use_processes {
            // Learned conditions and observed outcomes prioritize useful programs
            // before spending the bounded model-evaluation budget.
            for p in self.processes().iter().filter(|p| {
                p.query.features == goal.features
                    && p.query.initial == goal.initial
                    && p.predicted_outcome == goal.target
            }) {
                candidates.push(p.query.actions.clone());
            }
        }
        candidates.extend(sequences());
        let mut visited = BTreeSet::new();
        let mut result = Vec::new();
        for actions in candidates
            .into_iter()
            .filter(|a| visited.insert(a.clone()))
            .take(budget.evaluations)
        {
            let query = Query {
                features: goal.features,
                initial: goal.initial,
                actions,
            };
            let p = self.predict(&query)?;
            let process = if use_processes {
                self.processes()
                    .iter()
                    .find(|p| p.query == query)
                    .map(|p| p.id.clone())
            } else {
                None
            };
            result.push(Experiment {
                query,
                expected_success: p.probabilities[goal.target as usize],
                information: p.entropy,
                process,
            });
        }
        result.sort_by(|a, b| {
            b.expected_success
                .total_cmp(&a.expected_success)
                .then(a.query.actions.len().cmp(&b.query.actions.len()))
                .then(a.query.actions.cmp(&b.query.actions))
        });
        result.truncate(budget.max_processes);
        Ok(result)
    }
    /// Replay revalidates commits against their original public validation data.
    pub fn replay_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Observed(e) => {
                self.observe(e)?;
            }
            Event::RolledBack { from, to } => {
                if from != self.active {
                    return Err(Error::Corrupt("rollback parent mismatch".into()));
                }
                self.rollback(to)?;
            }
            Event::Committed {
                candidate,
                validation,
                policy,
                report,
            } => {
                let proof = self.validate(*candidate, &validation, policy)?;
                if digest(proof.report())? != digest(&report)? {
                    return Err(Error::Corrupt("commit validation report mismatch".into()));
                }
                self.commit(proof)?;
            }
        }
        Ok(())
    }
    fn discover_processes(&self, budget: Budget) -> Result<Vec<Process>> {
        let mut groups: BTreeMap<String, (Query, Vec<&Evidence>)> = BTreeMap::new();
        for e in &self.evidence {
            let key = digest(&e.experience.query)?;
            groups
                .entry(key)
                .or_insert_with(|| (e.experience.query.clone(), Vec::new()))
                .1
                .push(e);
        }
        let mut processes = Vec::new();
        for (query, examples) in groups.into_values() {
            let identities: BTreeSet<_> = examples.iter().map(|e| &e.experience.object).collect();
            if identities.len() < 3 {
                continue;
            }
            let mut counts = [0usize; 4];
            for e in &examples {
                counts[e.experience.outcome as usize] += 1;
            }
            let predicted_outcome = (0..4).max_by_key(|i| counts[*i]).unwrap_or(0) as u8;
            processes.push(Process {
                id: digest(&(&query, predicted_outcome))?,
                query,
                predicted_outcome,
                support: identities.len(),
                evidence: examples.iter().map(|e| e.id.clone()).collect(),
            });
        }
        processes.sort_by(|a, b| b.support.cmp(&a.support).then(a.id.cmp(&b.id)));
        processes.truncate(budget.max_processes);
        Ok(processes)
    }
}

impl CognitiveKernel for Kernel {
    fn observe(&mut self, event: Experience) -> Result<ObservationReceipt> {
        event.validate()?;
        if self
            .revisions
            .values()
            .any(|r| r.retention.iter().any(|e| e.object == event.object))
        {
            return Err(Error::Rejected(
                "accepted validation identities cannot become training evidence".into(),
            ));
        }
        if self.evidence.len() >= MAX_EVIDENCE {
            return Err(Error::Budget(
                "maximum 4096 public observations per session".into(),
            ));
        }
        let ordinal = self.evidence.len();
        let evidence_id = digest(&(ordinal, &event))?;
        let evidence_head = digest(&(&self.evidence_head, &evidence_id))?;
        let symbol = self
            .symbols
            .entry(event.object.clone())
            .or_insert_with(|| Symbol {
                identity: event.object.clone(),
                latest_outcome: event.outcome,
                evidence: Vec::new(),
            });
        symbol.latest_outcome = event.outcome;
        symbol.evidence.push(evidence_id.clone());
        self.evidence.push(Evidence {
            id: evidence_id.clone(),
            experience: event.clone(),
        });
        self.evidence_head = evidence_head.clone();
        self.events.push(Event::Observed(event));
        Ok(ObservationReceipt {
            evidence_id,
            evidence_head,
            ordinal,
        })
    }
    fn predict(&self, query: &Query) -> Result<Prediction> {
        self.model().predict(query, self.active)
    }
    fn propose(&self, goal: &Goal, budget: Budget) -> Result<Vec<Experiment>> {
        self.propose_internal(goal, budget, true)
    }
    fn develop(&self, budget: Budget, seed: u64) -> Result<CandidateRevision> {
        budget.validate()?;
        let evidence: Vec<_> = self.evidence.iter().map(|e| e.experience.clone()).collect();
        let (model, fit) = self.model().fit(&evidence, budget, seed)?;
        let processes = self.discover_processes(budget)?;
        Ok(CandidateRevision {
            parent: self.active,
            parent_state: digest(&self.revisions[&self.active])?,
            evidence_head: self.evidence_head.clone(),
            budget,
            model,
            processes,
            fit,
        })
    }
    fn commit(&mut self, proof: ValidatedRevision) -> Result<u64> {
        if proof.candidate.parent != self.active
            || proof.candidate.evidence_head != self.evidence_head
            || proof.candidate.model.kind() != self.kind
            || proof.candidate.parent_state != digest(&self.revisions[&self.active])?
        {
            return Err(Error::Stale);
        }
        let mut retention = self.revisions[&self.active].retention.clone();
        retention.extend(proof.validation.clone());
        if retention.len() > MAX_VALIDATION {
            return Err(Error::Budget("retention validation ledger is full".into()));
        }
        let state = RevisionState {
            model: proof.candidate.model.clone(),
            processes: proof.candidate.processes.clone(),
            retention,
        };
        let revision = self.next_revision;
        self.revisions.insert(revision, state);
        self.active = revision;
        self.next_revision += 1;
        self.events.push(Event::Committed {
            candidate: Box::new(proof.candidate),
            validation: proof.validation,
            policy: proof.policy,
            report: proof.report,
        });
        Ok(revision)
    }
}
