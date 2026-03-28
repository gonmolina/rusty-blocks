pub mod continuous;
pub mod math;
pub mod ports;
pub mod routing;
pub mod sources;

pub use continuous::Integrator;
pub use math::{Gain, Sum};
pub use ports::{InPort, OutPort};
pub use routing::{Demux, Mux};
pub use sources::{Constant, Step};

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
