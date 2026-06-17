use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D, Sum};
use bloques::thermo::LinearWater;
use std::sync::Arc;
use std::fs::File;
use std::io::Write;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- BLOQUES ---
    
    // Header Inferior (1.0 m3, seleccionamos T y P de nodo 0)
    let h_bottom = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![0], vec![0])));
    
    // Tubería Ascendente (5 celdas, longitud 5m, diámetro 0.05m, elevación -5m)
    // Monitoreamos T de la celda central 2
    let pipe_up = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.05, -5.0, thermo.clone(), vec![2], vec![])));
    
    // Header Superior (1.0 m3, seleccionamos T y P de nodo 0)
    let h_top = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![0], vec![0])));
    
    // Tubería Descendente (5 celdas, longitud 5m, diámetro 0.05m, elevación 5m)
    let pipe_down = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.05, 5.0, thermo.clone(), vec![], vec![])));

    // Señales de Control
    let valve_open = system.add_block(Box::new(Constant::new(vec![1.0])));
    let q_heat = system.add_block(Box::new(Constant::new(vec![10000.0]))); // 10 kW
    let q_cool = system.add_block(Box::new(Constant::new(vec![-10000.0]))); // -10 kW
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Sumadores para balances en los Headers
    let sum_w_top = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_top = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_w_bot = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_bot = system.add_block(Box::new(Sum::new("+-", 1))); 

    // --- CONEXIONES ---

    // 1. Tubería Ascendente
    system.connect(h_bottom, 1, pipe_up, 0); // P_bottom
    system.connect(h_bottom, 2, pipe_up, 1); // H_bottom
    system.connect(h_top, 1, pipe_up, 2);    // P_top
    system.connect(h_top, 2, pipe_up, 3);    // H_top
    system.connect(valve_open, 0, pipe_up, 4);
    system.connect(q_heat, 0, pipe_up, 5);

    // 2. Tubería Descendente
    system.connect(h_top, 1, pipe_down, 0);    
    system.connect(h_top, 2, pipe_down, 1);    
    system.connect(h_bottom, 1, pipe_down, 2); 
    system.connect(h_bottom, 2, pipe_down, 3); 
    system.connect(valve_open, 0, pipe_down, 4);
    system.connect(q_cool, 0, pipe_down, 5);

    // 3. Balance en Header Top
    system.connect(pipe_up, 5, sum_w_top, 0);    // w_out_up (+)
    system.connect(pipe_down, 4, sum_w_top, 1);  // w_in_down (-)
    system.connect(sum_w_top, 0, h_top, 0);      // w_net
    
    system.connect(pipe_up, 7, sum_wh_top, 0);   // wh_out_up (+)
    system.connect(pipe_down, 6, sum_wh_top, 1); // wh_in_down (-)
    system.connect(sum_wh_top, 0, h_top, 1);     // wh_net
    system.connect(q_zero, 0, h_top, 2);         

    // 4. Balance en Header Bottom
    system.connect(pipe_down, 5, sum_w_bot, 0);  // w_out_down (+)
    system.connect(pipe_up, 4, sum_w_bot, 1);    // w_in_up (-)
    system.connect(sum_w_bot, 0, h_bottom, 0);   // w_net
    
    system.connect(pipe_down, 7, sum_wh_bot, 0); // wh_out_down (+)
    system.connect(pipe_up, 6, sum_wh_bot, 1);   // wh_in_up (-)
    system.connect(sum_wh_bot, 0, h_bottom, 1);  
    system.connect(q_zero, 0, h_bottom, 2);      

    // --- SIMULACIÓN ---
    let mut solver = EulerSolver::new(&system).unwrap();
    let mut suggested_dt: f64 = 0.01; 
    let t_final = 5000.0;
    let sync_dt = 1.0;

    let mut csv_file = File::create("natural_convection_results.csv").unwrap();
    writeln!(csv_file, "t,m_bottom,m_top,t_bottom,t_top,p_bottom,p_top,w_up_in,w_up_out,w_down_in,w_down_out,rho_up_avg,rho_down_avg").unwrap();

    let steps = (t_final / sync_dt) as usize;
    for step_idx in 1..=steps {
        let target_t = step_idx as f64 * sync_dt;
        while solver.t < target_t - 1e-10 {
            let h = suggested_dt.min(target_t - solver.t);
            suggested_dt = solver.step_rk45(&system, h, 1e-4, 1e-3);
        }

        // Obtener salidas y estados
        let y_bot = &solver.get_outputs()[solver.get_y_offset(h_bottom)..];
        let y_top = &solver.get_outputs()[solver.get_y_offset(h_top)..];
        
        let t_bot = y_bot[0] - 273.15;
        let p_bot = y_bot[1];
        let t_top = y_top[0] - 273.15;
        let p_top = y_top[1];

        let x_bot = solver.get_block_state(h_bottom, 2);
        let x_top = solver.get_block_state(h_top, 2);
        let m_bot = x_bot[0];
        let m_top = x_top[0];

        let y_pipe_up = &solver.get_outputs()[solver.get_y_offset(pipe_up)..];
        let y_pipe_down = &solver.get_outputs()[solver.get_y_offset(pipe_down)..];

        let w_up_in = y_pipe_up[pipe_w_in_offset(&system, pipe_up)];
        let w_up_out = y_pipe_up[pipe_w_out_offset(&system, pipe_up)];
        let w_down_in = y_pipe_down[pipe_w_in_offset(&system, pipe_down)];
        let w_down_out = y_pipe_down[pipe_w_out_offset(&system, pipe_down)];

        // Densidades promedio de las tuberías
        // Pipe1D n_cells = 5. Estados: 6 flows, 5 masses, 5 energies. Total = 16.
        let x_pipe_up = solver.get_block_state(pipe_up, 16);
        let x_pipe_down = solver.get_block_state(pipe_down, 16);
        
        let vol_cell = std::f64::consts::PI * 0.025 * 0.025 * 1.0; // area * dz = pi * 0.025^2 * 1.0
        let m_up_total: f64 = x_pipe_up[6..11].iter().sum();
        let m_down_total: f64 = x_pipe_down[6..11].iter().sum();
        let rho_up_avg = m_up_total / (5.0 * vol_cell);
        let rho_down_avg = m_down_total / (5.0 * vol_cell);

        writeln!(
            csv_file,
            "{:.1},{:.4},{:.4},{:.2},{:.2},{:.2},{:.2},{:.6},{:.6},{:.6},{:.6},{:.4},{:.4}",
            solver.t, m_bot, m_top, t_bot, t_top, p_bot, p_top,
            w_up_in, w_up_out, w_down_in, w_down_out,
            rho_up_avg, rho_down_avg
        ).unwrap();

        if step_idx % 500 == 0 {
            println!("t = {:.1}s: W_up = {:.4} kg/s, T_bot = {:.2} C, T_top = {:.2} C, M_bot = {:.1} kg, M_top = {:.1} kg",
                     solver.t, w_up_in, t_bot, t_top, m_bot, m_top);
        }
    }
}

fn pipe_w_in_offset(system: &System, id: usize) -> usize {
    let block = &system.blocks[id];
    let mut offset = 0;
    for p in 0..4 { offset += block.output_width(p); }
    offset
}

fn pipe_w_out_offset(system: &System, id: usize) -> usize {
    let block = &system.blocks[id];
    let mut offset = 0;
    for p in 0..5 { offset += block.output_width(p); }
    offset
}
