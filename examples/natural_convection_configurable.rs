use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D, Sum};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- BLOQUES ---
    
    // Header Inferior (Configurable: Monitoreamos P y T en nodo 0)
    let h_bottom = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![0], vec![0])));
    
    // Tubería Ascendente (5 celdas, monitoreamos T en la celda central 2)
    let pipe_up = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.05, -5.0, thermo.clone(), vec![2], vec![])));
    
    // Header Superior
    let h_top = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![0], vec![0])));
    
    // Tubería Descendente
    let pipe_down = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.05, 5.0, thermo.clone(), vec![], vec![])));

    // Señales de Control
    let valve_open = system.add_block(Box::new(Constant::new(vec![1.0])));
    let q_heat = system.add_block(Box::new(Constant::new(vec![10000.0])));
    let q_cool = system.add_block(Box::new(Constant::new(vec![-10000.0])));
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Sumadores
    let sum_w_top = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_top = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_w_bot = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_bot = system.add_block(Box::new(Sum::new("+-", 1))); 

    // --- CONEXIONES ---

    // 1. Tubería Ascendente
    // Inputs Pipe: [p_in, h_in, p_out, h_out, valve, q]
    // Header Outputs: [SelectedTemps(0), SelectedPress(1), Enthalpy(2), Density(3)]
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
    // Pipe1D Outputs: [Temps(0), Press(1), W_all(2), WH_all(3), W_in(4), W_out(5), WH_in(6), WH_out(7)]
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
    let t_final = 200.0;
    let sync_dt = 1.0;

    println!("Simulación Sincronizada con Salidas Configurables...");
    println!("t [s], T_bottom_0 [C], T_pipe_up_2 [C], W_up_in [kg/s]");

    let steps = (t_final / sync_dt) as usize;
    for step_idx in 1..=steps {
        let target_t = step_idx as f64 * sync_dt;
        while solver.t < target_t - 1e-10 {
            let h = suggested_dt.min(target_t - solver.t);
            suggested_dt = solver.step_rk45(&system, h, 1e-4, 1e-3);
        }

        let y_bot = &solver.get_outputs()[solver.get_y_offset(h_bottom)..];
        let y_pipe = &solver.get_outputs()[solver.get_y_offset(pipe_up)..];
        
        let t_bot = y_bot[0]; // Port 0, index 0 (T de nodo 0)
        let t_pipe_mid = y_pipe[0]; // Port 0, index 0 (T de nodo 2, único seleccionado)
        
        // El puerto 4 de Pipe1D es W_in
        let w_in = y_pipe[pipe_up_w_in_offset(&system, pipe_up)];

        println!("{:.1}, {:.2}, {:.2}, {:.4}", solver.t, t_bot - 273.15, t_pipe_mid - 273.15, w_in);
    }
}

fn pipe_up_w_in_offset(system: &System, id: usize) -> usize {
    let block = &system.blocks[id];
    let mut offset = 0;
    for p in 0..4 { offset += block.output_width(p); }
    offset
}
