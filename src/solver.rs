use crate::system::{BlockId, Connection, System};

pub struct EulerSolver {
    pub t: f64,
    pub x: Vec<f64>,              // Global state vector
    pub outputs: Vec<Vec<Vec<f64>>>,  // [block][port][signal]
    execution_order: Vec<BlockId>,
    block_state_offsets: Vec<usize>,
    // Pre-indexed connections for each block to avoid O(N^2) search
    block_input_connections: Vec<Vec<Connection>>,
}

impl EulerSolver {
    pub fn new(system: &System) -> Result<Self, String> {
        let execution_order = system.calculate_execution_order()?;

        let mut x = Vec::new();
        let mut block_state_offsets = vec![0; system.blocks.len()];
        let mut outputs = Vec::new();
        let mut block_input_connections = vec![Vec::new(); system.blocks.len()];

        let mut current_offset = 0;
        for (i, block) in system.blocks.iter().enumerate() {
            block_state_offsets[i] = current_offset;
            
            // Init states
            let num_states = block.num_states();
            let mut ic = vec![0.0; num_states];
            block.get_initial_conditions(&mut ic);
            x.extend(ic);
            current_offset += num_states;

            // Init outputs structure
            let mut b_outputs = Vec::new();
            for p in 0..block.num_outputs() {
                b_outputs.push(vec![0.0; block.output_width(p)]);
            }
            outputs.push(b_outputs);
        }

        // --- Pre-index connections ---
        for &conn in &system.connections {
            block_input_connections[conn.to_block].push(conn);
        }

        Ok(Self {
            t: 0.0,
            x,
            outputs,
            execution_order,
            block_state_offsets,
            block_input_connections,
        })
    }

    pub fn set_block_state(&mut self, block_id: BlockId, state: &[f64]) {
        let offset = self.block_state_offsets[block_id];
        self.x[offset..offset + state.len()].copy_from_slice(state);
    }

    pub fn get_block_state(&self, block_id: BlockId, num_states: usize) -> &[f64] {
        let offset = self.block_state_offsets[block_id];
        &self.x[offset..offset + num_states]
    }

    fn compute_derivatives_internal(
        execution_order: &[BlockId],
        block_state_offsets: &[usize],
        block_input_connections: &[Vec<Connection>], // Use pre-indexed connections
        system: &System,
        t: f64,
        x: &[f64],
        outputs: &mut Vec<Vec<Vec<f64>>>,
        inputs: &mut Vec<Vec<Vec<f64>>>,
    ) -> Vec<f64> {
        // 1. Calculate outputs in topological order
        for &id in execution_order {
            let block = &system.blocks[id];

            // Fill inputs using the pre-indexed list for THIS block ONLY
            for conn in &block_input_connections[id] {
                let source_data = &outputs[conn.from_block][conn.from_port];
                inputs[id][conn.to_port].copy_from_slice(source_data);
            }

            let n_s = block.num_states();
            let offset = block_state_offsets[id];
            let b_states = &x[offset..offset + n_s];
            
            let u_refs: Vec<&[f64]> = inputs[id].iter().map(|v| v.as_slice()).collect();
            let mut y_refs: Vec<&mut [f64]> = outputs[id].iter_mut().map(|v| v.as_mut_slice()).collect();
            
            block.outputs(t, b_states, &u_refs, &mut y_refs);
        }

        // 2. Calculate derivatives
        let mut dx_global = vec![0.0; x.len()];
        for (id, block) in system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            if n_s > 0 {
                let offset = block_state_offsets[id];
                let b_states = &x[offset..offset + n_s];
                let mut b_dx = vec![0.0; n_s];
                
                let u_refs: Vec<&[f64]> = inputs[id].iter().map(|v| v.as_slice()).collect();
                block.derivatives(t, b_states, &u_refs, &mut b_dx);
                
                dx_global[offset..offset + n_s].copy_from_slice(&b_dx);
            }
        }
        dx_global
    }

    fn compute_derivatives(&self, system: &System, t: f64, x: &[f64], outputs: &mut Vec<Vec<Vec<f64>>>, inputs: &mut Vec<Vec<Vec<f64>>>) -> Vec<f64> {
        Self::compute_derivatives_internal(
            &self.execution_order, 
            &self.block_state_offsets, 
            &self.block_input_connections, 
            system, t, x, outputs, inputs
        )
    }

    fn create_inputs_buffer(&self, system: &System) -> Vec<Vec<Vec<f64>>> {
        let mut inputs = Vec::new();
        for block in &system.blocks {
            let mut b_inputs = Vec::new();
            for p in 0..block.num_inputs() {
                b_inputs.push(vec![0.0; block.input_width(p)]);
            }
            inputs.push(b_inputs);
        }
        inputs
    }

    fn get_dt_limit(&self, system: &System, suggested_dt: f64) -> f64 {
        let mut dt = suggested_dt;
        for block in &system.blocks {
            if let Some(t_event) = block.next_event(self.t) {
                if t_event > self.t + 1e-10 {
                    dt = dt.min(t_event - self.t);
                }
            }
        }
        dt.max(1e-12)
    }

    fn finalize_step(&mut self, system: &System, inputs: &Vec<Vec<Vec<f64>>>) {
        for (id, block) in system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            let offset = self.block_state_offsets[id];
            let b_states = &self.x[offset..offset + n_s];
            let u_refs: Vec<&[f64]> = inputs[id].iter().map(|v| v.as_slice()).collect();
            block.on_step_end(self.t, b_states, &u_refs);
        }
    }

    fn ensure_initial_step_finalized(&mut self, system: &System) {
        if self.t == 0.0 {
            let mut inputs = self.create_inputs_buffer(system);
            let mut outputs = self.outputs.clone();
            self.compute_derivatives(system, 0.0, &self.x, &mut outputs, &mut inputs);
            self.outputs = outputs;
            self.finalize_step(system, &inputs);
        }
    }

    pub fn step(&mut self, system: &System, suggested_dt: f64) {
        self.ensure_initial_step_finalized(system);
        let dt = self.get_dt_limit(system, suggested_dt);
        let mut inputs = self.create_inputs_buffer(system);
        let mut current_outputs = self.outputs.clone();
        
        let dx = self.compute_derivatives(system, self.t, &self.x, &mut current_outputs, &mut inputs);
        self.outputs = current_outputs;

        for i in 0..self.x.len() {
            self.x[i] += dt * dx[i];
        }
        self.t += dt;
        self.finalize_step(system, &inputs);
    }

    pub fn step_rk4(&mut self, system: &System, suggested_dt: f64) {
        self.ensure_initial_step_finalized(system);
        let dt = self.get_dt_limit(system, suggested_dt);
        let mut inputs = self.create_inputs_buffer(system);
        let mut temp_outputs = self.outputs.clone();

        let x0 = self.x.clone();
        let t0 = self.t;

        let k1 = self.compute_derivatives(system, t0, &x0, &mut temp_outputs, &mut inputs);

        let mut x_temp = vec![0.0; x0.len()];
        for i in 0..x0.len() { x_temp[i] = x0[i] + 0.5 * dt * k1[i]; }
        let k2 = self.compute_derivatives(system, t0 + 0.5 * dt, &x_temp, &mut temp_outputs, &mut inputs);

        for i in 0..x0.len() { x_temp[i] = x0[i] + 0.5 * dt * k2[i]; }
        let k3 = self.compute_derivatives(system, t0 + 0.5 * dt, &x_temp, &mut temp_outputs, &mut inputs);

        for i in 0..x0.len() { x_temp[i] = x0[i] + dt * k3[i]; }
        let k4 = self.compute_derivatives(system, t0 + dt, &x_temp, &mut temp_outputs, &mut inputs);

        for i in 0..self.x.len() {
            self.x[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }

        self.t += dt;
        let mut final_outputs = self.outputs.clone();
        self.compute_derivatives(system, self.t, &self.x, &mut final_outputs, &mut inputs);
        self.outputs = final_outputs;
        self.finalize_step(system, &inputs);
    }

    pub fn step_rk45(&mut self, system: &System, initial_dt: f64, atol: f64, rtol: f64) -> f64 {
        self.ensure_initial_step_finalized(system);
        let mut inputs = self.create_inputs_buffer(system);
        let mut temp_outputs = self.outputs.clone();

        let mut h = self.get_dt_limit(system, initial_dt);
        let x0 = self.x.clone();
        let t0 = self.t;

        loop {
            let k1 = self.compute_derivatives(system, t0, &x0, &mut temp_outputs, &mut inputs);
            let mut x_temp = vec![0.0; x0.len()];
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (1.0 / 5.0) * k1[i]; }
            let k2 = self.compute_derivatives(system, t0 + h * (1.5 / 5.0), &x_temp, &mut temp_outputs, &mut inputs);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (3.0 / 40.0 * k1[i] + 9.0 / 40.0 * k2[i]); }
            let k3 = self.compute_derivatives(system, t0 + h * (3.0 / 10.0), &x_temp, &mut temp_outputs, &mut inputs);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (44.0 / 45.0 * k1[i] - 56.0 / 15.0 * k2[i] + 32.0 / 9.0 * k3[i]); }
            let k4 = self.compute_derivatives(system, t0 + h * (4.0 / 5.0), &x_temp, &mut temp_outputs, &mut inputs);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (19372.0 / 6561.0 * k1[i] - 25360.0 / 2187.0 * k2[i] + 64448.0 / 6561.0 * k3[i] - 212.0 / 729.0 * k4[i]); }
            let k5 = self.compute_derivatives(system, t0 + h * (8.0 / 9.0), &x_temp, &mut temp_outputs, &mut inputs);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (9017.0 / 3168.0 * k1[i] - 355.0 / 33.0 * k2[i] + 46732.0 / 5247.0 * k3[i] + 49.0 / 176.0 * k4[i] - 5103.0 / 18656.0 * k5[i]); }
            let k6 = self.compute_derivatives(system, t0 + h, &x_temp, &mut temp_outputs, &mut inputs);
            let mut x5 = vec![0.0; x0.len()];
            for i in 0..x0.len() { x5[i] = x0[i] + h * (35.0 / 384.0 * k1[i] + 500.0 / 1113.0 * k3[i] + 125.0 / 192.0 * k4[i] - 2187.0 / 6784.0 * k5[i] + 11.0 / 84.0 * k6[i]); }
            let k7 = self.compute_derivatives(system, t0 + h, &x5, &mut temp_outputs, &mut inputs);
            let mut max_err: f64 = 0.0;
            for i in 0..x0.len() {
                let err = h * ((35.0 / 384.0 - 5179.0 / 57600.0) * k1[i] + (500.0 / 1113.0 - 7571.0 / 16695.0) * k3[i] + (125.0 / 192.0 - 393.0 / 640.0) * k4[i] + (-2187.0 / 6784.0 + 92097.0 / 339200.0) * k5[i] + (11.0 / 84.0 - 187.0 / 2100.0) * k6[i] - 1.0 / 40.0 * k7[i]);
                let sc = atol + rtol * x0[i].abs().max(x5[i].abs());
                max_err = f64::max(max_err, err.abs() / sc);
            }

            if max_err <= 1.0 {
                self.x = x5;
                self.t += h;
                self.outputs = temp_outputs;
                self.finalize_step(system, &inputs);
                let h_next = h * 0.9 * (1.0 / max_err.max(1e-10)).powf(0.2);
                return h_next.min(h * 5.0);
            } else {
                let h_new = h * 0.9 * (1.0 / max_err).powf(0.25);
                h = h_new.max(h * 0.1);
                if h < 1e-12 { panic!("Step size too small in RK45"); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Gain, Integrator};
    use crate::system::System;

    #[test]
    fn test_simulation_feedback_loop() {
        let mut system = System::new();
        let g1 = system.add_block(Box::new(Gain::new(-5.0, 1)));
        let i1 = system.add_block(Box::new(Integrator::new(vec![10.0])));
        system.connect(i1, 0, g1, 0);
        system.connect(g1, 0, i1, 0);
        let mut solver = EulerSolver::new(&system).unwrap();
        let dt = 0.001;
        for _ in 0..1000 { solver.step(&system, dt); }
        let final_state = solver.get_block_state(i1, 1)[0];
        assert!(final_state < 0.1);
    }
}
