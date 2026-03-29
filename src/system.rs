use crate::blocks::{Block, BlockRegistry, InPort, OutPort};
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
    ///
    /// In a simulation, a block can only calculate its output if its inputs are already 
    /// known for the current time step. This algorithm identifies the dependencies and 
    /// finds a valid sequence.
    ///
    /// # Direct Feedthrough & Algebraic Loops
    /// A dependency only exists if the target block has *direct feedthrough* (it needs 
    /// its input *now* to produce its output *now*). 
    /// *   **Integrators**: Do NOT create immediate dependencies because their current output 
    ///     depends on their state (the past), not the current input. This breaks loops.
    /// *   **Gains/Sums**: Create immediate dependencies.
    ///
    /// # Kahn's Algorithm Steps
    /// 1. **In-degree Calculation**: Count how many algebraic inputs each block has.
    /// 2. **Initial Queue**: Add all blocks with zero in-degree (blocks that can start 
    ///    executing immediately, like Integrators or Sources).
    /// 3. **Processing**: While the queue is not empty:
    ///    a. Pop a block `u` and add it to the execution order.
    ///    b. For each block `v` connected to `u`'s output:
    ///       - Decrement `v`'s in-degree.
    ///       - If `v`'s in-degree becomes zero, add it to the queue.
    /// 4. **Cycle Detection**: If the final order contains fewer blocks than the system, 
    ///    an **Algebraic Loop** (invalid circular dependency) exists.
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

/// A Subsystem is a specialized Block that encapsulates a complete internal System.
///
/// # Purpose
/// Subsystems allow for hierarchical modeling, enabling the user to:
/// 1. **Modularize**: Break down complex systems into smaller, manageable components.
/// 2. **Reuse**: Define a component once (e.g., a PID controller) and use it multiple times.
/// 3. **Abstract**: Hide internal implementation details from the parent system.
///
/// # Mechanism
/// *   **Interfaces**: It uses `InPort` and `OutPort` blocks as its boundary. Inputs to the 
///     Subsystem block in the parent system are mapped to its internal `InPort` blocks, and 
///     internal `OutPort` signals are exposed as outputs to the parent.
/// *   **State Management**: It aggregates the total number of continuous states of all 
///     its internal blocks. The global solver sees the Subsystem's states as a contiguous 
///     segment of the global state vector.
/// *   **Execution**: During a simulation step, the Subsystem executes its internal blocks 
///     in their own topological order whenever its `outputs()` or `derivatives()` methods 
///     are called by the parent system.
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
    pub fn from_config(config: SystemConfig, registry: &BlockRegistry) -> Self {
        let system = System::from_config(config, registry);
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

    pub fn build(v: Value, registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        let config: SystemConfig = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::from_config(config, registry)))
    }

    /// Determines if the subsystem has direct feedthrough by searching for instantaneous paths.
    ///
    /// An instantaneous path exists if a signal can travel from an `InPort` to an `OutPort` 
    /// passing only through blocks that also have direct feedthrough (e.g., `Gain`, `Sum`).
    ///
    /// # Algorithm
    /// 1. **Graph Construction**: Builds an adjacency list where an edge exists between Block A 
    ///    and Block B only if Block B has `has_direct_feedthrough() == true`. This effectively 
    ///    prunes paths that are broken by stateful blocks (like `Integrator`).
    /// 2. **Reachability Search (DFS)**: For each internal `InPort`, it performs a Depth-First 
    ///    Search through the algebraic graph.
    /// 3. **Terminal Condition**: If the search reaches any internal `OutPort`, the subsystem 
    ///    is marked as having direct feedthrough.
    ///
    /// # Fan-out / Fan-in handling
    /// The algorithm correctly handles multiple connections from a single output or multiple 
    /// inputs to a single block by exploring all possible branches in the adjacency list.
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

    fn next_event(&self, t: f64) -> Option<f64> {
        self.system.blocks.iter()
            .filter_map(|b| b.next_event(t))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    fn on_step_end(&self, t: f64, x: &[f64], u: &[&[f64]]) {
        // We must update internal signals one last time to ensure InPorts and intermediate 
        // blocks have the correct final values for the accepted step.
        self.update_internal_signals(t, x, u);
        
        let inputs = self.internal_inputs.borrow();
        for (id, block) in self.system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            let offset = self.block_state_offsets[id];
            let b_states = &x[offset..offset + n_s];
            let u_refs: Vec<&[f64]> = inputs[id].iter().map(|v| v.as_slice()).collect();
            block.on_step_end(t, b_states, &u_refs);
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
    fn test_subsystem_direct_feedthrough() {
        let registry = BlockRegistry::std();
        // Subsystem 1: InPort -> Gain -> OutPort (Should have direct feedthrough)
        let sub_config_direct = SystemConfig {
            name: "Direct".to_string(),
            blocks: vec![
                BlockJson { id: "in".to_string(), r#type: "InPort".to_string(), params: serde_json::json!({ "width": 1 }) },
                BlockJson { id: "gain".to_string(), r#type: "Gain".to_string(), params: serde_json::json!({ "k": 2.0, "width": 1 }) },
                BlockJson { id: "out".to_string(), r#type: "OutPort".to_string(), params: serde_json::json!({ "width": 1 }) },
            ],
            connections: vec![
                ConnectionConfig { from: "in".to_string(), from_port: 0, to: "gain".to_string(), to_port: 0 },
                ConnectionConfig { from: "gain".to_string(), from_port: 0, to: "out".to_string(), to_port: 0 },
            ],
        };
        let sub_direct = Subsystem::from_config(sub_config_direct, &registry);
        assert!(sub_direct.has_direct_feedthrough());

        // Subsystem 2: InPort -> Integrator -> OutPort (Should NOT have direct feedthrough)
        let sub_config_indirect = SystemConfig {
            name: "Indirect".to_string(),
            blocks: vec![
                BlockJson { id: "in".to_string(), r#type: "InPort".to_string(), params: serde_json::json!({ "width": 1 }) },
                BlockJson { id: "int".to_string(), r#type: "Integrator".to_string(), params: serde_json::json!({ "ic": [0.0] }) },
                BlockJson { id: "out".to_string(), r#type: "OutPort".to_string(), params: serde_json::json!({ "width": 1 }) },
            ],
            connections: vec![
                ConnectionConfig { from: "in".to_string(), from_port: 0, to: "int".to_string(), to_port: 0 },
                ConnectionConfig { from: "int".to_string(), from_port: 0, to: "out".to_string(), to_port: 0 },
            ],
        };
        let sub_indirect = Subsystem::from_config(sub_config_indirect, &registry);
        assert!(!sub_indirect.has_direct_feedthrough());
    }

    #[test]
    fn test_constant_block() {
        let registry = BlockRegistry::std();
        let json_data = r#"
        {
            "name": "Constant Test",
            "blocks": [
                { "id": "c1", "type": "Constant", "params": { "value": [5.0, 6.0] } },
                { "id": "g1", "type": "Gain", "params": { "k": 2.0, "width": 2 } }
            ],
            "connections": [
                { "from": "c1", "from_port": 0, "to": "g1", "to_port": 0 }
            ]
        }
        "#;

        let config: SystemConfig = serde_json::from_str(json_data).unwrap();
        let system = System::from_config(config, &registry);
        
        let mut y_c = vec![vec![0.0, 0.0]];
        let mut y_c_ptr: Vec<&mut [f64]> = y_c.iter_mut().map(|v| v.as_mut_slice()).collect();
        system.blocks[0].outputs(0.0, &[], &[], &mut y_c_ptr);
        assert_eq!(y_c[0], vec![5.0, 6.0]);
    }

    #[test]
    fn test_subsystem_basic() {
        let mut registry = BlockRegistry::std();
        registry.register("Subsystem", Subsystem::build);

        let json_data = r#"
        {
            "name": "Main System",
            "blocks": [
                { "id": "const1", "type": "Constant", "params": { "value": [10.0] } },
                { 
                  "id": "sub1", 
                  "type": "Subsystem", 
                  "params": {
                    "name": "MySubsystem",
                    "blocks": [
                        { "id": "in1", "type": "InPort", "params": { "width": 1 } },
                        { "id": "gain1", "type": "Gain", "params": { "k": 3.0, "width": 1 } },
                        { "id": "out1", "type": "OutPort", "params": { "width": 1 } }
                    ],
                    "connections": [
                        { "from": "in1", "from_port": 0, "to": "gain1", "to_port": 0 },
                        { "from": "gain1", "from_port": 0, "to": "out1", "to_port": 0 }
                    ]
                  }
                }
            ],
            "connections": [
                { "from": "const1", "from_port": 0, "to": "sub1", "to_port": 0 }
            ]
        }
        "#;

        let config: SystemConfig = serde_json::from_str(json_data).unwrap();
        let system = System::from_config(config, &registry);
        
        let mut y_sub = vec![vec![0.0]];
        let mut y_sub_ptr: Vec<&mut [f64]> = y_sub.iter_mut().map(|v| v.as_mut_slice()).collect();
        
        let u_sub = [vec![10.0]];
        let u_sub_refs: Vec<&[f64]> = u_sub.iter().map(|v| v.as_slice()).collect();
        
        system.blocks[1].outputs(0.0, &[], &u_sub_refs, &mut y_sub_ptr);
        assert_eq!(y_sub[0][0], 30.0);
    }
}
