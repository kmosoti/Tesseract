//! Small local density states and completely positive trace-preserving channels.
use nalgebra::{DMatrix, linalg::SymmetricEigen};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use tesseract_contracts::{Error, OUTCOMES, Result};

pub const EPS: f64 = 1e-9;
pub type Matrix = DMatrix<Complex64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "DensityWire")]
pub struct Density {
    matrix: Matrix,
}
#[derive(Deserialize)]
struct DensityWire {
    matrix: Matrix,
}
impl TryFrom<DensityWire> for Density {
    type Error = Error;
    fn try_from(value: DensityWire) -> Result<Self> {
        Self::new(value.matrix)
    }
}
impl Density {
    pub fn new(matrix: Matrix) -> Result<Self> {
        if matrix.shape() != (OUTCOMES, OUTCOMES)
            || matrix
                .iter()
                .any(|v| !v.re.is_finite() || !v.im.is_finite() || v.norm() > 1.0 + EPS)
        {
            return Err(Error::Invalid(
                "density must be finite and four-dimensional".into(),
            ));
        }
        if (&matrix - matrix.adjoint()).norm() > EPS
            || (matrix.trace() - Complex64::new(1.0, 0.0)).norm() > EPS
        {
            return Err(Error::Invalid(
                "density must be Hermitian and unit trace".into(),
            ));
        }
        let eigen = SymmetricEigen::new(matrix.clone());
        if eigen
            .eigenvalues
            .iter()
            .any(|x| !x.is_finite() || *x < -EPS)
        {
            return Err(Error::Invalid(
                "density must be positive semidefinite".into(),
            ));
        }
        Ok(Self { matrix })
    }
    pub fn basis(outcome: u8) -> Result<Self> {
        if outcome as usize >= OUTCOMES {
            return Err(Error::Invalid("invalid basis outcome".into()));
        }
        let mut matrix = Matrix::zeros(OUTCOMES, OUTCOMES);
        matrix[(outcome as usize, outcome as usize)] = Complex64::new(1.0, 0.0);
        Ok(Self { matrix })
    }
    pub fn matrix(&self) -> &Matrix {
        &self.matrix
    }
    pub fn probabilities(&self) -> Result<[f64; OUTCOMES]> {
        let p = std::array::from_fn(|i| self.matrix[(i, i)].re);
        if p.iter()
            .any(|x| !x.is_finite() || *x < -EPS || *x > 1.0 + EPS)
            || (p.iter().sum::<f64>() - 1.0).abs() > EPS
        {
            return Err(Error::Invalid("invalid density readout".into()));
        }
        // Remove only floating-point boundary noise, not invalid channel output.
        let mut p = p.map(|v| v.clamp(0.0, 1.0));
        let total: f64 = p.iter().sum();
        for x in &mut p {
            *x /= total;
        }
        Ok(p)
    }
    /// A projective observation is an instrument, not a trace-preserving channel.
    pub fn condition(&self, outcome: u8) -> Result<(Self, f64)> {
        if outcome as usize >= OUTCOMES {
            return Err(Error::Invalid("invalid observed outcome".into()));
        }
        let probability = self.probabilities()?[outcome as usize];
        if probability <= EPS {
            return Err(Error::Invalid(
                "cannot condition on zero-probability evidence".into(),
            ));
        }
        Ok((Self::basis(outcome)?, probability))
    }
}

/// Vertically stacked Kraus operators form an isometry W†W=I.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "ChannelWire")]
pub struct Channel {
    stacked: Matrix,
}
#[derive(Deserialize)]
struct ChannelWire {
    stacked: Matrix,
}
impl TryFrom<ChannelWire> for Channel {
    type Error = Error;
    fn try_from(value: ChannelWire) -> Result<Self> {
        let channel = Self {
            stacked: value.stacked,
        };
        channel.validate()?;
        Ok(channel)
    }
}
impl Channel {
    pub fn from_stacked(mut matrix: Matrix) -> Result<Self> {
        if matrix.ncols() != OUTCOMES
            || matrix.nrows() < OUTCOMES
            || matrix.nrows() > OUTCOMES.pow(3)
            || !matrix.nrows().is_multiple_of(OUTCOMES)
            || matrix
                .iter()
                .any(|v| !v.re.is_finite() || !v.im.is_finite())
        {
            return Err(Error::Invalid(
                "invalid stacked Kraus dimensions or coefficients".into(),
            ));
        }
        // Twice-reorthogonalized modified Gram-Schmidt: thin QR with fixed phase.
        for col in 0..OUTCOMES {
            for _ in 0..2 {
                for previous in 0..col {
                    let dot: Complex64 = (0..matrix.nrows())
                        .map(|r| matrix[(r, previous)].conj() * matrix[(r, col)])
                        .sum();
                    for r in 0..matrix.nrows() {
                        let old = matrix[(r, previous)];
                        matrix[(r, col)] -= old * dot;
                    }
                }
            }
            let norm = matrix.column(col).norm();
            if !norm.is_finite() || norm < EPS {
                return Err(Error::Invalid("rank-deficient Kraus isometry".into()));
            }
            for r in 0..matrix.nrows() {
                matrix[(r, col)] /= norm;
            }
        }
        let channel = Self { stacked: matrix };
        channel.validate()?;
        Ok(channel)
    }
    pub fn validate(&self) -> Result<()> {
        let w = &self.stacked;
        if w.ncols() != OUTCOMES
            || w.nrows() < OUTCOMES
            || w.nrows() > OUTCOMES.pow(3)
            || !w.nrows().is_multiple_of(OUTCOMES)
            || w.iter()
                .any(|x| !x.re.is_finite() || !x.im.is_finite() || x.norm() > 1.0 + EPS)
        {
            return Err(Error::Invalid("Kraus operators are not complete".into()));
        }
        let residual = (w.adjoint() * w - Matrix::identity(OUTCOMES, OUTCOMES)).norm();
        if !residual.is_finite() || residual > EPS {
            return Err(Error::Invalid(
                "Kraus completeness residual is invalid".into(),
            ));
        }
        Ok(())
    }
    pub fn from_transition(rows: &[[f64; OUTCOMES]; OUTCOMES]) -> Result<Self> {
        // K_ab = sqrt(P(a|b)) |a><b| embeds a classical Markov operator exactly.
        let mut w = Matrix::zeros(OUTCOMES.pow(3), OUTCOMES);
        for (input, row) in rows.iter().enumerate() {
            if row.iter().any(|v| !v.is_finite() || *v < 0.0)
                || (row.iter().sum::<f64>() - 1.0).abs() > EPS
            {
                return Err(Error::Invalid(
                    "transition row is not a distribution".into(),
                ));
            }
            for (output, p) in row.iter().enumerate() {
                let k = input * OUTCOMES + output;
                w[(k * OUTCOMES + output, input)] = Complex64::new(p.sqrt(), 0.0);
            }
        }
        Self::from_stacked(w)
    }
    pub fn apply(&self, state: &Density) -> Density {
        let mut result = Matrix::zeros(OUTCOMES, OUTCOMES);
        for k in 0..self.stacked.nrows() / OUTCOMES {
            let operator = self.stacked.rows(k * OUTCOMES, OUTCOMES);
            result += operator * state.matrix() * operator.adjoint();
        }
        Density { matrix: result }
    }
    pub fn stacked(&self) -> &Matrix {
        &self.stacked
    }
    pub fn has_complex_coefficients(&self) -> bool {
        self.stacked.iter().any(|v| v.im.abs() > EPS)
    }
    /// Average the channel with coefficient conjugation in the fixed public
    /// basis. The real Choi matrix is independent of the Kraus representation.
    pub fn real_coefficient_ablation(&self) -> Result<Self> {
        let dimension = OUTCOMES * OUTCOMES;
        let mut choi = DMatrix::<f64>::zeros(dimension, dimension);
        for k in 0..self.stacked.nrows() / OUTCOMES {
            for i in 0..dimension {
                for j in 0..dimension {
                    let a = self.stacked[(k * OUTCOMES + i % OUTCOMES, i / OUTCOMES)];
                    let b = self.stacked[(k * OUTCOMES + j % OUTCOMES, j / OUTCOMES)];
                    choi[(i, j)] += (a * b.conj()).re;
                }
            }
        }
        let eigen = SymmetricEigen::new(choi);
        if eigen
            .eigenvalues
            .iter()
            .any(|v| !v.is_finite() || *v < -EPS)
        {
            return Err(Error::Invalid("ablated Choi matrix is not positive".into()));
        }
        let mut stacked = Matrix::zeros(OUTCOMES.pow(3), OUTCOMES);
        for k in 0..dimension {
            for i in 0..dimension {
                stacked[(k * OUTCOMES + i % OUTCOMES, i / OUTCOMES)] = Complex64::new(
                    eigen.eigenvectors[(i, k)] * eigen.eigenvalues[k].max(0.0).sqrt(),
                    0.0,
                );
            }
        }
        Self::from_stacked(stacked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_properties_hold_for_composed_complex_channels() {
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        let mut rng = ChaCha8Rng::seed_from_u64(12);
        for _ in 0..24 {
            let w = Matrix::from_fn(8, 4, |_, _| {
                Complex64::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0))
            });
            let channel = Channel::from_stacked(w).unwrap();
            let mut state = Density::basis(0).unwrap();
            for _ in 0..8 {
                state = channel.apply(&state);
                Density::new(state.matrix().clone()).unwrap();
            }
            let p = state.probabilities().unwrap();
            let (conditioned, likelihood) = state.condition(0).unwrap();
            assert_eq!(likelihood, p[0]);
            assert_eq!(conditioned.probabilities().unwrap(), [1.0, 0.0, 0.0, 0.0]);
        }
    }
    #[test]
    fn invalid_states_and_isometries_are_rejected() {
        assert!(Channel::from_stacked(Matrix::zeros(8, 4)).is_err());
        assert!(Density::new(Matrix::identity(4, 4)).is_err());
        let mut m = Matrix::zeros(4, 4);
        m[(0, 0)] = Complex64::new(-1.0, 0.0);
        m[(1, 1)] = Complex64::new(2.0, 0.0);
        assert!(Density::new(m).is_err());
        assert!(Density::basis(0).unwrap().condition(1).is_err());
    }
    #[test]
    fn classical_transition_embedding_is_exact() {
        let rows = [[0.1, 0.2, 0.3, 0.4]; 4];
        let channel = Channel::from_transition(&rows).unwrap();
        for i in 0..4 {
            let p = channel
                .apply(&Density::basis(i).unwrap())
                .probabilities()
                .unwrap();
            for (a, b) in p.iter().zip(rows[i as usize]) {
                assert!((a - b).abs() < EPS);
            }
        }
    }
    #[test]
    fn phase_ablation_is_invariant_to_a_kraus_global_phase() {
        let a = Channel::from_stacked(Matrix::identity(4, 4)).unwrap();
        let b = Channel::from_stacked(Matrix::identity(4, 4) * Complex64::new(0.0, 1.0)).unwrap();
        for i in 0..4 {
            let state = Density::basis(i).unwrap();
            assert!(
                (a.real_coefficient_ablation()
                    .unwrap()
                    .apply(&state)
                    .matrix()
                    - b.real_coefficient_ablation()
                        .unwrap()
                        .apply(&state)
                        .matrix())
                .norm()
                    < EPS
            );
        }
    }
    #[test]
    fn deserialization_cannot_bypass_numeric_constructors() {
        let invalid = serde_json::json!({"matrix":Matrix::zeros(1,1)});
        assert!(serde_json::from_value::<Density>(invalid).is_err());
        let invalid = serde_json::json!({"stacked":Matrix::zeros(8,4)});
        assert!(serde_json::from_value::<Channel>(invalid).is_err());
    }
    #[test]
    fn huge_finite_coefficients_cannot_hide_invalidity_through_nan() {
        let mut density = Matrix::identity(4, 4) * Complex64::new(0.25, 0.0);
        density[(0, 1)] = Complex64::new(1e308, 1e308);
        density[(1, 0)] = density[(0, 1)].conj();
        assert!(Density::new(density).is_err());
        let invalid =
            serde_json::json!({"stacked":Matrix::from_element(4,4,Complex64::new(1e308,1e308))});
        assert!(serde_json::from_value::<Channel>(invalid).is_err());
        assert!(
            Channel::from_stacked(Matrix::from_element(8, 4, Complex64::new(1e308, 1e308)))
                .is_err()
        );
    }
}
