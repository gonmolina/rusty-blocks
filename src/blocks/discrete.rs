use super::{Block, BlockRegistry};
use serde::Deserialize;
use serde_json::Value;

// ── UnitDelay ────────────────────────────────────────────────────────

pub struct UnitDelay {
    ts: f64,
    width: usize,
    initial_conditions: Vec<f64>,
}

impl UnitDelay {
    pub fn new(ts: f64, width: usize, ic: Vec<f64>) -> Self {
        assert!(ts > 0.0, "UnitDelay requires ts > 0");
        Self { ts, width, initial_conditions: ic }
    }
    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct P { ts: f64, #[serde(default="d1")] width: usize, #[serde(default="d0")] ic: Vec<f64> }
        fn d1()->usize{1} fn d0()->Vec<f64>{vec![0.0]}
        let p: P = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.ts, p.width, p.ic)))
    }
}

impl Block for UnitDelay {
    fn num_states(&self) -> usize { self.width }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _: usize) -> usize { self.width }
    fn output_width(&self, _: usize) -> usize { self.width }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {}
    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) { y.copy_from_slice(x); }
    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, x: &mut [f64]) { x.copy_from_slice(&self.initial_conditions); }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }
    fn update(&self, _t: f64, _x: &[f64], u: &[f64]) -> Vec<f64> { u.to_vec() }
}

// ── DiscreteIntegrator ───────────────────────────────────────────────

pub struct DiscreteIntegrator {
    ts: f64,
    width: usize,
    initial_conditions: Vec<f64>,
}

impl DiscreteIntegrator {
    pub fn new(ts: f64, width: usize, ic: Vec<f64>) -> Self {
        assert!(ts > 0.0);
        Self { ts, width, initial_conditions: ic }
    }
    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct P { ts: f64, #[serde(default="d1")] width: usize, #[serde(default="d0")] ic: Vec<f64> }
        fn d1()->usize{1} fn d0()->Vec<f64>{vec![0.0]}
        let p: P = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.ts, p.width, p.ic)))
    }
}

impl Block for DiscreteIntegrator {
    fn num_states(&self) -> usize { self.width }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _: usize) -> usize { self.width }
    fn output_width(&self, _: usize) -> usize { self.width }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {}
    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) { y.copy_from_slice(x); }
    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, x: &mut [f64]) { x.copy_from_slice(&self.initial_conditions); }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }
    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let mut nx = x.to_vec();
        for i in 0..self.width { nx[i] += self.ts * u[i]; }
        nx
    }
}

// ── ZeroOrderHold ────────────────────────────────────────────────────

pub struct ZeroOrderHold {
    ts: f64,
    width: usize,
}

impl ZeroOrderHold {
    pub fn new(ts: f64, width: usize) -> Self { assert!(ts > 0.0); Self { ts, width } }
    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct P { ts: f64, #[serde(default="d1")] width: usize }
        fn d1()->usize{1}
        let p: P = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.ts, p.width)))
    }
}

impl Block for ZeroOrderHold {
    fn num_states(&self) -> usize { self.width }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _: usize) -> usize { self.width }
    fn output_width(&self, _: usize) -> usize { self.width }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {}
    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) { y.copy_from_slice(x); }
    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, x: &mut [f64]) { x.fill(0.0); }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }
    fn update(&self, _t: f64, _x: &[f64], u: &[f64]) -> Vec<f64> { u.to_vec() }
}

// ── DiscreteFilter ───────────────────────────────────────────────────

pub struct DiscreteFilter {
    ts: f64,
    b: Vec<f64>,
    a: Vec<f64>,
    width: usize,
}

impl DiscreteFilter {
    pub fn new(ts: f64, b: Vec<f64>, a: Vec<f64>, width: usize) -> Self {
        assert!(ts > 0.0 && !b.is_empty());
        Self { ts, b, a, width }
    }
    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct P { ts: f64, b: Vec<f64>, #[serde(default)] a: Vec<f64>, #[serde(default="d1")] width: usize }
        fn d1()->usize{1}
        let p: P = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.ts, p.b, p.a, p.width)))
    }
}

impl Block for DiscreteFilter {
    fn num_states(&self) -> usize {
        let nb = if self.b.len() > 1 { self.b.len() - 1 } else { 0 };
        (nb + self.a.len()) * self.width
    }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _: usize) -> usize { self.width }
    fn output_width(&self, _: usize) -> usize { self.width }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {}

    fn outputs(&self, _t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        let nb_d = if self.b.len() > 1 { self.b.len() - 1 } else { 0 };
        let na = self.a.len();
        for w in 0..self.width {
            let mut val = self.b[0] * u[w];
            for k in 0..nb_d { val += self.b[k+1] * x[w + k * self.width]; }
            let yo = nb_d * self.width;
            for k in 0..na { val -= self.a[k] * x[yo + w + k * self.width]; }
            y[w] = val;
        }
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, x: &mut [f64]) { x.fill(0.0); }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let nb_d = if self.b.len() > 1 { self.b.len() - 1 } else { 0 };
        let na = self.a.len();
        let total = self.num_states();
        let mut nx = vec![0.0; total];

        // shift u delays
        if nb_d > 0 {
            if nb_d > 1 { let cl = (nb_d-1)*self.width; nx[self.width..self.width+cl].copy_from_slice(&x[..cl]); }
            nx[..self.width].copy_from_slice(u);
        }
        // shift y delays
        let yo = nb_d * self.width;
        if na > 0 {
            if na > 1 { let cl = (na-1)*self.width; nx[yo+self.width..yo+self.width+cl].copy_from_slice(&x[yo..yo+cl]); }
            // compute y_new and store
            for w in 0..self.width {
                let mut val = self.b[0] * u[w];
                for k in 0..nb_d { val += self.b[k+1] * x[w + k * self.width]; }
                for k in 0..na { val -= self.a[k] * x[yo + w + k * self.width]; }
                nx[yo + w] = val;
            }
        }
        nx
    }
}
