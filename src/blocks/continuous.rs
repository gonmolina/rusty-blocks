use super::Block;

/// An Integrator block.
pub struct Integrator {
    initial_conditions: Vec<f64>,
}

impl Integrator {
    pub fn new(initial_conditions: Vec<f64>) -> Self {
        Self { initial_conditions }
    }
}

impl Block for Integrator {
    fn num_states(&self) -> usize { self.initial_conditions.len() }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _port: usize) -> usize { self.initial_conditions.len() }
    fn output_width(&self, _port: usize) -> usize { self.initial_conditions.len() }

    fn derivatives(&self, _t: f64, _x: &[f64], u: &[&[f64]], dx: &mut [f64]) {
        dx.copy_from_slice(u[0]);
    }

    fn outputs(&self, _t: f64, x: &[f64], _u: &[&[f64]], y: &mut [&mut [f64]]) {
        y[0].copy_from_slice(x);
    }

    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, x: &mut [f64]) {
        x.copy_from_slice(&self.initial_conditions);
    }
}
