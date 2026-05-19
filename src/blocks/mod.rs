pub mod continuous;
pub mod discrete;
pub mod math;
pub mod ports;
pub mod routing;
pub mod sinks;
pub mod sources;

pub use continuous::Integrator;
pub use discrete::{UnitDelay, DiscreteIntegrator, ZeroOrderHold, DiscreteFilter};
pub use math::{Gain, Sum};
pub use ports::{InPort, OutPort};
pub use routing::{Demux, Mux};
pub use sinks::FileSink;
pub use sources::{Constant, Step};

use std::collections::HashMap;
use serde_json::Value;

/// Signature for a function that creates a block from JSON parameters.
pub type BlockBuilder = fn(Value, &BlockRegistry) -> Result<Box<dyn Block>, String>;

/// A registry that maps block type names to their builder functions.
pub struct BlockRegistry {
    builders: HashMap<String, BlockBuilder>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, builder: BlockBuilder) {
        self.builders.insert(name.to_string(), builder);
    }

    pub fn build(&self, name: &str, params: Value) -> Result<Box<dyn Block>, String> {
        let builder = self.builders.get(name)
            .ok_or_else(|| format!("Unknown block type: {}", name))?;
        builder(params, self)
    }

    /// Creates a registry pre-populated with all standard blocks.
    pub fn std() -> Self {
        let mut r = Self::new();
        r.register("Gain", math::Gain::build);
        r.register("Sum", math::Sum::build);
        r.register("Integrator", continuous::Integrator::build);
        r.register("Constant", sources::Constant::build);
        r.register("Step", sources::Step::build);
        r.register("Mux", routing::Mux::build);
        r.register("Demux", routing::Demux::build);
        r.register("InPort", ports::InPort::build);
        r.register("OutPort", ports::OutPort::build);
        r.register("FileSink", sinks::FileSink::build);
        r.register("Scope", sinks::Scope::build);
        r.register("UnitDelay", discrete::UnitDelay::build);
        r.register("DiscreteIntegrator", discrete::DiscreteIntegrator::build);
        r.register("ZeroOrderHold", discrete::ZeroOrderHold::build);
        r.register("DiscreteFilter", discrete::DiscreteFilter::build);
        r
    }
}

/// Interface for a simulation block, similar to Simulink.
pub trait Block {
    fn num_states(&self) -> usize;
    fn num_inputs(&self) -> usize;
    fn num_outputs(&self) -> usize;
    fn input_width(&self, port: usize) -> usize;
    fn output_width(&self, port: usize) -> usize;
    
    fn total_input_width(&self) -> usize {
        (0..self.num_inputs()).map(|p| self.input_width(p)).sum()
    }
    fn total_output_width(&self) -> usize {
        (0..self.num_outputs()).map(|p| self.output_width(p)).sum()
    }

    fn derivatives(&self, t: f64, x: &[f64], u: &[f64], dx: &mut [f64]);
    fn outputs(&self, t: f64, x: &[f64], u: &[f64], y: &mut [f64]);
    
    fn has_direct_feedthrough(&self) -> bool;
    fn get_initial_conditions(&self, x: &mut [f64]);

    /// Sample time for discrete-time blocks. `None` = continuous.
    fn sample_time(&self) -> Option<f64> { None }
    /// State update for discrete blocks. `x[k+1] = f(t, x[k], u[k])`.
    fn update(&self, _t: f64, _x: &[f64], _u: &[f64]) -> Vec<f64> { vec![] }

    fn next_event(&self, _t: f64) -> Option<f64> { None }
    fn on_step_end(&self, _t: f64, _x: &[f64], _u: &[f64]) {}

    fn is_in_port(&self) -> bool { false }
    fn is_out_port(&self) -> bool { false }
    fn downcast_ref_inport(&self) -> Option<&InPort> { None }
    fn downcast_ref_outport(&self) -> Option<&OutPort> { None }
}
