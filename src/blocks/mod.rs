pub mod continuous;
pub mod math;
pub mod ports;
pub mod routing;
pub mod sources;

pub use continuous::Integrator;
pub use math::Gain;
pub use ports::{InPort, OutPort};
pub use routing::Mux;
pub use sources::Constant;

use std::collections::HashMap;
use serde_json::Value;

/// Signature for a function that creates a block from JSON parameters.
/// Now includes a reference to the registry to support recursive building (Subsystems).
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
    
    fn derivatives(&self, t: f64, x: &[f64], u: &[&[f64]], dx: &mut [f64]);
    fn outputs(&self, t: f64, x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]);
    
    fn has_direct_feedthrough(&self) -> bool;
    fn get_initial_conditions(&self, x: &mut [f64]);

    // Helper methods for Subsystems
    fn is_in_port(&self) -> bool { false }
    fn is_out_port(&self) -> bool { false }
    fn downcast_ref_inport(&self) -> Option<&InPort> { None }
    fn downcast_ref_outport(&self) -> Option<&OutPort> { None }
}
