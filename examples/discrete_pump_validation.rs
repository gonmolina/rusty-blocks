use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, DiscreteHeader, DiscretePipe1D, Sum, DiscreteCentrifugalPump};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);
    let ts = 0.01; // 10 ms sample time

    // --- PARÁMETROS DE LA BOMBA ---
    // Curva: dP [Pa] = 0.2222 * RPM^2 - 200 * W^2
    // A 1500 RPM: 5 bar a caudal cero, 0 bar a 50 kg/s.
    // NOTE: coeffs[2] is explicit in DiscreteCentrifugalPump. Since dt is small (10 ms)
    // and internal volume is small, it remains stable.
    let coeffs = [0.2222, 0.0, -200.0];
    let geom_inertia = 0.001; 
    let fric_pasivo = 0.5;

    // --- BLOQUES ---
    
    // Header de descarga (0.05 m3)
    let h_discharge = system.add_block(Box::new(DiscreteHeader::new(0.05, thermo.clone(), vec![0], vec![0], ts, 0.9)));

    // Referencias constantes de la succión (Reservorio/Atmósfera a 1.0 bar y 20 °C)
    let p_suction_ref = system.add_block(Box::new(Constant::new(vec![1e5])));
    let h_suction_ref = system.add_block(Box::new(Constant::new(vec![83700.0])));
    
    // La Bomba
    let pump = system.add_block(Box::new(DiscreteCentrifugalPump::new(coeffs, geom_inertia, fric_pasivo, ts)));
    
    // Tubería de Retorno (para cerrar el lazo y crear resistencia)
    let pipe_return = system.add_block(Box::new(DiscretePipe1D::new(1, 5.0, 0.1, 0.0, thermo.clone(), vec![], vec![], ts)));

    // Señales de Control
    let pump_speed = system.add_block(Box::new(Constant::new(vec![1500.0]))); // 1500 RPM
    let valve_pos = system.add_block(Box::new(Constant::new(vec![0.5])));     // 50% abierta
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Sumadores para balance en el Header de descarga
    let sum_w_discharge = system.add_block(Box::new(Sum::new("+-", 1))); 
    let sum_wh_discharge = system.add_block(Box::new(Sum::new("+-", 1))); 

    // --- CONEXIONES ---

    // 1. Conexiones de la Bomba
    system.connect(p_suction_ref, 0, pump, 0);
    system.connect(h_suction_ref, 0, pump, 1);
    system.connect(h_discharge, 1, pump, 2);
    system.connect(h_discharge, 2, pump, 3);
    system.connect(pump_speed, 0, pump, 4);
    system.connect(q_zero, 0, pump, 5);

    // 2. Conexiones de la Tubería de Retorno
    system.connect(h_discharge, 1, pipe_return, 0);
    system.connect(h_discharge, 2, pipe_return, 1);
    system.connect(p_suction_ref, 0, pipe_return, 2);
    system.connect(h_suction_ref, 0, pipe_return, 3);
    system.connect(valve_pos, 0, pipe_return, 4);
    system.connect(q_zero, 0, pipe_return, 5);

    // 3. Balance en Header Discharge (Entra Bomba, Sale Retorno)
    system.connect(pump, 5, sum_w_discharge, 0);
    system.connect(pipe_return, 4, sum_w_discharge, 1);
    system.connect(sum_w_discharge, 0, h_discharge, 0);
    
    system.connect(pump, 7, sum_wh_discharge, 0);
    system.connect(pipe_return, 6, sum_wh_discharge, 1);
    system.connect(sum_wh_discharge, 0, h_discharge, 1);
    system.connect(q_zero, 0, h_discharge, 2);

    // --- SIMULACIÓN DISCRETA/HIBRIDA ---
    let mut solver = EulerSolver::new_hybrid(&system).unwrap();
    let t_final = 50.0;
    let sync_dt = 1.0;

    let mut csv_file = std::fs::File::create("discrete_pump_validation_results.csv").unwrap();
    use std::io::Write;
    writeln!(csv_file, "t,w_pump,w_return,p_suc,p_dis,dp,m_dis").unwrap();

    println!("Simulación de Bomba Centrífuga Discreta en Lazo Realista...");
    println!("t [s], W_pump [kg/s], W_return [kg/s], P_suc [Pa], P_dis [Pa], dP [Pa], M_dis [kg]");

    let steps = (t_final / sync_dt) as usize;
    for step_idx in 1..=steps {
        let target_t = step_idx as f64 * sync_dt;
        while solver.t < target_t - 1e-10 {
            solver.step_hybrid(&system, ts);
        }

        let y_discharge = &solver.get_outputs()[solver.get_y_offset(h_discharge)..];
        let y_pump = &solver.get_outputs()[solver.get_y_offset(pump)..];
        let y_pipe = &solver.get_outputs()[solver.get_y_offset(pipe_return)..];
        
        let p_suc = 1e5; // Fija por referencia
        let p_dis = y_discharge[1];
        let flow_pump = y_pump[2];
        
        let w_in_pipe = y_pipe[pipe_w_in_offset(&system, pipe_return)];

        let x_discharge = solver.get_block_state(h_discharge, 2);
        let m_dis = x_discharge[0];

        writeln!(
            csv_file,
            "{:.1},{:.6},{:.6},{:.2},{:.2},{:.2},{:.2}",
            solver.t, flow_pump, w_in_pipe, p_suc, p_dis, p_dis - p_suc, m_dis
        ).unwrap();

        println!("{:.1}, {:.4}, {:.4}, {:.1}, {:.1}, {:.1}, {:.1}", 
                 solver.t, flow_pump, w_in_pipe, p_suc, p_dis, p_dis - p_suc, m_dis);
    }
}

fn pipe_w_in_offset(system: &System, id: usize) -> usize {
    let block = &system.blocks[id];
    let mut offset = 0;
    for p in 0..4 { offset += block.output_width(p); }
    offset
}
