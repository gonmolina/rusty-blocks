use crate::blocks::{Block, BlockRegistry};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::cell::RefCell;
use serde_json::Value;

pub type BlockId = usize;
pub type PortId = usize;

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
    pub r#type: String,
    pub params: Value,
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

    pub fn from_config(config: SystemConfig, registry: &BlockRegistry) -> Self {
        let mut system = Self::new();
        let mut id_map = HashMap::new();

        for b_json in config.blocks {
            let block = registry.build(&b_json.r#type, b_json.params)
                .expect(&format!("Error building block {}: {}", b_json.id, b_json.r#type));
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

    /// Determines the execution order of blocks using Kahn's algorithm for topological sorting.
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

struct InternalFlatConnection {
    pub from_y_idx: usize,
    pub to_u_idx: usize,
    pub width: usize,
}

/// A Subsystem is a specialized Block that encapsulates a complete internal System.
pub struct Subsystem {
    pub system: System,
    execution_order: Vec<BlockId>,
    block_state_offsets: Vec<usize>,
    block_u_offsets: Vec<usize>,
    block_y_offsets: Vec<usize>,
    num_states: usize,
    in_port_block_ids: Vec<BlockId>,
    out_port_block_ids: Vec<BlockId>,
    has_direct_feedthrough: bool,
    internal_u: RefCell<Vec<f64>>,
    internal_y: RefCell<Vec<f64>>,
    internal_connections: Vec<InternalFlatConnection>,
}

impl Subsystem {
    pub fn from_config(config: SystemConfig, registry: &BlockRegistry) -> Self {
        let system = System::from_config(config, registry);
        let execution_order = system.calculate_execution_order().expect("Algebraic loop in subsystem");
        let mut block_state_offsets = vec![0; system.blocks.len()];
        let mut block_u_offsets = vec![0; system.blocks.len()];
        let mut block_y_offsets = vec![0; system.blocks.len()];
        let mut in_port_block_ids = Vec::new();
        let mut out_port_block_ids = Vec::new();
        let mut x_off = 0; let mut u_off = 0; let mut y_off = 0;
        for (i, block) in system.blocks.iter().enumerate() {
            block_state_offsets[i] = x_off;
            block_u_offsets[i] = u_off;
            block_y_offsets[i] = y_off;
            x_off += block.num_states();
            u_off += block.total_input_width();
            y_off += block.total_output_width();
            if block.is_in_port() { in_port_block_ids.push(i); }
            if block.is_out_port() { out_port_block_ids.push(i); }
        }
        let mut internal_connections = Vec::new();
        for conn in &system.connections {
            let mut from_port_offset = 0;
            for p in 0..conn.from_port { from_port_offset += system.blocks[conn.from_block].output_width(p); }
            let mut to_port_offset = 0;
            for p in 0..conn.to_port { to_port_offset += system.blocks[conn.to_block].input_width(p); }
            internal_connections.push(InternalFlatConnection {
                from_y_idx: block_y_offsets[conn.from_block] + from_port_offset,
                to_u_idx: block_u_offsets[conn.to_block] + to_port_offset,
                width: system.blocks[conn.from_block].output_width(conn.from_port),
            });
        }
        let has_direct_feedthrough = Self::calculate_direct_feedthrough(&system, &in_port_block_ids, &out_port_block_ids);
        Self {
            system,
            execution_order,
            block_state_offsets,
            block_u_offsets,
            block_y_offsets,
            num_states: x_off,
            in_port_block_ids,
            out_port_block_ids,
            has_direct_feedthrough,
            internal_u: RefCell::new(vec![0.0; u_off]),
            internal_y: RefCell::new(vec![0.0; y_off]),
            internal_connections,
        }
    }

    pub fn build(v: Value, registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        let config: SystemConfig = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::from_config(config, registry)))
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

    fn update_internal_signals(&self, t: f64, x: &[f64], u: &[f64]) {
        let mut u_buf = self.internal_u.borrow_mut();
        let mut y_buf = self.internal_y.borrow_mut();
        let mut u_offset = 0;
        for &block_id in &self.in_port_block_ids {
            let block = &self.system.blocks[block_id];
            let w = block.total_output_width();
            let y_start = self.block_y_offsets[block_id];
            let in_port = block.downcast_ref_inport().unwrap();
            in_port.value.borrow_mut().copy_from_slice(&u[u_offset..u_offset + w]);
            y_buf[y_start..y_start + w].copy_from_slice(&u[u_offset..u_offset + w]);
            u_offset += w;
        }
        for &id in &self.execution_order {
            let block = &self.system.blocks[id];
            // Resolve connections to this block
            for conn in &self.internal_connections {
                if conn.to_u_idx >= self.block_u_offsets[id] && conn.to_u_idx < self.block_u_offsets[id] + block.total_input_width() {
                    // RATIONALE: Direct pointer movement avoids expensive bounds checking in 
                    // deeply nested subsystems and matches the global solver optimization.
                    //
                    // SAFETY:
                    // 1. Internal offsets are pre-calculated during Subsystem construction and match widths.
                    // 2. internal_u and internal_y are separate vectors (no overlap).
                    // 3. Topological execution order ensures signal source validity.
                    let (src_ptr, dst_ptr) = unsafe { 
                        (
                            y_buf.as_ptr().add(conn.from_y_idx), 
                            u_buf.as_mut_ptr().add(conn.to_u_idx)
                        ) 
                    };
                    unsafe { std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, conn.width); }
                }
            }
            let n_s = block.num_states();
            let b_states = &x[self.block_state_offsets[id] .. self.block_state_offsets[id] + n_s];
            let b_inputs = &u_buf[self.block_u_offsets[id] .. self.block_u_offsets[id] + block.total_input_width()];
            let b_outputs = &mut y_buf[self.block_y_offsets[id] .. self.block_y_offsets[id] + block.total_output_width()];
            block.outputs(t, b_states, b_inputs, b_outputs);
        }
    }
}

impl Block for Subsystem {
    fn num_states(&self) -> usize { self.num_states }
    fn num_inputs(&self) -> usize { self.in_port_block_ids.len() }
    fn num_outputs(&self) -> usize { self.out_port_block_ids.len() }
    fn input_width(&self, port: usize) -> usize { self.system.blocks[self.in_port_block_ids[port]].total_output_width() }
    fn output_width(&self, port: usize) -> usize { self.system.blocks[self.out_port_block_ids[port]].total_input_width() }
    fn derivatives(&self, t: f64, x: &[f64], u: &[f64], dx: &mut [f64]) {
        self.update_internal_signals(t, x, u);
        let u_buf = self.internal_u.borrow();
        for (id, block) in self.system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            if n_s > 0 {
                let b_states = &x[self.block_state_offsets[id] .. self.block_state_offsets[id] + n_s];
                let b_inputs = &u_buf[self.block_u_offsets[id] .. self.block_u_offsets[id] + block.total_input_width()];
                let b_dx = &mut dx[self.block_state_offsets[id] .. self.block_state_offsets[id] + n_s];
                block.derivatives(t, b_states, b_inputs, b_dx);
            }
        }
    }
    fn outputs(&self, t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        self.update_internal_signals(t, x, u);
        let u_buf = self.internal_u.borrow();
        let mut y_offset = 0;
        for &block_id in &self.out_port_block_ids {
            let block = &self.system.blocks[block_id];
            let w = block.total_input_width();
            let u_start = self.block_u_offsets[block_id];
            y[y_offset..y_offset + w].copy_from_slice(&u_buf[u_start..u_start + w]);
            y_offset += w;
        }
    }
    fn has_direct_feedthrough(&self) -> bool { self.has_direct_feedthrough }
    fn get_initial_conditions(&self, x: &mut [f64]) {
        for (i, block) in self.system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            if n_s > 0 { block.get_initial_conditions(&mut x[self.block_state_offsets[i]..self.block_state_offsets[i] + n_s]); }
        }
    }
    fn next_event(&self, t: f64) -> Option<f64> { self.system.blocks.iter().filter_map(|b| b.next_event(t)).min_by(|a, b| a.partial_cmp(b).unwrap()) }
    fn on_step_end(&self, t: f64, x: &[f64], u: &[f64]) {
        self.update_internal_signals(t, x, u);
        let u_buf = self.internal_u.borrow();
        for (id, block) in self.system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            let b_states = &x[self.block_state_offsets[id] .. self.block_state_offsets[id] + n_s];
            let b_inputs = &u_buf[self.block_u_offsets[id] .. self.block_u_offsets[id] + block.total_input_width()];
            block.on_step_end(t, b_states, b_inputs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Gain, Integrator, Constant};
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
    fn test_constant_block() {
        let registry = BlockRegistry::std();
        let json_data = r#"{"name": "C", "blocks": [{"id": "c1", "type": "Constant", "params": {"value": [5.0, 6.0]}}], "connections": []}"#;
        let config: SystemConfig = serde_json::from_str(json_data).unwrap();
        let system = System::from_config(config, &registry);
        let mut y_c = vec![0.0, 0.0];
        system.blocks[0].outputs(0.0, &[], &[], &mut y_c);
        assert_eq!(y_c, vec![5.0, 6.0]);
    }
}
