use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, ClosedTank, Step};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- BLOQUES ---
    
    // Tanque Cerrado: 1 m3 total, 0.2 m2 área, 2 bar nominal, 0.5 m3 gas inicial
    let tank = system.add_block(Box::new(ClosedTank::new(
        1.0, 0.2, 2e5, 0.5, thermo.clone(), vec![0], vec![0]
    )));

    // Señales: Inyectamos 1 kg/s de agua por 10s
    let w_in = system.add_block(Box::new(Step::new(1.0, 0.0, 10.0)));
    let wh_in = system.add_block(Box::new(Step::new(83700.0, 0.0, 10.0))); // ~20C
    let zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // --- CONEXIONES ---
    system.connect(w_in, 0, tank, 0);
    system.connect(wh_in, 0, tank, 1);
    system.connect(zero, 0, tank, 2); // w_out
    system.connect(zero, 0, tank, 3); // wh_out
    system.connect(zero, 0, tank, 4); // q_ext

    // --- SIMULACIÓN SINCRONIZADA ---
    let mut solver = EulerSolver::new(&system).unwrap();
    let mut suggested_dt: f64 = 0.01; 
    let t_final = 20.0;
    let sync_dt = 2.0;

    println!("Simulación de Tanque Cerrado (Llenado y Compresión)...");
    println!("t [s], Level [m], P_base [bar], T [C]");

    let total_sync_steps = (t_final / sync_dt) as usize;

    for step_idx in 0..=total_sync_steps {
        let target_t = step_idx as f64 * sync_dt;
        
        while solver.t < target_t - 1e-10 {
            let h = suggested_dt.min(target_t - solver.t);
            suggested_dt = solver.step_rk45(&system, h, 1e-4, 1e-3);
        }

        let y_tank = &solver.get_outputs()[solver.get_y_offset(tank)..];
        // Outputs: [T(0), P(1), Level(2), H(3), Rho(4), P_base(5)]
        println!("{:.1}, {:.3}, {:.2}, {:.2}", 
                 solver.t, y_tank[2], y_tank[5] / 1e5, y_tank[0] - 273.15);
    }
}
