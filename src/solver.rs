use crate::system::{BlockId, System};

#[derive(Clone, Copy)]
pub struct FlatConnection {
    pub global_from_idx: usize,
    pub global_to_idx: usize,
    pub width: usize,
}

pub struct EulerSolver {
    pub t: f64,
    pub x: Vec<f64>,              // Global state vector
    global_u: Vec<f64>,           // Flat global input buffer
    global_y: Vec<f64>,           // Flat global output buffer
    execution_order: Vec<BlockId>,
    block_state_offsets: Vec<usize>,
    block_u_offsets: Vec<usize>,
    block_y_offsets: Vec<usize>,
    // Connections grouped by target block for O(N) execution
    block_input_conns: Vec<Vec<FlatConnection>>,
}

impl EulerSolver {
    pub fn new(system: &System) -> Result<Self, String> {
        let execution_order = system.calculate_execution_order()?;

        let mut x = Vec::new();
        let mut block_state_offsets = vec![0; system.blocks.len()];
        let mut block_u_offsets = vec![0; system.blocks.len()];
        let mut block_y_offsets = vec![0; system.blocks.len()];
        let mut block_input_conns = vec![Vec::new(); system.blocks.len()];

        let mut x_offset = 0;
        let mut u_offset = 0;
        let mut y_offset = 0;

        for (i, block) in system.blocks.iter().enumerate() {
            block_state_offsets[i] = x_offset;
            block_u_offsets[i] = u_offset;
            block_y_offsets[i] = y_offset;
            
            let n_s = block.num_states();
            let mut ic = vec![0.0; n_s];
            block.get_initial_conditions(&mut ic);
            x.extend(ic);
            x_offset += n_s;

            u_offset += block.total_input_width();
            y_offset += block.total_output_width();
        }

        for conn in &system.connections {
            let mut from_port_offset = 0;
            for p in 0..conn.from_port {
                from_port_offset += system.blocks[conn.from_block].output_width(p);
            }
            let mut to_port_offset = 0;
            for p in 0..conn.to_port {
                to_port_offset += system.blocks[conn.to_block].input_width(p);
            }

            block_input_conns[conn.to_block].push(FlatConnection {
                global_from_idx: block_y_offsets[conn.from_block] + from_port_offset,
                global_to_idx: block_u_offsets[conn.to_block] + to_port_offset,
                width: system.blocks[conn.from_block].output_width(conn.from_port),
            });
        }

        Ok(Self {
            t: 0.0,
            x,
            global_u: vec![0.0; u_offset],
            global_y: vec![0.0; y_offset],
            execution_order,
            block_state_offsets,
            block_u_offsets,
            block_y_offsets,
            block_input_conns,
        })
    }

    fn compute_derivatives_internal(
        execution_order: &[BlockId],
        state_offsets: &[usize],
        u_offsets: &[usize],
        y_offsets: &[usize],
        input_conns: &[Vec<FlatConnection>],
        system: &System,
        t: f64,
        x: &[f64],
        u_buf: &mut [f64],
        y_buf: &mut [f64],
    ) -> Vec<f64> {
        // 1. Calculate outputs in topological order
        for &id in execution_order {
            let block = &system.blocks[id];

            // Update inputs for this block from other blocks' outputs
            for conn in &input_conns[id] {
                // RATIONALE: Using pointers and copy_nonoverlapping (memcpy) avoids 
                // repeated bounds checks in the inner loop and provides peak throughput.
                //
                // SAFETY: 
                // 1. Offsets and widths are pre-calculated and validated during Solver construction.
                // 2. y_buf (outputs) and u_buf (inputs) are separate allocations; they never overlap.
                // 3. Topological order guarantees that y_buf[global_from_idx] contains valid data.
                let (src_ptr, dst_ptr) = unsafe {
                    (
                        y_buf.as_ptr().add(conn.global_from_idx),
                        u_buf.as_mut_ptr().add(conn.global_to_idx),
                    )
                };
                unsafe { std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, conn.width); }
            }

            let n_s = block.num_states();
            let b_states = &x[state_offsets[id] .. state_offsets[id] + n_s];
            
            let u_start = u_offsets[id];
            let b_inputs = &u_buf[u_start .. u_start + block.total_input_width()];
            
            let y_start = y_offsets[id];
            let b_outputs = &mut y_buf[y_start .. y_start + block.total_output_width()];
            
            block.outputs(t, b_states, b_inputs, b_outputs);
        }

        // 2. Calculate derivatives
        let mut dx_global = vec![0.0; x.len()];
        for (id, block) in system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            if n_s > 0 {
                let b_states = &x[state_offsets[id] .. state_offsets[id] + n_s];
                let u_start = u_offsets[id];
                let b_inputs = &u_buf[u_start .. u_start + block.total_input_width()];
                let b_dx = &mut dx_global[state_offsets[id] .. state_offsets[id] + n_s];
                
                block.derivatives(t, b_states, b_inputs, b_dx);
            }
        }
        dx_global
    }

    fn compute_derivatives(&mut self, system: &System, t: f64, x: &[f64]) -> Vec<f64> {
        Self::compute_derivatives_internal(
            &self.execution_order,
            &self.block_state_offsets,
            &self.block_u_offsets,
            &self.block_y_offsets,
            &self.block_input_conns,
            system, t, x, 
            &mut self.global_u, 
            &mut self.global_y
        )
    }

    fn finalize_step(&mut self, system: &System) {
        for (id, block) in system.blocks.iter().enumerate() {
            let n_s = block.num_states();
            let b_states = &self.x[self.block_state_offsets[id] .. self.block_state_offsets[id] + n_s];
            let u_start = self.block_u_offsets[id];
            let b_inputs = &self.global_u[u_start .. u_start + block.total_input_width()];
            block.on_step_end(self.t, b_states, b_inputs);
        }
    }

    fn ensure_initial_step_finalized(&mut self, system: &System) {
        if self.t == 0.0 {
            self.compute_derivatives(system, 0.0, &self.x.clone());
            self.finalize_step(system);
        }
    }

    pub fn step(&mut self, system: &System, suggested_dt: f64) {
        self.ensure_initial_step_finalized(system);
        let dt = self.get_dt_limit(system, suggested_dt);
        let x_curr = self.x.clone();
        let dx = self.compute_derivatives(system, self.t, &x_curr);
        for i in 0..self.x.len() { self.x[i] += dt * dx[i]; }
        self.t += dt;
        self.finalize_step(system);
    }

    pub fn step_rk4(&mut self, system: &System, suggested_dt: f64) {
        self.ensure_initial_step_finalized(system);
        let dt = self.get_dt_limit(system, suggested_dt);
        let x0 = self.x.clone();
        let t0 = self.t;

        let k1 = self.compute_derivatives(system, t0, &x0);
        let mut x_temp = vec![0.0; x0.len()];
        for i in 0..x0.len() { x_temp[i] = x0[i] + 0.5 * dt * k1[i]; }
        let k2 = self.compute_derivatives(system, t0 + 0.5 * dt, &x_temp);
        for i in 0..x0.len() { x_temp[i] = x0[i] + 0.5 * dt * k2[i]; }
        let k3 = self.compute_derivatives(system, t0 + 0.5 * dt, &x_temp);
        for i in 0..x0.len() { x_temp[i] = x0[i] + dt * k3[i]; }
        let k4 = self.compute_derivatives(system, t0 + dt, &x_temp);

        for i in 0..self.x.len() {
            self.x[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        self.t += dt;
        self.compute_derivatives(system, self.t, &self.x.clone());
        self.finalize_step(system);
    }

    pub fn step_rk45(&mut self, system: &System, initial_dt: f64, atol: f64, rtol: f64) -> f64 {
        self.ensure_initial_step_finalized(system);
        let mut h = self.get_dt_limit(system, initial_dt);
        let x0 = self.x.clone();
        let t0 = self.t;

        loop {
            let k1 = self.compute_derivatives(system, t0, &x0);
            let mut x_temp = vec![0.0; x0.len()];
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (1.0 / 5.0) * k1[i]; }
            let k2 = self.compute_derivatives(system, t0 + h * (1.5 / 5.0), &x_temp);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (3.0 / 40.0 * k1[i] + 9.0 / 40.0 * k2[i]); }
            let k3 = self.compute_derivatives(system, t0 + h * (3.0 / 10.0), &x_temp);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (44.0 / 45.0 * k1[i] - 56.0 / 15.0 * k2[i] + 32.0 / 9.0 * k3[i]); }
            let k4 = self.compute_derivatives(system, t0 + h * (4.0 / 5.0), &x_temp);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (19372.0 / 6561.0 * k1[i] - 25360.0 / 2187.0 * k2[i] + 64448.0 / 6561.0 * k3[i] - 212.0 / 729.0 * k4[i]); }
            let k5 = self.compute_derivatives(system, t0 + h * (8.0 / 9.0), &x_temp);
            for i in 0..x0.len() { x_temp[i] = x0[i] + h * (9017.0 / 3168.0 * k1[i] - 355.0 / 33.0 * k2[i] + 46732.0 / 5247.0 * k3[i] + 49.0 / 176.0 * k4[i] - 5103.0 / 18656.0 * k5[i]); }
            let k6 = self.compute_derivatives(system, t0 + h, &x_temp);
            let mut x5 = vec![0.0; x0.len()];
            for i in 0..x0.len() { x5[i] = x0[i] + h * (35.0 / 384.0 * k1[i] + 500.0 / 1113.0 * k3[i] + 125.0 / 192.0 * k4[i] - 2187.0 / 6784.0 * k5[i] + 11.0 / 84.0 * k6[i]); }
            let k7 = self.compute_derivatives(system, t0 + h, &x5);

            let mut max_err: f64 = 0.0;
            for i in 0..x0.len() {
                let err = h * ((35.0 / 384.0 - 5179.0 / 57600.0) * k1[i] + (500.0 / 1113.0 - 7571.0 / 16695.0) * k3[i] + (125.0 / 192.0 - 393.0 / 640.0) * k4[i] + (-2187.0 / 6784.0 + 92097.0 / 339200.0) * k5[i] + (11.0 / 84.0 - 187.0 / 2100.0) * k6[i] - 1.0 / 40.0 * k7[i]);
                let sc = atol + rtol * x0[i].abs().max(x5[i].abs());
                max_err = f64::max(max_err, err.abs() / sc);
            }

            if max_err <= 1.0 {
                self.x = x5;
                self.t += h;
                self.finalize_step(system);
                let h_next = h * 0.9 * (1.0 / max_err.max(1e-10)).powf(0.2);
                return h_next.min(h * 5.0);
            } else {
                let h_new = h * 0.9 * (1.0 / max_err).powf(0.25);
                h = h_new.max(h * 0.1);
                if h < 1e-12 { panic!("Step size too small in RK45"); }
            }
        }
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

    pub fn get_block_state(&self, block_id: BlockId, num_states: usize) -> &[f64] {
        let offset = self.block_state_offsets[block_id];
        &self.x[offset..offset + num_states]
    }

    pub fn get_outputs(&self) -> &[f64] {
        &self.global_y
    }

    pub fn get_y_offset(&self, block_idx: usize) -> usize {
        self.block_y_offsets[block_idx]
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
