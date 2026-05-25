use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, StratifiedTank, Step};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- BLOQUES ---
    
    // Tanque Estratificado (3 capas, 0.1 m3 total)
    let tank = system.add_block(Box::new(StratifiedTank::new(
        3, 0.1, 0.5, thermo.clone(), vec![0, 1, 2], vec![]
    )));

    // Escenario: Carga desde el TOPE con agua caliente (80 C)
    // 0.1 kg/s entrando por arriba, 0.1 kg/s saliendo por abajo
    let w_in_top = system.add_block(Box::new(Constant::new(vec![0.1])));
    let w_out_bot = system.add_block(Box::new(Constant::new(vec![-0.1])));

    // Energía de entrada en el tope:
    // Fase 1: Agua caliente (80 C)
    // Fase 2: Agua fría (10 C ~ 41 kJ/kg) a partir de los 200s
    let wh_top_signal = system.add_block(Box::new(Step::new(0.1 * 334000.0, 0.1 * 41000.0, 200.0)));
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // --- CONEXIONES ---
    system.connect(w_in_top, 0, tank, 0);   // w_top (Entrando)
    system.connect(wh_top_signal, 0, tank, 1); // wh_top
    system.connect(w_out_bot, 0, tank, 2);  // w_bot (Saliendo)
    system.connect(q_zero, 0, tank, 3);    // wh_bot
    system.connect(q_zero, 0, tank, 4);    // q_ext

    // --- SIMULACIÓN ---
    let mut solver = EulerSolver::new(&system).unwrap();
    let mut dt: f64 = 0.01;
    let t_final = 400.0;
    let sync_dt = 20.0;

    println!("Simulación de Tanque Estratificado: Calentamiento (0-200s) -> Enfriamiento (200-400s)");
    println!("t [s], T_top [C], T_mid [C], T_bot [C]");

    let steps = (t_final / sync_dt) as usize;
    for step_idx in 1..=steps {
        let target_t = step_idx as f64 * sync_dt;
        while solver.t < target_t - 1e-10 {
            let h = dt.min(target_t - solver.t);
            dt = solver.step_rk45(&system, h, 1e-4, 1e-3);
        }

        let y_tank = &solver.get_outputs()[solver.get_y_offset(tank)..];
        println!("{:.1}, {:.2}, {:.2}, {:.2}", 
                 solver.t, y_tank[0] - 273.15, y_tank[1] - 273.15, y_tank[2] - 273.15);
        
        if (solver.t - 200.0).abs() < 1e-5 {
            println!("--- INICIO DE ENFRIAMIENTO (Agua Fría sobre Caliente) ---");
        }
    }
}
