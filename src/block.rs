use std::cell::RefCell;

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

/// A simple Constant Gain block.
pub struct Gain {
    k: f64,
    width: usize,
}

impl Gain {
    pub fn new(k: f64, width: usize) -> Self {
        Self { k, width }
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

/// A block that outputs a constant value.
pub struct Constant {
    value: Vec<f64>,
}

impl Constant {
    pub fn new(value: Vec<f64>) -> Self {
        Self { value }
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

/// A block that sums its inputs. All inputs must have the same width.
pub struct Sum {
    signs: String,
    width: usize,
}

impl Sum {
    pub fn new(signs: &str, width: usize) -> Self {
        Self { signs: signs.to_string(), width }
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
