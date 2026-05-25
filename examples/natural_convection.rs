use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D, Sum};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- BLOQUES ---
    
    // Headers de 2.0 m3
    let h_bottom = system.add_block(Box::new(Header::new(2.0, thermo.clone())));
    let h_top = system.add_block(Box::new(Header::new(2.0, thermo.clone())));
    
    // Tuberías de 5m
    let pipe_up = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.05, -5.0, thermo.clone())));
    let pipe_down = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.05, 5.0, thermo.clone())));

    // Señales de Control
    let valve_open = system.add_block(Box::new(Constant::new(vec![1.0])));
    let q_heat = system.add_block(Box::new(Constant::new(vec![10000.0]))); // 10 kW
    let q_cool = system.add_block(Box::new(Constant::new(vec![-10000.0]))); // -10 kW
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Sumadores
    let sum_w_top = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_top = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_w_bot = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_bot = system.add_block(Box::new(Sum::new("+-", 1))); 

    // --- CONEXIONES ---
    system.connect(h_bottom, 0, pipe_up, 0); 
    system.connect(h_bottom, 1, pipe_up, 1); 
    system.connect(h_top, 0, pipe_up, 2);    
    system.connect(h_top, 1, pipe_up, 3);    
    system.connect(valve_open, 0, pipe_up, 4);
    system.connect(q_heat, 0, pipe_up, 5);

    system.connect(h_top, 0, pipe_down, 0);    
    system.connect(h_top, 1, pipe_down, 1);    
    system.connect(h_bottom, 0, pipe_down, 2); 
    system.connect(h_bottom, 1, pipe_down, 3); 
    system.connect(valve_open, 0, pipe_down, 4);
    system.connect(q_cool, 0, pipe_down, 5);

    system.connect(pipe_up, 1, sum_w_top, 0);    
    system.connect(pipe_down, 0, sum_w_top, 1);  
    system.connect(sum_w_top, 0, h_top, 0);      
    system.connect(pipe_up, 3, sum_wh_top, 0);   
    system.connect(pipe_down, 2, sum_wh_top, 1); 
    system.connect(sum_wh_top, 0, h_top, 1);     
    system.connect(q_zero, 0, h_top, 2);         

    system.connect(pipe_down, 1, sum_w_bot, 0);  
    system.connect(pipe_up, 0, sum_w_bot, 1);    
    system.connect(sum_w_bot, 0, h_bottom, 0);   
    system.connect(pipe_down, 3, sum_wh_bot, 0); 
    system.connect(pipe_up, 2, sum_wh_bot, 1);   
    system.connect(sum_wh_bot, 0, h_bottom, 1);  
    system.connect(q_zero, 0, h_bottom, 2);      

    // --- SIMULACIÓN SINCRONIZADA (RK45 Sub-stepping) ---
    let mut solver = EulerSolver::new(&system).unwrap();
    let mut suggested_dt: f64 = 0.01; 
    let t_final = 2000.0;
    let sync_dt = 0.1; // Sincronismo de datos cada 0.1s

    println!("Simulación RK45 con Sincronismo cada {}s...", sync_dt);
    println!("t [s], Flow (W_up), T_bottom [C], T_top [C], n_substeps");

    let total_sync_steps = (t_final / sync_dt) as usize;

    for step_idx in 1..=total_sync_steps {
        let target_t = step_idx as f64 * sync_dt;
        let mut sub_steps = 0;
        
        while solver.t < target_t - 1e-10 {
            let h = suggested_dt.min(target_t - solver.t);
            suggested_dt = solver.step_rk45(&system, h, 1e-4, 1e-3);
            sub_steps += 1;
        }

        // Reporte cada 100s (cada 1000 pasos de sincronismo)
        if step_idx % 1000 == 0 {
            let x_up = solver.get_block_state(pipe_up, 16); 
            let x_bot = solver.get_block_state(h_bottom, 2);
            let x_top = solver.get_block_state(h_top, 2);
            let t_bot = x_bot[1] / x_bot[0] / 4184.0 + 20.0;
            let t_top = x_top[1] / x_top[0] / 4184.0 + 20.0;
            println!("{:.1}, {:.4}, {:.2}, {:.2}, {}", solver.t, x_up[0], t_bot, t_top, sub_steps);
        }
    }
}
