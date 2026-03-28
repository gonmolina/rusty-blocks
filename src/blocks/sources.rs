use super::{Block, BlockRegistry};
use serde::Deserialize;
use serde_json::Value;

/// A block that outputs a constant value.
pub struct Constant {
    value: Vec<f64>,
}

impl Constant {
    pub fn new(value: Vec<f64>) -> Self {
        Self { value }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { value: Vec<f64> }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.value)))
    }
}

impl Block for Constant {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 0 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _port: usize) -> usize { 0 }
    fn output_width(&self, _port: usize) -> usize { self.value.len() }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}

    fn outputs(&self, _t: f64, _x: &[f64], _u: &[&[f64]], y: &mut [&mut [f64]]) {
        y[0].copy_from_slice(&self.value);
    }

    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}

/// A block that outputs a step signal (scalar).
pub struct Step {
    initial_value: f64,
    final_value: f64,
    step_time: f64,
}

impl Step {
    pub fn new(initial_value: f64, final_value: f64, step_time: f64) -> Self {
        Self { initial_value, final_value, step_time }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { initial_value: f64, final_value: f64, step_time: f64 }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.initial_value, p.final_value, p.step_time)))
    }
}

impl Block for Step {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 0 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _port: usize) -> usize { 0 }
    fn output_width(&self, _port: usize) -> usize { 1 }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}

    fn outputs(&self, t: f64, _x: &[f64], _u: &[&[f64]], y: &mut [&mut [f64]]) {
        y[0][0] = if t < self.step_time { self.initial_value } else { self.final_value };
    }

    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}
