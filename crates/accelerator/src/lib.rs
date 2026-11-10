//! Experimental realification of complex density channels; CPU remains authoritative.
use tesseract_contracts::{Action, Error, Features, Query, Result};
use tesseract_kernel::{Model, density::Density};

pub const MAX_BATCH: usize = 4096;
/// Row-major matrix for row vectors [Re(rho), Im(rho)] with row-major rho.
/// Builds the full complex superoperator, preserving off-diagonal coherence.
pub fn operators(model: &Model) -> Result<Vec<Vec<f32>>> {
    model.validate()?;
    let Model::Density { channels, .. } = model else {
        return Err(Error::Invalid("GPU probe requires density model".into()));
    };
    let mut result = Vec::new();
    for channel in channels {
        let w = channel.stacked();
        let mut matrix = vec![0f32; 32 * 32];
        for i in 0..4 {
            for j in 0..4 {
                for a in 0..4 {
                    for b in 0..4 {
                        let coefficient = (0..w.nrows() / 4)
                            .map(|k| w[(k * 4 + i, a)] * w[(k * 4 + j, b)].conj())
                            .sum::<num_complex::Complex64>();
                        let input = a * 4 + b;
                        let out = i * 4 + j;
                        matrix[input * 32 + out] = coefficient.re as f32;
                        matrix[input * 32 + 16 + out] = coefficient.im as f32;
                        matrix[(16 + input) * 32 + out] = -coefficient.im as f32;
                        matrix[(16 + input) * 32 + 16 + out] = coefficient.re as f32;
                    }
                }
            }
        }
        result.push(matrix);
    }
    Ok(result)
}
pub fn cpu_program(
    model: &Model,
    features: Features,
    actions: &[Action],
    initials: &[u8],
) -> Result<Vec<[f64; 4]>> {
    if actions.is_empty() || actions.len() > 3 {
        return Err(Error::Budget(
            "action program must have 1..=3 actions".into(),
        ));
    }
    model.validate()?;
    if initials.is_empty() || initials.len() > MAX_BATCH {
        return Err(Error::Budget("batch must be 1..=4096".into()));
    }
    let Model::Density { channels, .. } = model else {
        return Err(Error::Invalid("density model required".into()));
    };
    initials
        .iter()
        .map(|initial| {
            Query {
                features,
                initial: *initial,
                actions: actions.to_vec(),
            }
            .validate()?;
            let mut state = Density::basis(*initial)?;
            for action in actions {
                state = channels[features.index() * 3 + action.index()].apply(&state);
            }
            state.probabilities()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseract_kernel::density::{Channel, Matrix};
    #[test]
    fn realification_matches_nonclassical_complex_channels() {
        let channels = (0..6)
            .map(|channel| {
                let mut w = Matrix::zeros(8, 4);
                for r in 0..8 {
                    for c in 0..4 {
                        let x = (r * 7 + c * 3 + channel + 1) as f64;
                        w[(r, c)] = num_complex::Complex64::new(x.sin(), (x * 0.73).cos());
                    }
                }
                Channel::from_stacked(w).unwrap()
            })
            .collect();
        let model = Model::Density {
            complex: true,
            channels,
        };
        let operators = operators(&model).unwrap();
        for initial in 0..4 {
            let mut state = vec![0f64; 32];
            state[(initial as usize) * 5] = 1.0;
            for (step, action) in [Action::Heat, Action::Press, Action::Cool]
                .iter()
                .enumerate()
            {
                let operator = &operators[action.index()];
                state = (0..32)
                    .map(|j| {
                        (0..32)
                            .map(|i| state[i] * operator[i * 32 + j] as f64)
                            .sum()
                    })
                    .collect();
                let expected = cpu_program(
                    &model,
                    Features { porous: false },
                    &[Action::Heat, Action::Press, Action::Cool][..=step],
                    &[initial],
                )
                .unwrap();
                for outcome in 0..4 {
                    assert!((state[outcome * 5] - expected[0][outcome]).abs() < 1e-6);
                }
            }
        }
    }
}
