use super::{Block, BlockRegistry};
use serde::Deserialize;
use serde_json::Value;

/// Multiplexer: Combines multiple input ports into one output port.
pub struct Mux {
    input_widths: Vec<usize>,
    total_width: usize,
}

impl Mux {
    pub fn new(input_widths: Vec<usize>) -> Self {
        let total_width = input_widths.iter().sum();
        Self { input_widths, total_width }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { input_widths: Vec<usize> }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.input_widths)))
    }
}

impl Block for Mux {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { self.input_widths.len() }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, port: usize) -> usize { self.input_widths[port] }
    fn output_width(&self, _port: usize) -> usize { self.total_width }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}

    fn outputs(&self, _t: f64, _x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]) {
        let mut offset = 0;
        for i in 0..self.input_widths.len() {
            let w = self.input_widths[i];
            y[0][offset..offset + w].copy_from_slice(u[i]);
            offset += w;
        }
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}

/// Demultiplexer: Splits one input port into multiple output ports.
pub struct Demux {
    output_widths: Vec<usize>,
    total_input_width: usize,
}

impl Demux {
    pub fn new(output_widths: Vec<usize>) -> Self {
        let total_input_width = output_widths.iter().sum();
        Self { output_widths, total_input_width }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { output_widths: Vec<usize> }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.output_widths)))
    }
}

impl Block for Demux {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { self.output_widths.len() }
    fn input_width(&self, _port: usize) -> usize { self.total_input_width }
    fn output_width(&self, port: usize) -> usize { self.output_widths[port] }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}

    fn outputs(&self, _t: f64, _x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]) {
        let mut offset = 0;
        for i in 0..self.output_widths.len() {
            let w = self.output_widths[i];
            y[i].copy_from_slice(&u[0][offset..offset + w]);
            offset += w;
        }
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}
