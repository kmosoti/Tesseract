//! Evaluation lives outside the cognitive boundary and consumes public episodes.
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::Path, time::Instant};
use tesseract_contracts::{
    Budget, Error, Experience, Features, Goal, Prediction, Result, SCHEMA, digest,
};
use tesseract_journal::Session;
use tesseract_kernel::{
    CognitiveKernel, FitReport, Kernel, Kind, Model, ValidationPolicy, ValidationReport,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Protocol {
    pub schema: u32,
    pub noise: f64,
    pub fit_evaluations: usize,
    pub max_processes: usize,
    pub bootstrap_resamples: usize,
    pub minimum_relative_gain: f64,
    pub maximum_cost_ratio: f64,
}
impl Default for Protocol {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            noise: 0.05,
            fit_evaluations: 192,
            max_processes: 64,
            bootstrap_resamples: 10_000,
            minimum_relative_gain: 0.05,
            maximum_cost_ratio: 2.0,
        }
    }
}
impl Protocol {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let p: Self = serde_json::from_reader(fs::File::open(path)?)?;
        p.validate()?;
        Ok(p)
    }
    pub fn validate(&self) -> Result<()> {
        self.budget().validate()?;
        if self.schema != SCHEMA
            || !self.noise.is_finite()
            || !(0.0..=0.25).contains(&self.noise)
            || self.bootstrap_resamples != 10_000
            || !self.minimum_relative_gain.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_relative_gain)
            || !self.maximum_cost_ratio.is_finite()
            || !(1.0..=10.0).contains(&self.maximum_cost_ratio)
        {
            return Err(Error::Invalid(
                "invalid or unsupported experiment protocol".into(),
            ));
        }
        Ok(())
    }
    pub fn budget(&self) -> Budget {
        Budget {
            evaluations: self.fit_evaluations,
            max_processes: self.max_processes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub seed: u64,
    pub kind: Kind,
    pub protocol_hash: String,
    pub training_hash: String,
    pub validation_hash: String,
    pub evaluation_hash: String,
    pub predictions_hash: String,
    pub committed: bool,
    pub processes: usize,
    pub transfer_loss: f64,
    pub retrieval_loss: f64,
    pub frozen_loss: f64,
    pub no_consolidation_loss: f64,
    pub phase_ablated_loss: Option<f64>,
    pub order_ablated_loss: f64,
    pub proposed_success_rate: f64,
    pub proposed_mean_actions: f64,
    pub macro_budgeted_success_rate: f64,
    pub without_macro_success_rate: f64,
    pub mean_predictive_entropy: f64,
    pub fit: FitReport,
    pub validation: Option<ValidationReport>,
    pub rejection: Option<String>,
    pub replay_matches: bool,
    pub queries_read_only: bool,
    pub intervening_observations_preserve_competence: bool,
    pub observed_examples: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostics {
    pub fit_millis: f64,
    pub elapsed_millis: f64,
    pub peak_rss_kib: Option<u64>,
    pub model_bytes: usize,
    pub state_bytes: usize,
    pub journal_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedResult {
    pub behavior: Behavior,
    pub behavior_hash: String,
    pub diagnostics: Diagnostics,
}

fn mean_loss(model: &Model, evidence: &[Experience]) -> Result<f64> {
    model.loss(evidence)
}
fn retrieval_loss(training: &[Experience], test: &[Experience]) -> Result<f64> {
    let mut loss = 0.0;
    for e in test {
        // Contextual nearest-prefix retrieval uses public fields, never object IDs.
        let matches: Vec<_> = training
            .iter()
            .filter(|t| t.query.features == e.query.features && t.query.initial == e.query.initial)
            .collect();
        let similarity = |t: &&Experience| {
            t.query
                .actions
                .iter()
                .zip(&e.query.actions)
                .take_while(|(a, b)| a == b)
                .count()
        };
        let best = matches.iter().map(similarity).max().unwrap_or(0);
        let mut counts = [0.25; 4];
        for t in matches.iter().filter(|t| similarity(t) == best) {
            counts[t.outcome as usize] += 1.0;
        }
        let sum: f64 = counts.iter().sum();
        let p = counts.map(|c| c / sum);
        loss += Prediction::new(p, 0)?.brier(e.outcome)?;
    }
    Ok(loss / test.len() as f64)
}
fn peak_rss() -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
}

fn order_ablated_loss(model: &Model, evidence: &[Experience]) -> Result<f64> {
    let mut loss = 0.0;
    for e in evidence {
        let a = &e.query.actions;
        let mut permutations = std::collections::BTreeSet::new();
        // Evaluation uses triples. Averaging all permutations makes the
        // prediction invariant to action order without changing the model.
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    if i != j && i != k && j != k {
                        permutations.insert(vec![a[i], a[j], a[k]]);
                    }
                }
            }
        }
        let mut probabilities = [0.0; 4];
        for actions in &permutations {
            let query = tesseract_contracts::Query {
                actions: actions.clone(),
                ..e.query.clone()
            };
            let p = model.predict(&query, 0)?;
            for (sum, p) in probabilities.iter_mut().zip(p.probabilities) {
                *sum += p / permutations.len() as f64;
            }
        }
        loss += Prediction::new(probabilities, 0)?.brier(e.outcome)?;
    }
    Ok(loss / evidence.len() as f64)
}

pub fn run_variant(seed: u64, kind: Kind, protocol: &Protocol) -> Result<SeedResult> {
    protocol.validate()?;
    let started = Instant::now();
    let data = tesseract_workshop::dataset(seed, protocol.noise)?;
    let initial = Model::new(Kind::Bayesian)?;
    let no_consolidation_loss = mean_loss(&initial, &data.evaluation)?;
    let mut first_pass = BTreeMap::new();
    for e in &data.training {
        first_pass
            .entry(digest(&e.query)?)
            .or_insert_with(|| e.clone());
    }
    let (frozen, _) = initial.fit(
        &first_pass.into_values().collect::<Vec<_>>(),
        Budget {
            evaluations: 1,
            max_processes: 64,
        },
        seed,
    )?;
    let frozen_loss = mean_loss(&frozen, &data.evaluation)?;
    let retrieval_loss = retrieval_loss(&data.training, &data.evaluation)?;
    let mut kernel = Kernel::new(kind)?;
    for e in &data.training {
        kernel.observe(e.clone())?;
    }
    let fit_start = Instant::now();
    let optimizer_seed = u64::from_str_radix(&digest(&(seed, "fit-optimizer"))?[..16], 16)
        .map_err(|_| Error::Invalid("invalid optimizer seed material".into()))?;
    let candidate = kernel.develop(protocol.budget(), optimizer_seed)?;
    let fit = candidate.fit_report().clone();
    let mut validation = None;
    let mut rejection = None;
    match kernel.validate(candidate, &data.validation, ValidationPolicy::default()) {
        Ok(proof) => {
            validation = Some(proof.report().clone());
            kernel.commit(proof)?;
        }
        Err(Error::Rejected(reason)) => {
            rejection = Some(reason);
        }
        Err(other) => return Err(other),
    }
    let fit_millis = fit_start.elapsed().as_secs_f64() * 1000.0;
    let competence = digest(kernel.model())?;
    // Other public experiences alter working memory, not accepted operators.
    for (i, e) in data.training.iter().take(8).enumerate() {
        let mut intervening = e.clone();
        intervening.object = format!("intervening-{seed}-{i}");
        intervening.outcome = (e.outcome + 1) % 4;
        kernel.observe(intervening)?;
    }
    let intervening_observations_preserve_competence = competence == digest(kernel.model())?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("events.jsonl");
    let stored = Session::store(&path, &kernel)?;
    let journal_bytes = fs::metadata(&path)?.len();
    drop(stored);
    let restored = Session::open(&path)?;
    let replay_matches = restored.kernel().state_digest()? == kernel.state_digest()?;
    let before = restored.kernel().state_digest()?;
    let mut predictions = Vec::new();
    let mut transfer_loss = 0.0;
    for e in &data.evaluation {
        let p = restored.kernel().predict(&e.query)?;
        transfer_loss += p.brier(e.outcome)?;
        predictions.push(p);
    }
    transfer_loss /= data.evaluation.len() as f64;
    let phase_ablated_loss = if kind == Kind::Complex {
        Some(
            restored
                .kernel()
                .model()
                .phase_ablation()?
                .loss(&data.evaluation)?,
        )
    } else {
        None
    };
    let order_ablated_loss = order_ablated_loss(restored.kernel().model(), &data.evaluation)?;
    let mut world = tesseract_workshop::Workshop::evaluation(seed, protocol.noise)?;
    let mut macro_world = tesseract_workshop::Workshop::evaluation(seed, protocol.noise)?;
    let mut ablated_world = tesseract_workshop::Workshop::evaluation(seed, protocol.noise)?;
    let mut successes = 0;
    let mut macro_successes = 0;
    let mut ablated_successes = 0;
    let mut action_count = 0;
    for porous in [false, true] {
        for initial in 0..4 {
            for target in 0..4 {
                let goal = Goal {
                    features: Features { porous },
                    initial,
                    target,
                };
                let proposed = restored.kernel().propose(
                    &goal,
                    Budget {
                        evaluations: 39,
                        max_processes: 1,
                    },
                )?;
                let experiment = &proposed[0];
                action_count += experiment.query.actions.len();
                let observed = world.interact(experiment.query.clone(), "goal-evaluation")?;
                successes += usize::from(observed.outcome == target);
                let tiny_budget = Budget {
                    evaluations: 1,
                    max_processes: 1,
                };
                let learned = restored.kernel().propose(&goal, tiny_budget)?;
                let ablated = restored
                    .kernel()
                    .propose_without_processes(&goal, tiny_budget)?;
                macro_successes += usize::from(
                    macro_world
                        .interact(learned[0].query.clone(), "macro-evaluation")?
                        .outcome
                        == target,
                );
                ablated_successes += usize::from(
                    ablated_world
                        .interact(ablated[0].query.clone(), "macro-ablation")?
                        .outcome
                        == target,
                );
            }
        }
    }
    let queries_read_only = before == restored.kernel().state_digest()?;
    let mean_predictive_entropy =
        predictions.iter().map(|p| p.entropy).sum::<f64>() / predictions.len() as f64;
    let behavior = Behavior {
        seed,
        kind,
        protocol_hash: digest(protocol)?,
        training_hash: digest(&data.training)?,
        validation_hash: digest(&data.validation)?,
        evaluation_hash: digest(&data.evaluation)?,
        predictions_hash: digest(&predictions)?,
        committed: kernel.revision() != 0,
        processes: kernel.processes().len(),
        transfer_loss,
        retrieval_loss,
        frozen_loss,
        no_consolidation_loss,
        phase_ablated_loss,
        order_ablated_loss,
        proposed_success_rate: successes as f64 / 32.0,
        proposed_mean_actions: action_count as f64 / 32.0,
        macro_budgeted_success_rate: macro_successes as f64 / 32.0,
        without_macro_success_rate: ablated_successes as f64 / 32.0,
        mean_predictive_entropy,
        fit,
        validation,
        rejection,
        replay_matches,
        queries_read_only,
        intervening_observations_preserve_competence,
        observed_examples: data.training.len() + 8,
    };
    let diagnostics = Diagnostics {
        fit_millis,
        elapsed_millis: started.elapsed().as_secs_f64() * 1000.0,
        peak_rss_kib: peak_rss(),
        model_bytes: serde_json::to_vec(kernel.model())?.len(),
        state_bytes: serde_json::to_vec(&kernel)?.len(),
        journal_bytes,
    };
    Ok(SeedResult {
        behavior_hash: digest(&behavior)?,
        behavior,
        diagnostics,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub hypothesis: String,
    pub candidate: String,
    pub baseline: String,
    pub mean_candidate_loss: f64,
    pub mean_baseline_loss: f64,
    pub relative_gain: Option<f64>,
    pub paired_95_interval: Option<[f64; 2]>,
    pub fit_time_ratio: Option<f64>,
    pub peak_memory_ratio: Option<f64>,
    pub statistical_gate: bool,
    pub cost_gate: Option<bool>,
    pub verdict: String,
}
fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let v: Vec<_> = values.collect();
    v.iter().sum::<f64>() / v.len() as f64
}
fn interval(pairs: &[(f64, f64)], resamples: usize) -> Option<[f64; 2]> {
    if pairs.len() < 2 {
        return None;
    }
    let mut rng = ChaCha8Rng::seed_from_u64(0x7465737365726163);
    let mut effects = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let (mut baseline, mut candidate) = (0.0, 0.0);
        for _ in pairs {
            let (b, c) = pairs[rng.gen_range(0..pairs.len())];
            baseline += b;
            candidate += c;
        }
        if baseline <= 1e-12 {
            return None;
        }
        effects.push((baseline - candidate) / baseline);
    }
    effects.sort_by(f64::total_cmp);
    Some([
        effects[resamples * 25 / 1000],
        effects[resamples * 975 / 1000],
    ])
}

pub fn compare(
    results: &[SeedResult],
    protocol: &Protocol,
    final_tier: bool,
) -> Result<Vec<Comparison>> {
    protocol.validate()?;
    if results.is_empty() {
        return Err(Error::Invalid("empty result set".into()));
    }
    let mut grouped: BTreeMap<u64, Vec<&SeedResult>> = BTreeMap::new();
    for r in results {
        if r.behavior_hash != digest(&r.behavior)? || r.behavior.protocol_hash != digest(protocol)?
        {
            return Err(Error::Corrupt("result provenance mismatch".into()));
        }
        grouped.entry(r.behavior.seed).or_default().push(r);
    }
    for group in grouped.values() {
        if group.len() != 3
            || Kind::ALL
                .iter()
                .any(|k| group.iter().filter(|r| r.behavior.kind == *k).count() != 1)
            || group.iter().any(|r| {
                !r.behavior.replay_matches
                    || !r.behavior.queries_read_only
                    || !r.behavior.intervening_observations_preserve_competence
                    || r.behavior.evaluation_hash != group[0].behavior.evaluation_hash
                    || r.behavior.training_hash != group[0].behavior.training_hash
                    || r.behavior.validation_hash != group[0].behavior.validation_hash
            })
        {
            return Err(Error::Corrupt(
                "incomplete, unmatched or invalid paired experiment".into(),
            ));
        }
    }
    if final_tier && grouped.keys().copied().collect::<Vec<_>>() != (1000..1100).collect::<Vec<_>>()
    {
        return Err(Error::Invalid(
            "final verdict requires all frozen seeds 1000..1099".into(),
        ));
    }
    let mut comparisons = Vec::new();
    for kind in Kind::ALL {
        let candidates: Vec<_> = results.iter().filter(|r| r.behavior.kind == kind).collect();
        let controls = [
            (
                "contextual_retrieval",
                mean(candidates.iter().map(|r| r.behavior.retrieval_loss)),
            ),
            (
                "frozen_first_pass",
                mean(candidates.iter().map(|r| r.behavior.frozen_loss)),
            ),
            (
                "no_consolidation",
                mean(candidates.iter().map(|r| r.behavior.no_consolidation_loss)),
            ),
        ];
        let (baseline, baseline_mean) = controls
            .into_iter()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("three controls");
        let pairs: Vec<_> = candidates
            .iter()
            .map(|r| {
                let b = match baseline {
                    "contextual_retrieval" => r.behavior.retrieval_loss,
                    "frozen_first_pass" => r.behavior.frozen_loss,
                    _ => r.behavior.no_consolidation_loss,
                };
                (b, r.behavior.transfer_loss)
            })
            .collect();
        let candidate_mean = mean(pairs.iter().map(|p| p.1));
        let gain =
            (baseline_mean > 1e-12).then(|| (baseline_mean - candidate_mean) / baseline_mean);
        let ci = interval(&pairs, protocol.bootstrap_resamples);
        let passes = gain.is_some_and(|g| g >= protocol.minimum_relative_gain)
            && ci.is_some_and(|c| c[0] > 0.0)
            && candidates
                .iter()
                .all(|r| r.behavior.committed && r.behavior.processes > 0);
        comparisons.push(Comparison {
            hypothesis: "durable_transition_learning".into(),
            candidate: format!("{kind:?}"),
            baseline: baseline.into(),
            mean_candidate_loss: candidate_mean,
            mean_baseline_loss: baseline_mean,
            relative_gain: gain,
            paired_95_interval: ci,
            fit_time_ratio: None,
            peak_memory_ratio: None,
            statistical_gate: passes,
            cost_gate: None,
            verdict: if !final_tier {
                "diagnostic"
            } else if passes {
                "supported_on_workshop"
            } else {
                "not_supported"
            }
            .into(),
        });
    }
    for kind in Kind::ALL {
        let candidates: Vec<_> = results.iter().filter(|r| r.behavior.kind == kind).collect();
        let pairs: Vec<_> = candidates
            .iter()
            .map(|r| {
                (
                    1.0 - r.behavior.without_macro_success_rate,
                    1.0 - r.behavior.macro_budgeted_success_rate,
                )
            })
            .collect();
        let baseline_loss = mean(pairs.iter().map(|p| p.0));
        let candidate_loss = mean(pairs.iter().map(|p| p.1));
        let gain =
            (baseline_loss > 1e-12).then(|| (baseline_loss - candidate_loss) / baseline_loss);
        let ci = interval(&pairs, protocol.bootstrap_resamples);
        let passes = gain.is_some_and(|g| g >= protocol.minimum_relative_gain)
            && ci.is_some_and(|v| v[0] > 0.0);
        comparisons.push(Comparison {
            hypothesis: "macro_search_value".into(),
            candidate: format!("{kind:?}"),
            baseline: "same_model_without_macros".into(),
            mean_candidate_loss: candidate_loss,
            mean_baseline_loss: baseline_loss,
            relative_gain: gain,
            paired_95_interval: ci,
            fit_time_ratio: None,
            peak_memory_ratio: None,
            statistical_gate: passes,
            cost_gate: None,
            verdict: if !final_tier {
                "diagnostic"
            } else if passes {
                "supported_at_one_evaluation_budget"
            } else {
                "not_supported"
            }
            .into(),
        });
    }
    let complex: Vec<_> = results
        .iter()
        .filter(|r| r.behavior.kind == Kind::Complex)
        .collect();
    let baseline_kind = [Kind::Bayesian, Kind::Real]
        .into_iter()
        .min_by(|a, b| {
            mean(
                results
                    .iter()
                    .filter(|r| r.behavior.kind == *a)
                    .map(|r| r.behavior.transfer_loss),
            )
            .total_cmp(&mean(
                results
                    .iter()
                    .filter(|r| r.behavior.kind == *b)
                    .map(|r| r.behavior.transfer_loss),
            ))
        })
        .expect("two classical candidates");
    let pairs: Vec<_> = complex
        .iter()
        .map(|r| {
            let b = grouped[&r.behavior.seed]
                .iter()
                .find(|b| b.behavior.kind == baseline_kind)
                .expect("validated group");
            (b.behavior.transfer_loss, r.behavior.transfer_loss)
        })
        .collect();
    let bm = mean(pairs.iter().map(|p| p.0));
    let cm = mean(pairs.iter().map(|p| p.1));
    let gain = (bm > 1e-12).then(|| (bm - cm) / bm);
    let ci = interval(&pairs, protocol.bootstrap_resamples);
    let baseline: Vec<_> = results
        .iter()
        .filter(|r| r.behavior.kind == baseline_kind)
        .collect();
    let bt = mean(baseline.iter().map(|r| r.diagnostics.fit_millis));
    let ct = mean(complex.iter().map(|r| r.diagnostics.fit_millis));
    let time_ratio = (bt > 0.0).then_some(ct / bt);
    let memory_ratio = if results.iter().all(|r| r.diagnostics.peak_rss_kib.is_some()) {
        Some(
            mean(
                complex
                    .iter()
                    .map(|r| r.diagnostics.peak_rss_kib.unwrap_or(0) as f64),
            ) / mean(
                baseline
                    .iter()
                    .map(|r| r.diagnostics.peak_rss_kib.unwrap_or(0) as f64),
            ),
        )
    } else {
        None
    };
    let statistical_gate =
        gain.is_some_and(|g| g >= protocol.minimum_relative_gain) && ci.is_some_and(|c| c[0] > 0.0);
    let cost_gate = time_ratio.is_some_and(|r| r <= protocol.maximum_cost_ratio)
        && memory_ratio.is_some_and(|r| r <= protocol.maximum_cost_ratio);
    comparisons.push(Comparison {
        hypothesis: "complex_phase_value".into(),
        candidate: "Complex".into(),
        baseline: format!("{baseline_kind:?}"),
        mean_candidate_loss: cm,
        mean_baseline_loss: bm,
        relative_gain: gain,
        paired_95_interval: ci,
        fit_time_ratio: time_ratio,
        peak_memory_ratio: memory_ratio,
        statistical_gate,
        cost_gate: Some(cost_gate),
        verdict: if !final_tier {
            "diagnostic"
        } else if statistical_gate && cost_gate {
            "requires_capacity_matched_replication"
        } else {
            "not_supported"
        }
        .into(),
    });
    Ok(comparisons)
}

pub fn write_exclusive<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    if let Some(parent) = path.as_ref().parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_qualifies_durable_transfer_and_rejects_missing_pairs() {
        let p = Protocol {
            fit_evaluations: 1,
            ..Protocol::default()
        };
        let r = run_variant(101, Kind::Bayesian, &p).unwrap();
        assert!(r.behavior.committed && r.behavior.replay_matches && r.behavior.queries_read_only);
        assert!(r.behavior.transfer_loss < r.behavior.no_consolidation_loss);
        assert!(compare(&[r], &p, false).is_err());
        assert!(interval(&[(0.0, 0.0); 2], 10_000).is_none());
    }
    #[test]
    fn protocol_and_outputs_fail_closed() {
        let p = Protocol {
            noise: f64::NAN,
            ..Protocol::default()
        };
        assert!(p.validate().is_err());
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.json");
        write_exclusive(&out, &"first").unwrap();
        assert!(write_exclusive(&out, &"second").is_err());
    }
}
