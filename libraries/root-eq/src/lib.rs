use num_traits::{real::Real, NumAssign};

pub trait RootEquation {
    type Scalar;
    fn root(&self, x: Self::Scalar) -> Self::Scalar;
    fn diff(&self, x: Self::Scalar) -> Self::Scalar;
}

pub struct NewtonRaphson<Eq: RootEquation> {
    pub equation: Eq,
    pub tolerance: Eq::Scalar,
    pub max_iterations: usize,
}

impl<Eq: RootEquation<Scalar: Real + NumAssign>> NewtonRaphson<Eq> {
    pub fn solve(&self, mut x: Eq::Scalar) -> Eq::Scalar {
        for _ in 0..self.max_iterations {
            let f = self.equation.root(x);
            let df = self.equation.diff(x);
            let dx = f / df;
            x -= dx;
            if dx.abs() < self.tolerance {
                break;
            }
        }
        x
    }
}
