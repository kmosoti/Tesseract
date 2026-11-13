//! Fixed synthetic proposal benchmark for online symbolic hypothesis revision.
//! A frozen small LLM is NOT used. This isolates the learning mechanism.

const EPS: f64 = 0.04;
const FORGET: f64 = 0.97;
const PRIOR: [f64; 3] = [0.60, 0.25, 0.15];
const SPLIT_MIN: usize = 24;
const SPLIT_POSTERIOR: f64 = 0.92;
const PHASE_LENGTHS: [usize; 4] = [360, 480, 240, 480];
const METHODS: [&str; 5] = [
    "fixed",
    "retrieval",
    "family_bayes",
    "context_bayes",
    "tesseract",
];
const PHASES: [&str; 4] = ["warmup", "transfer", "novel", "drift"];

#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x6d2b_79f5 } else { seed })
    }

    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn int(&mut self, bound: u32) -> usize {
        (self.next() % bound) as usize
    }
}

#[derive(Clone, Copy)]
struct Example {
    values: [i32; 3],
    family: usize,
    mode: usize,
}

/// Program 3 is the deliberately omitted true meaning in the novel phase.
/// The fixed candidate generator only offers programs 0, 1, and 2.
fn program(id: usize, x: [i32; 3]) -> bool {
    let [a, b, c] = x;
    match id {
        0 => c > b && b > a,
        1 => c >= 8 && b >= 8,
        2 => c - b >= 4,
        3 => c >= 8 && c > b && b > a,
        _ => unreachable!("bounded program id"),
    }
}

fn candidate_outputs(x: [i32; 3]) -> [bool; 3] {
    [program(0, x), program(1, x), program(2, x)]
}

#[derive(Clone)]
struct Belief {
    logp: [f64; 3],
    observations: usize,
}

impl Belief {
    fn new() -> Self {
        Self {
            logp: PRIOR.map(f64::ln),
            observations: 0,
        }
    }

    fn probabilities(&self) -> [f64; 3] {
        let highest = self.logp.into_iter().fold(f64::NEG_INFINITY, f64::max);
        let mut weights = self.logp.map(|v| (v - highest).exp());
        let total: f64 = weights.into_iter().sum();
        for weight in &mut weights {
            *weight /= total;
        }
        weights
    }

    fn best(&self) -> usize {
        let p = self.probabilities();
        let mut best = 0;
        for i in 1..3 {
            if p[i] > p[best] {
                best = i;
            }
        }
        best
    }

    fn predict(&self, x: [i32; 3]) -> f64 {
        let probabilities = self.probabilities();
        let bits = candidate_outputs(x);
        EPS + (1.0 - 2.0 * EPS)
            * probabilities
                .iter()
                .enumerate()
                .map(|(i, weight)| if bits[i] { *weight } else { 0.0 })
                .sum::<f64>()
    }

    fn update(&mut self, x: [i32; 3], target: bool) {
        let bits = candidate_outputs(x);
        for (i, evidence) in self.logp.iter_mut().enumerate() {
            let likelihood = if bits[i] == target { 1.0 - EPS } else { EPS };
            *evidence = FORGET * *evidence + likelihood.ln();
        }
        self.observations += 1;
    }
}

struct Frozen;

impl Frozen {
    fn predict(&self, x: [i32; 3]) -> f64 {
        let bits = candidate_outputs(x);
        EPS + (1.0 - 2.0 * EPS)
            * PRIOR
                .iter()
                .enumerate()
                .map(|(i, weight)| if bits[i] { *weight } else { 0.0 })
                .sum::<f64>()
    }
}

struct FamilyBayes([Belief; 4]);

impl FamilyBayes {
    fn new() -> Self {
        Self(std::array::from_fn(|_| Belief::new()))
    }

    fn predict(&self, example: Example) -> f64 {
        self.0[example.family].predict(example.values)
    }

    fn update(&mut self, example: Example, target: bool) {
        self.0[example.family].update(example.values, target);
    }
}

struct ContextBayes([Belief; 8]);

impl ContextBayes {
    fn new() -> Self {
        Self(std::array::from_fn(|_| Belief::new()))
    }

    fn index(example: Example) -> usize {
        example.family * 2 + example.mode
    }

    fn predict(&self, example: Example) -> f64 {
        self.0[Self::index(example)].predict(example.values)
    }

    fn update(&mut self, example: Example, target: bool) {
        self.0[Self::index(example)].update(example.values, target);
    }
}

/// Mitosis is a specific, prespecified context split. It is not general genesis.
struct Plastic {
    pooled: FamilyBayes,
    modes: ContextBayes,
    split: [bool; 4],
    minted: usize,
}

impl Plastic {
    fn new() -> Self {
        Self {
            pooled: FamilyBayes::new(),
            modes: ContextBayes::new(),
            split: [false; 4],
            minted: 0,
        }
    }

    fn predict(&self, example: Example) -> f64 {
        if self.split[example.family] {
            self.modes.predict(example)
        } else {
            self.pooled.predict(example)
        }
    }

    fn update(&mut self, example: Example, target: bool) {
        self.pooled.update(example, target);
        self.modes.update(example, target);
        if !self.split[example.family] {
            let a = &self.modes.0[example.family * 2];
            let b = &self.modes.0[example.family * 2 + 1];
            let max_a = a.probabilities().into_iter().fold(0.0, f64::max);
            let max_b = b.probabilities().into_iter().fold(0.0, f64::max);
            if a.observations >= SPLIT_MIN
                && b.observations >= SPLIT_MIN
                && a.best() != b.best()
                && max_a >= SPLIT_POSTERIOR
                && max_b >= SPLIT_POSTERIOR
            {
                self.split[example.family] = true;
                self.minted += 1;
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Memory {
    values: [i32; 3],
    target: bool,
}

struct Retrieval([Vec<Memory>; 8]);

impl Retrieval {
    fn new() -> Self {
        Self(std::array::from_fn(|_| Vec::new()))
    }

    fn predict(&self, example: Example) -> f64 {
        let history = &self.0[ContextBayes::index(example)];
        if history.is_empty() {
            return 0.5;
        }
        let mut ranked: Vec<(i32, bool)> = history
            .iter()
            .map(|memory| {
                let distance = example
                    .values
                    .iter()
                    .zip(memory.values)
                    .map(|(a, b)| (a - b).abs())
                    .sum::<i32>();
                (distance, memory.target)
            })
            .collect();
        ranked.sort_by_key(|entry| entry.0);
        let (mut yes, mut no) = (1.0, 1.0);
        for &(distance, target) in ranked.iter().take(9) {
            let weight = 4.0 / (1.0 + f64::from(distance));
            if target {
                yes += weight;
            } else {
                no += weight;
            }
        }
        yes / (yes + no)
    }

    fn update(&mut self, example: Example, target: bool) {
        let history = &mut self.0[ContextBayes::index(example)];
        history.push(Memory {
            values: example.values,
            target,
        });
        if history.len() > 100 {
            history.remove(0);
        }
    }
}

struct SeedResult {
    losses: [[f64; 5]; 4],
    split: [bool; 4],
    minted: usize,
}

fn run_seed(seed: u32) -> SeedResult {
    let mut rng = Rng::new(seed ^ 0x9e37_79b9);
    let fixed = Frozen;
    let mut retrieval = Retrieval::new();
    let mut family = FamilyBayes::new();
    let mut contextual = ContextBayes::new();
    let mut plastic = Plastic::new();
    let mut losses = [[0.0; 5]; 4];

    for (phase, length) in PHASE_LENGTHS.into_iter().enumerate() {
        for i in 0..length {
            let family_id = if phase == 2 { 3 } else { i % 3 };
            let mode = (i / 3 + rng.int(2)) % 2;
            let x = [
                rng.int(12) as i32,
                rng.int(12) as i32,
                rng.int(12) as i32,
            ];
            let chosen = if family_id == 3 {
                3
            } else {
                (family_id
                    + seed as usize % 3
                    + usize::from(mode == 1 && family_id == 1)
                    + usize::from(phase == 3 && family_id == 0 && mode == 0))
                    % 3
            };
            let mut target = program(chosen, x);
            if rng.int(1000) < 40 {
                target = !target;
            }
            let example = Example {
                values: x,
                family: family_id,
                mode,
            };
            let predictions = [
                fixed.predict(x),
                retrieval.predict(example),
                family.predict(example),
                contextual.predict(example),
                plastic.predict(example),
            ];
            for (sum, prediction) in losses[phase].iter_mut().zip(predictions) {
                *sum += (prediction - f64::from(u8::from(target))).powi(2);
            }

            // Prequential evaluation: predict first; only then reveal feedback.
            retrieval.update(example, target);
            family.update(example, target);
            contextual.update(example, target);
            plastic.update(example, target);
        }
        for loss in &mut losses[phase] {
            *loss /= length as f64;
        }
    }
    SeedResult {
        losses,
        split: plastic.split,
        minted: plastic.minted,
    }
}

fn benchmark() -> serde_json::Value {
    let seeds: Vec<_> = (1000..1100).map(run_seed).collect();
    let mut phases = serde_json::Map::new();
    for (phase, name) in PHASES.iter().enumerate() {
        let mut methods = serde_json::Map::new();
        for (idx, method) in METHODS.iter().enumerate() {
            let mean =
                seeds.iter().map(|seed| seed.losses[phase][idx]).sum::<f64>() / seeds.len() as f64;
            methods.insert((*method).to_string(), serde_json::json!(mean));
        }
        phases.insert((*name).to_string(), serde_json::Value::Object(methods));
    }

    let mut rng = Rng::new(0x7465_7373);
    let mut relative_gains = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let mut candidate = 0.0;
        let mut baseline = 0.0;
        for _ in &seeds {
            let seed = &seeds[rng.int(seeds.len() as u32)];
            candidate += seed.losses[1][4];
            baseline += seed.losses[1][1];
        }
        relative_gains.push((baseline - candidate) / baseline);
    }
    relative_gains.sort_by(f64::total_cmp);

    let false_splits = seeds.iter().filter(|seed| seed.split[3]).count();
    let mean_mints = seeds.iter().map(|seed| seed.minted as f64).sum::<f64>()
        / seeds.len() as f64;

    serde_json::json!({
        "kind": "synthetic_fixed_candidate_proposer_not_llm",
        "seed_start": 1000,
        "seed_end": 1099,
        "examples_per_seed": PHASE_LENGTHS.iter().sum::<usize>(),
        "missing_program_in_novel_phase": true,
        "noise": EPS,
        "forgetting": FORGET,
        "phase_mean_brier": phases,
        "transfer_gain_vs_retrieval_ci95": [relative_gains[250], relative_gains[9750]],
        "spurious_splits_novel_family": false_splits,
        "mean_minted_splits": mean_mints
    })
}

#[test]
fn frozen_proposal_online_learning_benchmark() {
    let report = benchmark();
    println!("TESSERACT_PLASTIC_SYMBOLS_RESULT={report}");
    let phase = &report["phase_mean_brier"];
    let tess = phase["transfer"]["tesseract"].as_f64().unwrap();
    let retrieval = phase["transfer"]["retrieval"].as_f64().unwrap();
    assert!(tess.is_finite());
    assert!(retrieval.is_finite());
    // Negative outcomes are recorded, not disallowed by a passing test.
    assert_eq!(run_seed(1000).losses, run_seed(1000).losses);
}
