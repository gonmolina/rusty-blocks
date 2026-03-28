use super::{Block, BlockRegistry};
use serde::Deserialize;
use serde_json::Value;

/// A simple Constant Gain block.
pub struct Gain {
    k: f64,
    width: usize,
}

impl Gain {
    pub fn new(k: f64, width: usize) -> Self {
        Self { k, width }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { k: f64, width: usize }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.k, p.width)))
    }
}

impl Block for Gain {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _port: usize) -> usize { self.width }
    fn output_width(&self, _port: usize) -> usize { self.width }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}

    fn outputs(&self, _t: f64, _x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]) {
        for i in 0..self.width {
            y[0][i] = u[0][i] * self.k;
        }
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}

/// A block that sums its inputs. All inputs must have the same width.
pub struct Sum {
    signs: String,
    width: usize,
}

impl Sum {
    pub fn new(signs: &str, width: usize) -> Self {
        Self { signs: signs.to_string(), width }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { signs: String, width: usize }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(&p.signs, p.width)))
    }
}

impl Block for Sum {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { self.signs.len() }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _port: usize) -> usize { self.width }
    fn output_width(&self, _port: usize) -> usize { self.width }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}

    fn outputs(&self, _t: f64, _x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]) {
        for j in 0..self.width {
            let mut sum = 0.0;
            for (i, sign) in self.signs.chars().enumerate() {
                match sign {
                    '+' => sum += u[i][j],
                    '-' => sum -= u[i][j],
                    _ => {}
                }
            }
            y[0][j] = sum;
        }
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}
