use crate::density::{Channel, Density};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use tesseract_contracts::{Budget, Error, Experience, OUTCOMES, Prediction, Query, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Bayesian,
    Real,
    Complex,
}
impl Kind {
    pub const ALL: [Self; 3] = [Self::Bayesian, Self::Real, Self::Complex];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tabular {
    pub(crate) rows: Vec<[[f64; OUTCOMES]; OUTCOMES]>,
}
impl Default for Tabular {
    fn default() -> Self {
        Self {
            rows: vec![[[0.25; OUTCOMES]; OUTCOMES]; 6],
        }
    }
}
impl Tabular {
    fn fit(experiences: &[Experience]) -> Self {
        let mut counts = vec![[[0.25; OUTCOMES]; OUTCOMES]; 6];
        for e in experiences.iter().filter(|e| e.query.actions.len() == 1) {
            counts[e.query.features.index() * 3 + e.query.actions[0].index()]
                [e.query.initial as usize][e.outcome as usize] += 1.0;
        }
        for channel in &mut counts {
            for row in channel {
                let total: f64 = row.iter().sum();
                for p in row {
                    *p /= total;
                }
            }
        }
        Self { rows: counts }
    }
    fn predict(&self, query: &Query) -> [f64; OUTCOMES] {
        let mut state = [0.0; OUTCOMES];
        state[query.initial as usize] = 1.0;
        for action in &query.actions {
            let rows = &self.rows[query.features.index() * 3 + action.index()];
            state = std::array::from_fn(|out| {
                (0..OUTCOMES)
                    .map(|input| state[input] * rows[input][out])
                    .sum()
            });
        }
        state
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Model {
    Bayesian(Tabular),
    Density {
        complex: bool,
        channels: Vec<Channel>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitReport {
    pub objective_evaluations: usize,
    pub training_examples: usize,
    pub objective: f64,
    pub parameter_scalars: usize,
}
impl Model {
    pub fn new(kind: Kind) -> Result<Self> {
        Self::from_tabular(kind, Tabular::default())
    }
    fn from_tabular(kind: Kind, tabular: Tabular) -> Result<Self> {
        if kind == Kind::Bayesian {
            return Ok(Self::Bayesian(tabular));
        }
        Ok(Self::Density {
            complex: kind == Kind::Complex,
            channels: tabular
                .rows
                .iter()
                .map(Channel::from_transition)
                .collect::<Result<_>>()?,
        })
    }
    pub fn kind(&self) -> Kind {
        match self {
            Self::Bayesian(_) => Kind::Bayesian,
            Self::Density { complex: true, .. } => Kind::Complex,
            _ => Kind::Real,
        }
    }
    pub fn predict(&self, query: &Query, revision: u64) -> Result<Prediction> {
        self.validate()?;
        self.predict_validated(query, revision)
    }
    fn predict_validated(&self, query: &Query, revision: u64) -> Result<Prediction> {
        query.validate()?;
        let probabilities = match self {
            Self::Bayesian(tabular) => tabular.predict(query),
            Self::Density { channels, .. } => {
                let mut state = Density::basis(query.initial)?;
                for action in &query.actions {
                    state = channels[query.features.index() * 3 + action.index()].apply(&state);
                }
                state.probabilities()?
            }
        };
        Prediction::new(probabilities, revision)
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Bayesian(tabular) => {
                if tabular.rows.len() != 6 {
                    return Err(Error::Invalid(
                        "expected six context-action operators".into(),
                    ));
                }
                for channel in &tabular.rows {
                    for row in channel {
                        Prediction::new(*row, 0)?;
                    }
                }
            }
            Self::Density { complex, channels } => {
                if channels.len() != 6 {
                    return Err(Error::Invalid(
                        "expected six context-action channels".into(),
                    ));
                }
                for c in channels {
                    c.validate()?;
                    if !complex && c.has_complex_coefficients() {
                        return Err(Error::Invalid("real candidate contains phases".into()));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn parameter_scalars(&self) -> usize {
        match self {
            Self::Bayesian(t) => t.rows.len() * OUTCOMES * (OUTCOMES - 1),
            Self::Density { complex, channels } => channels
                .iter()
                .map(|c| c.stacked().len() * if *complex { 2 } else { 1 })
                .sum(),
        }
    }
    /// Remove imaginary Choi coefficients in the public observation basis.
    /// This channel-level diagnostic is independent of Kraus gauge.
    pub fn phase_ablation(&self) -> Result<Self> {
        self.validate()?;
        match self {
            Self::Bayesian(_) => Ok(self.clone()),
            Self::Density { channels, .. } => {
                let channels = channels
                    .iter()
                    .map(Channel::real_coefficient_ablation)
                    .collect::<Result<_>>()?;
                Ok(Self::Density {
                    complex: false,
                    channels,
                })
            }
        }
    }
    pub fn fit(
        &self,
        experiences: &[Experience],
        budget: Budget,
        seed: u64,
    ) -> Result<(Self, FitReport)> {
        budget.validate()?;
        if experiences.is_empty() {
            return Err(Error::Invalid("cannot fit without experience".into()));
        }
        for e in experiences {
            e.validate()?;
        }
        let mut model = Self::from_tabular(self.kind(), Tabular::fit(experiences))?;
        let mut loss = model.loss(experiences)?;
        let mut evaluations = 1;
        if matches!(model, Self::Density { .. }) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            while evaluations < budget.evaluations {
                let mut trial = model.clone();
                if let Self::Density { complex, channels } = &mut trial {
                    let index = rng.gen_range(0..channels.len());
                    let mut w = channels[index].stacked().clone();
                    let row = rng.gen_range(0..w.nrows());
                    let col = rng.gen_range(0..w.ncols());
                    let step = if rng.gen_bool(0.5) { 0.2 } else { -0.2 };
                    if *complex && rng.gen_bool(0.5) {
                        w[(row, col)].im += step;
                    } else {
                        w[(row, col)].re += step;
                    }
                    if let Ok(channel) = Channel::from_stacked(w) {
                        channels[index] = channel;
                    } else {
                        evaluations += 1;
                        continue;
                    }
                }
                let proposed = trial.loss(experiences)?;
                evaluations += 1;
                if proposed + 1e-12 < loss {
                    model = trial;
                    loss = proposed;
                }
            }
        }
        model.validate()?;
        let report = FitReport {
            objective_evaluations: evaluations,
            training_examples: experiences.len(),
            objective: loss,
            parameter_scalars: model.parameter_scalars(),
        };
        Ok((model, report))
    }
    pub fn loss(&self, evidence: &[Experience]) -> Result<f64> {
        self.validate()?;
        if evidence.is_empty() {
            return Err(Error::Invalid("empty scoring evidence".into()));
        }
        let mut sum = 0.0;
        for e in evidence {
            sum += self.predict_validated(&e.query, 0)?.brier(e.outcome)?;
        }
        Ok(sum / evidence.len() as f64)
    }
}
