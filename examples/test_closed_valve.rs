use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D};
use bloques::thermo::LinearWater;
use std::sync::Arc;

fn main() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    let p_src = system.add_block(Box::new(Constant::new(vec![2e5])));
    let h_src = system.add_block(Box::new(Constant::new(vec![83700.0])));
    
    // Valve: CERRADA (0.0)
    let valve_pos = system.add_block(Box::new(Constant::new(vec![0.0])));
    let heater_q = system.add_block(Box::new(Constant::new(vec![0.0])));

    let pipe = system.add_block(Box::new(Pipe1D::new(1, 10.0, 0.1, thermo.clone())));
    let header = system.add_block(Box::new(Header::new(1.0, thermo.clone())));
    let q_ext_h = system.add_block(Box::new(Constant::new(vec![0.0])));

    system.connect(p_src, 0, pipe, 0);
    system.connect(h_src, 0, pipe, 1);
    system.connect(header, 0, pipe, 2);
    system.connect(header, 1, pipe, 3);
    system.connect(valve_pos, 0, pipe, 4);
    system.connect(heater_q, 0, pipe, 5);
    system.connect(pipe, 1, header, 0);
    system.connect(pipe, 3, header, 1);
    system.connect(q_ext_h, 0, header, 2);

    let mut solver = EulerSolver::new(&system).unwrap();
    let dt = 0.01; // Paso más grande
    
    println!("t, Flow (W)");
    for i in 0..1000 {
        solver.step(&system, dt);
        if i % 100 == 0 {
            let x_pipe = solver.get_block_state(pipe, 4);
            println!("{:.3}, {:.4}", solver.t, x_pipe[0]);
        }
    }
}
