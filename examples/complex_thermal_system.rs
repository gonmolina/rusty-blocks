use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D, Sum, CentrifugalPump, Gain};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // --- BLOQUES FÍSICOS (SISTEMA ABIERTO PARA ESTABILIDAD) ---
    // En lugar de un lazo cerrado hermético que explota por rigidez acústica,
    // usamos fuentes de presión constantes en los extremos para ver el control térmico.

    let p_cold = system.add_block(Box::new(Constant::new(vec![1e5]))); // 1 bar
    let h_cold_val = system.add_block(Box::new(Constant::new(vec![83700.0]))); // 20C
    
    let pump = system.add_block(Box::new(CentrifugalPump::new([0.2222, 0.0, -20.0], 0.01, 0.1)));
    let h_mid = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![0], vec![0])));
    
    let pipe_heater = system.add_block(Box::new(Pipe1D::new(5, 5.0, 0.1, 0.0, thermo.clone(), vec![2], vec![2])));
    
    let h_hot = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![0], vec![0])));
    let p_sink = system.add_block(Box::new(Constant::new(vec![1e5])));
    let h_sink = system.add_block(Box::new(Constant::new(vec![83700.0])));

    // --- CONTROL ---
    let setpoint_t = system.add_block(Box::new(Constant::new(vec![273.15 + 60.0])));
    let error_t = system.add_block(Box::new(Sum::new("+-", 1)));
    let p_gain = system.add_block(Box::new(Gain::new(10000.0, 1))); // 10 kW/C
    let pump_speed = system.add_block(Box::new(Constant::new(vec![1500.0])));
    let valve_open = system.add_block(Box::new(Constant::new(vec![1.0])));
    let q_zero = system.add_block(Box::new(Constant::new(vec![0.0])));

    // --- CONEXIONES ---
    system.connect(p_cold, 0, pump, 0); system.connect(h_cold_val, 0, pump, 1);
    system.connect(h_mid, 1, pump, 2);  system.connect(h_mid, 2, pump, 3);
    system.connect(pump_speed, 0, pump, 4); system.connect(q_zero, 0, pump, 5);

    system.connect(pump, 5, h_mid, 0); system.connect(pump, 7, h_mid, 1); system.connect(q_zero, 0, h_mid, 2);

    system.connect(h_mid, 1, pipe_heater, 0); system.connect(h_mid, 2, pipe_heater, 1);
    system.connect(h_hot, 1, pipe_heater, 2); system.connect(h_hot, 2, pipe_heater, 3);
    system.connect(valve_open, 0, pipe_heater, 4); system.connect(p_gain, 0, pipe_heater, 5);

    let sum_w_hot = system.add_block(Box::new(Sum::new("+-", 1)));
    let sum_wh_hot = system.add_block(Box::new(Sum::new("+-", 1)));
    
    // Pipe -> Sumadores -> Header_Hot
    system.connect(pipe_heater, 5, sum_w_hot, 0); 
    // Suponemos una "fuga" controlada o salida al sink
    let w_sink = system.add_block(Box::new(Constant::new(vec![1.0]))); // Salida constante 1kg/s
    system.connect(w_sink, 0, sum_w_hot, 1);
    system.connect(sum_w_hot, 0, h_hot, 0);
    
    system.connect(pipe_heater, 7, sum_wh_hot, 0);
    // wh_sink = w_sink * h_hot
    let mult_wh = system.add_block(Box::new(Gain::new(1.0, 1))); // Simplificado
    system.connect(h_hot, 2, mult_wh, 0);
    system.connect(mult_wh, 0, sum_wh_hot, 1);
    system.connect(sum_wh_hot, 0, h_hot, 1);
    system.connect(q_zero, 0, h_hot, 2);

    system.connect(setpoint_t, 0, error_t, 0);
    system.connect(h_hot, 0, error_t, 1);
    system.connect(error_t, 0, p_gain, 0);

    // --- SIMULACIÓN ---
    let mut solver = EulerSolver::new(&system).unwrap();
    let mut dt: f64 = 0.001;
    println!("t [s], T_hot [C], Flow [kg/s], Q_heater [kW]");

    for i in 0..100 {
        let target_t = (i+1) as f64 * 5.0;
        while solver.t < target_t - 1e-10 {
            dt = solver.step_rk45(&system, dt, 1e-4, 1e-3);
        }
        let y_hot = &solver.get_outputs()[solver.get_y_offset(h_hot)..];
        let y_ctrl = &solver.get_outputs()[solver.get_y_offset(p_gain)..];
        let y_pump = &solver.get_outputs()[solver.get_y_offset(pump)..];
        println!("{:.1}, {:.2}, {:.4}, {:.1}", solver.t, y_hot[0]-273.15, y_pump[2], y_ctrl[0]/1000.0);
    }
}
