use super::Block;
use std::cell::RefCell;

/// InPort: Entry point for signals into a subsystem.
pub struct InPort {
    width: usize,
    pub value: RefCell<Vec<f64>>,
}

impl InPort {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            value: RefCell::new(vec![0.0; width]),
        }
    }
}

impl Block for InPort {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 0 }
    fn num_outputs(&self) -> usize { 1 }
    fn input_width(&self, _port: usize) -> usize { 0 }
    fn output_width(&self, _port: usize) -> usize { self.width }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}
    fn outputs(&self, _t: f64, _x: &[f64], _u: &[&[f64]], y: &mut [&mut [f64]]) {
        y[0].copy_from_slice(&self.value.borrow());
    }
    fn has_direct_feedthrough(&self) -> bool { false }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
    fn is_in_port(&self) -> bool { true }
    fn downcast_ref_inport(&self) -> Option<&InPort> { Some(self) }
}

/// OutPort: Exit point for signals from a subsystem.
pub struct OutPort {
    width: usize,
    pub value: RefCell<Vec<f64>>,
}

impl OutPort {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            value: RefCell::new(vec![0.0; width]),
        }
    }
}

impl Block for OutPort {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 0 }
    fn input_width(&self, _port: usize) -> usize { self.width }
    fn output_width(&self, _port: usize) -> usize { 0 }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}
    fn outputs(&self, _t: f64, _x: &[f64], u: &[&[f64]], _y: &mut [&mut [f64]]) {
        self.value.borrow_mut().copy_from_slice(u[0]);
    }
    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
    fn is_out_port(&self) -> bool { true }
    fn downcast_ref_outport(&self) -> Option<&OutPort> { Some(self) }
}
