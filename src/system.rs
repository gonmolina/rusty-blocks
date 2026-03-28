use crate::block::{Block, Constant, Demux, Gain, InPort, Integrator, Mux, OutPort, Step, Sum};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::cell::RefCell;

pub type BlockId = usize;
pub type PortId = usize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum BlockConfig {
    Gain { k: f64, width: usize },
    Integrator { ic: Vec<f64> },
    Constant { value: Vec<f64> },
    Step { initial_value: f64, final_value: f64, step_time: f64 },
    Sum { signs: String, width: usize },
    Mux { input_widths: Vec<usize> },
    Demux { output_widths: Vec<usize> },
    InPort { width: usize },
    OutPort { width: usize },
    Subsystem(SystemConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub from: String,
    pub from_port: PortId,
    pub to: String,
    pub to_port: PortId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub name: String,
    pub blocks: Vec<BlockJson>,
    pub connections: Vec<ConnectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockJson {
    pub id: String,
    #[serde(flatten)]
    pub config: BlockConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Connection {
    pub from_block: BlockId,
    pub from_port: PortId,
    pub to_block: BlockId,
    pub to_port: PortId,
}

/// A System represents a collection of blocks and their connections.
pub struct System {
    pub blocks: Vec<Box<dyn Block>>,
    pub connections: Vec<Connection>,
}

impl System {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn add_block(&mut self, block: Box<dyn Block>) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(block);
        id
    }

    pub fn from_config(config: SystemConfig) -> Self {
        let mut system = Self::new();
        let mut id_map = HashMap::new();

        for b_json in config.blocks {
            let block: Box<dyn Block> = match b_json.config {
                BlockConfig::Gain { k, width } => Box::new(Gain::new(k, width)),
                BlockConfig::Integrator { ic } => Box::new(Integrator::new(ic)),
                BlockConfig::Constant { value } => Box::new(Constant::new(value)),
                BlockConfig::Step { initial_value, final_value, step_time } => 
                    Box::new(Step::new(initial_value, final_value, step_time)),
                BlockConfig::Sum { signs, width } => Box::new(Sum::new(&signs, width)),
                BlockConfig::Mux { input_widths } => Box::new(Mux::new(input_widths)),
                BlockConfig::Demux { output_widths } => Box::new(Demux::new(output_widths)),
                BlockConfig::InPort { width } => Box::new(InPort::new(width)),
                BlockConfig::OutPort { width } => Box::new(OutPort::new(width)),
                BlockConfig::Subsystem(sub_config) => Box::new(Subsystem::from_config(sub_config)),
            };
            let internal_id = system.add_block(block);
            id_map.insert(b_json.id, internal_id);
        }

        for conn in config.connections {
            let from_id = *id_map.get(&conn.from).expect("Source block not found");
            let to_id = *id_map.get(&conn.to).expect("Target block not found");
            system.connect(from_id, conn.from_port, to_id, conn.to_port);
        }

        system
    }

    pub fn connect(&mut self, from_block: BlockId, from_port: PortId, to_block: BlockId, to_port: PortId) {
        assert!(from_block < self.blocks.len());
        assert!(to_block < self.blocks.len());
        let from_width = self.blocks[from_block].output_width(from_port);
        let to_width = self.blocks[to_block].input_width(to_port);
        assert_eq!(from_width, to_width, "Width mismatch at connection");
        self.connections.push(Connection { from_block, from_port, to_block, to_port });
    }

    pub fn calculate_execution_order(&self) -> Result<Vec<BlockId>, String> {
        let n = self.blocks.len();
        let mut adj = vec![Vec::new(); n];
        let mut in_degree = vec![0; n];
        for conn in &self.connections {
            if self.blocks[conn.to_block].has_direct_feedthrough() {
                adj[conn.from_block].push(conn.to_block);
                in_degree[conn.to_block] += 1;
            }
        }
        let mut queue = VecDeque::new();
        for (i, el) in in_degree.iter().enumerate() { if *el == 0 { queue.push_back(i); } }
        let mut order = Vec::new();
        while let Some(u) = queue.pop_front() {
            order.push(u);
            for &v in &adj[u] { in_degree[v] -= 1; if in_degree[v] == 0 { queue.push_back(v); } }
        }
        if order.len() < n { return Err("Algebraic loop detected!".to_string()); }
        Ok(order)
    }
}

pub struct Subsystem {
    pub system: System,
    execution_order: Vec<BlockId>,
    block_state_offsets: Vec<usize>,
    num_states: usize,
    in_port_block_ids: Vec<BlockId>,
    out_port_block_ids: Vec<BlockId>,
    has_direct_feedthrough: bool,
    internal_outputs: RefCell<Vec<Vec<Vec<f64>>>>,
    internal_inputs: RefCell<Vec<Vec<Vec<f64>>>>,
}

impl Subsystem {
    pub fn from_config(config: SystemConfig) -> Self {
        let system = System::from_config(config);
        let execution_order = system.calculate_execution_order().expect("Algebraic loop in subsystem");
        let mut block_state_offsets = vec![0; system.blocks.len()];
        let mut current_offset = 0;
        let mut in_port_block_ids = Vec::new();
        let mut out_port_block_ids = Vec::new();
        let mut internal_outputs = Vec::new();
        let mut internal_inputs = Vec::new();

        for (i, block) in system.blocks.iter().enumerate() {
            block_state_offsets[i] = current_offset;
            current_offset += block.num_states();
            if block.is_in_port() { in_port_block_ids.push(i); }
            if block.is_out_port() { out_port_block_ids.push(i); }
            let mut b_outputs = Vec::new();
            for p in 0..block.num_outputs() { b_outputs.push(vec![0.0; block.output_width(p)]); }
            internal_outputs.push(b_outputs);
            let mut b_inputs = Vec::new();
            for p in 0..block.num_inputs() { b_inputs.push(vec![0.0; block.input_width(p)]); }
            internal_inputs.push(b_inputs);
        }

        let has_direct_feedthrough = Self::calculate_direct_feedthrough(&system, &in_port_block_ids, &out_port_block_ids);

        Self {
            system,
            execution_order,
            block_state_offsets,
            num_states: current_offset,
            in_port_block_ids,
            out_port_block_ids,
            has_direct_feedthrough,
            internal_outputs: RefCell::new(internal_outputs),
            internal_inputs: RefCell::new(internal_inputs),
        }
    }

    fn calculate_direct_feedthrough(system: &System, in_ports: &[BlockId], out_ports: &[BlockId]) -> bool {
        let n = system.blocks.len();
        let mut adj = vec![Vec::new(); n];
        for conn in &system.connections {
            if system.blocks[conn.to_block].has_direct_feedthrough() {
                adj[conn.from_block].push(conn.to_block);
            }
        }

        for &start_node in in_ports {
            let mut visited = vec![false; n];
            let mut stack = vec![start_node];
            while let Some(u) = stack.pop() {
                if visited[u] { continue; }
                visited[u] = true;
                if out_ports.contains(&u) { return true; }
                for &v in &adj[u] { stack.push(v); }
            }
        }
        false
    }

    fn update_internal_signals(&self, t: f64, x: &[f64], u: &[&[f64]]) {
        let mut outputs = self.internal_outputs.borrow_mut();
        let mut inputs = self.internal_inputs.borrow_mut();

        for (i, &block_id) in self.in_port_block_ids.iter().enumerate() {
            let in_port = self.system.blocks[block_id].downcast_ref_inport().unwrap();
            in_port.value.borrow_mut().copy_from_slice(u[i]);
        }

        for &id in &self.execution_order {
            let block = &self.system.blocks[id];
            for conn in &self.system.connections {
                if conn.to_block == id {
                    let source_data = &outputs[conn.from_block][conn.from_port];
                    inputs[conn.to_block][conn.to_port].copy_from_slice(source_data);
                }
            }
            let n_s = block.num_states();
            let offset = self.block_state_offsets[id];
            let b_states = &x[offset..offset + n_s];
            let u_refs: Vec<&[f64]> = inputs[id].iter().map(|v| v.as_slice()).collect();
            let mut y_refs: Vec<&mut [f64]> = outputs[id].iter_mut().map(|v| v.as_mut_slice()).collect();
            block.outputs(t, b_states, &u_refs, &mut y_refs);
        }
    }
}

impl Block for Subsystem {
    fn num_states(&self) -> usize { self.num_states }
    fn num_inputs(&self) -> usize { self.in_port_block_ids.len() }
    fn num_outputs(&self) -> usize { self.out_port_block_ids.len() }

    fn input_width(&self, port: usize) -> usize {
        let block_id = self.in_port_block_ids[port];
        self.system.blocks[block_id].output_width(0)
    }
    fn output_width(&self, port: usize) -> usize {
        let block_id = self.out_port_block_ids[port];
        self.system.blocks[block_id].input_width(0)
    }

    fn derivatives(&self, t: f64, x: &[f64], u: &[&[f64]], dx: &mut [f64]) {
        self.update_internal_signals(t, x, u);
        let inputs = self.internal_inputs.borrow();
        for (id, block) in self.system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            if n_s > 0 {
                let offset = self.block_state_offsets[id];
                let b_states = &x[offset..offset + n_s];
                let u_refs: Vec<&[f64]> = inputs[id].iter().map(|v| v.as_slice()).collect();
                let mut b_dx = vec![0.0; n_s];
                block.derivatives(t, b_states, &u_refs, &mut b_dx);
                dx[offset..offset + n_s].copy_from_slice(&b_dx);
            }
        }
    }

    fn outputs(&self, t: f64, x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]) {
        self.update_internal_signals(t, x, u);
        for (i, &block_id) in self.out_port_block_ids.iter().enumerate() {
            let out_port = self.system.blocks[block_id].downcast_ref_outport().unwrap();
            y[i].copy_from_slice(&out_port.value.borrow());
        }
    }

    fn has_direct_feedthrough(&self) -> bool { self.has_direct_feedthrough }
    fn get_initial_conditions(&self, x: &mut [f64]) {
        for (i, block) in self.system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            if n_s > 0 {
                let offset = self.block_state_offsets[i];
                block.get_initial_conditions(&mut x[offset..offset + n_s]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Gain, Integrator};

    #[test]
    fn test_execution_order_simple_chain() {
        let mut system = System::new();
        let g1 = system.add_block(Box::new(Gain::new(2.0, 1)));
        let i1 = system.add_block(Box::new(Integrator::new(vec![0.0])));
        let g2 = system.add_block(Box::new(Gain::new(0.5, 1)));
        system.connect(i1, 0, g2, 0);
        system.connect(g2, 0, g1, 0);
        let order = system.calculate_execution_order().unwrap();
        assert_eq!(order, vec![1, 2, 0]);
    }

    #[test]
    fn test_subsystem_direct_feedthrough() {
        // Subsystem 1: InPort -> Gain -> OutPort (Should have direct feedthrough)
        let sub_config_direct = SystemConfig {
            name: "Direct".to_string(),
            blocks: vec![
                BlockJson { id: "in".to_string(), config: BlockConfig::InPort { width: 1 } },
                BlockJson { id: "gain".to_string(), config: BlockConfig::Gain { k: 2.0, width: 1 } },
                BlockJson { id: "out".to_string(), config: BlockConfig::OutPort { width: 1 } },
            ],
            connections: vec![
                ConnectionConfig { from: "in".to_string(), from_port: 0, to: "gain".to_string(), to_port: 0 },
                ConnectionConfig { from: "gain".to_string(), from_port: 0, to: "out".to_string(), to_port: 0 },
            ],
        };
        let sub_direct = Subsystem::from_config(sub_config_direct);
        assert!(sub_direct.has_direct_feedthrough());

        // Subsystem 2: InPort -> Integrator -> OutPort (Should NOT have direct feedthrough)
        let sub_config_indirect = SystemConfig {
            name: "Indirect".to_string(),
            blocks: vec![
                BlockJson { id: "in".to_string(), config: BlockConfig::InPort { width: 1 } },
                BlockJson { id: "int".to_string(), config: BlockConfig::Integrator { ic: vec![0.0] } },
                BlockJson { id: "out".to_string(), config: BlockConfig::OutPort { width: 1 } },
            ],
            connections: vec![
                ConnectionConfig { from: "in".to_string(), from_port: 0, to: "int".to_string(), to_port: 0 },
                ConnectionConfig { from: "int".to_string(), from_port: 0, to: "out".to_string(), to_port: 0 },
            ],
        };
        let sub_indirect = Subsystem::from_config(sub_config_indirect);
        assert!(!sub_indirect.has_direct_feedthrough());
    }
}
