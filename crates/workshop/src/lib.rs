//! Evaluator-side simulator. Hidden rules are private and never serialized to kernels.
use rand::{Rng, SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use tesseract_contracts::{Action, Error, Experience, Features, Query, Result, digest, sequences};

#[derive(Clone)]
pub struct Workshop {
    rules: [[[u8; 4]; 3]; 2],
    noise: f64,
    observation_rng: ChaCha8Rng,
    identity_rng: ChaCha8Rng,
}

fn stream(seed: u64, name: &str) -> ChaCha8Rng {
    use rand::RngCore;
    // Explicit named-stream seed material; do not share world and observation draws.
    let hash = digest(&(seed, name)).expect("fixed scalar seed serializes");
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hash[2 * i..2 * i + 2], 16).expect("SHA-256 hex");
    }
    let mut rng = ChaCha8Rng::from_seed(bytes);
    let _ = rng.next_u64();
    rng
}

impl Workshop {
    /// Same hidden rules, independent sensor/identity streams for goal tests.
    pub fn evaluation(seed: u64, noise: f64) -> Result<Self> {
        let mut world = Self::new(seed, noise)?;
        world.observation_rng = stream(seed, "goal-evaluation-observations");
        world.identity_rng = stream(seed, "goal-evaluation-identities");
        Ok(world)
    }
    pub fn new(seed: u64, noise: f64) -> Result<Self> {
        if !noise.is_finite() || !(0.0..=0.25).contains(&noise) {
            return Err(Error::Invalid("noise must be 0..=0.25".into()));
        }
        let mut rng = stream(seed, "world-rules");
        let mut rules = [[[0u8; 4]; 3]; 2];
        // Conjugated permutations preserve a known order-effect fixture while
        // changing the physical labeling independently in every world/context.
        for context in &mut rules {
            let mut labels = [0u8, 1, 2, 3];
            labels.shuffle(&mut rng);
            let mut inverse = [0usize; 4];
            for (i, label) in labels.iter().enumerate() {
                inverse[*label as usize] = i;
            }
            for state in 0..4 {
                let canonical = inverse[state];
                context[0][state] = labels[canonical ^ 1];
                context[1][state] = labels[(canonical + 1) % 4];
                context[2][state] = labels[0];
            }
        }
        Ok(Self {
            rules,
            noise,
            observation_rng: stream(seed, "observations"),
            identity_rng: stream(seed, "identities"),
        })
    }
    /// Returns only the final public sensor reading and its public input message.
    pub fn interact(&mut self, query: Query, namespace: &str) -> Result<Experience> {
        query.validate()?;
        let mut state = query.initial;
        for action in &query.actions {
            state = self.rules[query.features.index()][action.index()][state as usize];
        }
        if self.observation_rng.gen_bool(self.noise) {
            state = (state + 1 + self.observation_rng.gen_range(0..3)) % 4;
        }
        let object = format!("{namespace}-{:016x}", self.identity_rng.r#gen::<u64>());
        let e = Experience {
            object,
            query,
            outcome: state,
        };
        e.validate()?;
        Ok(e)
    }
}

#[derive(Clone)]
pub struct Dataset {
    pub training: Vec<Experience>,
    pub validation: Vec<Experience>,
    pub evaluation: Vec<Experience>,
}

pub fn dataset(seed: u64, noise: f64) -> Result<Dataset> {
    // Separate sensor/identity streams and object identities for each split, same rules.
    let mut world = Workshop::new(seed, noise)?;
    let mut training = Vec::new();
    let mut validation = Vec::new();
    let mut evaluation = Vec::new();
    for porous in [false, true] {
        for initial in 0..4 {
            for action in Action::ALL {
                for _ in 0..6 {
                    training.push(world.interact(
                        Query {
                            features: Features { porous },
                            initial,
                            actions: vec![action],
                        },
                        "train",
                    )?);
                }
            }
        }
    }
    // Multi-step training supports macro discovery but reserves unseen triples.
    for porous in [false, true] {
        for initial in 0..4 {
            for _ in 0..3 {
                training.push(world.interact(
                    Query {
                        features: Features { porous },
                        initial,
                        actions: vec![Action::Heat, Action::Press],
                    },
                    "train",
                )?);
            }
        }
    }
    for porous in [false, true] {
        for initial in 0..4 {
            for actions in sequences() {
                let query = Query {
                    features: Features { porous },
                    initial,
                    actions,
                };
                if query.actions.len() < 3 {
                    validation.push(world.interact(query, "validation")?);
                } else {
                    evaluation.push(world.interact(query, "evaluation")?);
                }
            }
        }
    }
    Ok(Dataset {
        training,
        validation,
        evaluation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixtures_are_reproducible_disjoint_and_have_order_effects() {
        let a = dataset(7, 0.05).unwrap();
        let b = dataset(7, 0.05).unwrap();
        assert_eq!(a.training, b.training);
        assert_eq!(a.evaluation, b.evaluation);
        assert!(a.training.iter().all(|t| {
            a.validation
                .iter()
                .chain(&a.evaluation)
                .all(|e| e.object != t.object)
        }));
        assert!(a.evaluation.iter().all(|e| e.query.actions.len() == 3));
        let mut world = Workshop::new(0, 0.0).unwrap();
        let outcomes: Vec<_> = [
            vec![Action::Heat, Action::Press],
            vec![Action::Press, Action::Heat],
        ]
        .into_iter()
        .map(|actions| {
            world
                .interact(
                    Query {
                        features: Features { porous: false },
                        initial: 0,
                        actions,
                    },
                    "order",
                )
                .unwrap()
                .outcome
        })
        .collect();
        assert_ne!(outcomes[0], outcomes[1]);
    }
}
