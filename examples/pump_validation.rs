use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D, Sum, CentrifugalPump};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- PARÁMETROS DE LA BOMBA ---
    // Curva: dP [Pa] = 0.2222 * RPM^2 - 200 * W^2
    // A 1500 RPM: 5 bar a caudal cero, 0 bar a 50 kg/s.
    let coeffs = [0.2222, 0.0, -200.0];
    let n_nominal = 1500.0;
    let rho_nominal = 1000.0;
    let geom_inertia = 0.001; // Inercia mucho mayor para estabilizar la curva empinada
    let fric_pasivo = 0.5;

    // --- BLOQUES ---
    
    // Headers grandes (100 m3) para amortiguar el arranque
    let h_suction = system.add_block(Box::new(Header::new(100.0, thermo.clone(), vec![0], vec![0])));
    let h_discharge = system.add_block(Box::new(Header::new(100.0, thermo.clone(), vec![0], vec![0])));

    // La Bomba
    let pump = system.add_block(Box::new(CentrifugalPump::new(n_nominal, rho_nominal, coeffs, geom_inertia, fric_pasivo)));
    
    // Tubería de Retorno (para cerrar el lazo y crear resistencia)
    // 5m, 0.1m diámetro, con una válvula para variar la carga
    let pipe_return = system.add_block(Box::new(Pipe1D::new(1, 5.0, 0.1, 0.0, thermo.clone(), vec![], vec![])));

    // Señales de Control
    let pump_speed = system.add_block(Box::new(Constant::new(vec![1500.0]))); // 1500 RPM
    let valve_pos = system.add_block(Box::new(Constant::new(vec![0.5])));     // 50% abierta
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Sumadores para balances
    let sum_w_suction = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_suction = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_w_discharge = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_discharge = system.add_block(Box::new(Sum::new("+-", 1))); 

    // --- CONEXIONES ---

    // 1. Conexiones de la Bomba
    // Inputs: [p_in, h_in, p_out, h_out, speed, rho]
    system.connect(h_suction, 1, pump, 0);    // P_suction
    system.connect(h_suction, 2, pump, 1);    // H_suction
    system.connect(h_discharge, 1, pump, 2);  // P_discharge
    system.connect(h_discharge, 2, pump, 3);  // H_discharge
    system.connect(pump_speed, 0, pump, 4);
    system.connect(q_zero, 0, pump, 5);       // No rho override

    // 2. Conexiones de la Tubería de Retorno
    system.connect(h_discharge, 1, pipe_return, 0); 
    system.connect(h_discharge, 2, pipe_return, 1);
    system.connect(h_suction, 1, pipe_return, 2);
    system.connect(h_suction, 2, pipe_return, 3);
    system.connect(valve_pos, 0, pipe_return, 4);
    system.connect(q_zero, 0, pipe_return, 5);

    // 3. Balance en Header Suction (Entra Retorno, Sale Bomba)
    system.connect(pipe_return, 5, sum_w_suction, 0);  // W_out_pipe (+)
    system.connect(pump, 4, sum_w_suction, 1);         // W_in_pump (-)
    system.connect(sum_w_suction, 0, h_suction, 0);
    
    system.connect(pipe_return, 7, sum_wh_suction, 0); // WH_out_pipe (+)
    system.connect(pump, 6, sum_wh_suction, 1);        // WH_in_pump (-)
    system.connect(sum_wh_suction, 0, h_suction, 1);
    system.connect(q_zero, 0, h_suction, 2);

    // 4. Balance en Header Discharge (Entra Bomba, Sale Retorno)
    system.connect(pump, 5, sum_w_discharge, 0);       // W_out_pump (+)
    system.connect(pipe_return, 4, sum_w_discharge, 1); // W_in_pipe (-)
    system.connect(sum_w_discharge, 0, h_discharge, 0);
    
    system.connect(pump, 7, sum_wh_discharge, 0);      // WH_out_pump (+)
    system.connect(pipe_return, 6, sum_wh_discharge, 1); // WH_in_pipe (-)
    system.connect(sum_wh_discharge, 0, h_discharge, 1);
    system.connect(q_zero, 0, h_discharge, 2);

    // --- SIMULACIÓN SINCRONIZADA ---
    let mut solver = EulerSolver::new(&system).unwrap();
    let mut suggested_dt: f64 = 0.01; 
    let t_final = 50.0;
    let sync_dt = 1.0;

    println!("Simulación de Bomba Centrífuga (Curva Realista)...");
    println!("t [s], Speed [RPM], Flow [kg/s], P_suction [bar], P_discharge [bar], dP [bar]");

    let steps = (t_final / sync_dt) as usize;
    for step_idx in 1..=steps {
        let target_t = step_idx as f64 * sync_dt;
        while solver.t < target_t - 1e-10 {
            let h = suggested_dt.min(target_t - solver.t);
            suggested_dt = solver.step_rk45(&system, h, 1e-4, 1e-3);
        }

        let y_suction = &solver.get_outputs()[solver.get_y_offset(h_suction)..];
        let y_discharge = &solver.get_outputs()[solver.get_y_offset(h_discharge)..];
        let y_pump = &solver.get_outputs()[solver.get_y_offset(pump)..];
        
        let p_suc = y_suction[1] / 1e5;
        let p_dis = y_discharge[1] / 1e5;
        let flow = y_pump[2]; // W_in de la bomba (Port 2 o 4)

        println!("{:.1}, {:.0}, {:.4}, {:.2}, {:.2}, {:.2}", 
                 solver.t, 1500.0, flow, p_suc, p_dis, p_dis - p_suc);
    }
}
