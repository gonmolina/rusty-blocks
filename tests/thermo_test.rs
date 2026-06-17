use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, Header, Pipe1D};
use bloques::thermo::LinearWater;
use std::sync::Arc;

#[test]
fn test_pipe_flow_with_valve() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    // Fuentes
    let p_src = system.add_block(Box::new(Constant::new(vec![2e5])));
    let h_src = system.add_block(Box::new(Constant::new(vec![83700.0])));
    
    // Valve: 10% abierta
    let valve_pos = system.add_block(Box::new(Constant::new(vec![0.1])));
    // Heater: 0W
    let heater_q = system.add_block(Box::new(Constant::new(vec![0.0])));

    let pipe = system.add_block(Box::new(Pipe1D::new(1, 10.0, 0.1, 0.0, thermo.clone(), vec![], vec![])));
    let header = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![], vec![0])));
    let q_ext_h = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Conexiones Pipe
    system.connect(p_src, 0, pipe, 0);
    system.connect(h_src, 0, pipe, 1);
    system.connect(header, 1, pipe, 2);
    system.connect(header, 2, pipe, 3);
    system.connect(valve_pos, 0, pipe, 4);
    system.connect(heater_q, 0, pipe, 5);

    // Conexiones Header
    system.connect(pipe, 5, header, 0);
    system.connect(pipe, 7, header, 1);
    system.connect(q_ext_h, 0, header, 2);

    let mut solver = EulerSolver::new(&system).unwrap();
    let dt = 0.01;
    
    for _ in 0..100 {
        solver.step(&system, dt);
    }

    let x_pipe = solver.get_block_state(pipe, 3 + 1 + 2);
    let w_valve_closed = x_pipe[0];

    // Ahora con válvula 100% abierta
    let mut system2 = System::new();
    let p_src2 = system2.add_block(Box::new(Constant::new(vec![2e5])));
    let h_src2 = system2.add_block(Box::new(Constant::new(vec![83700.0])));
    let valve_pos2 = system2.add_block(Box::new(Constant::new(vec![1.0])));
    let heater_q2 = system2.add_block(Box::new(Constant::new(vec![0.0])));
    let pipe2 = system2.add_block(Box::new(Pipe1D::new(1, 10.0, 0.1, 0.0, thermo.clone(), vec![], vec![])));
    let header2 = system2.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![], vec![0])));
    let q_ext_h2 = system2.add_block(Box::new(Constant::new(vec![0.0])));

    system2.connect(p_src2, 0, pipe2, 0);
    system2.connect(h_src2, 0, pipe2, 1);
    system2.connect(header2, 1, pipe2, 2);
    system2.connect(header2, 2, pipe2, 3);
    system2.connect(valve_pos2, 0, pipe2, 4);
    system2.connect(heater_q2, 0, pipe2, 5);
    system2.connect(pipe2, 5, header2, 0);
    system2.connect(pipe2, 7, header2, 1);
    system2.connect(q_ext_h2, 0, header2, 2);

    let mut solver2 = EulerSolver::new(&system2).unwrap();
    for _ in 0..100 {
        solver2.step(&system2, dt);
    }

    let x_pipe2 = solver2.get_block_state(pipe2, 3 + 1 + 2);
    let w_valve_open = x_pipe2[0];

    println!("W with 10% valve: {:.4}, W with 100% valve: {:.4}", w_valve_closed, w_valve_open);
    assert!(w_valve_open > w_valve_closed);
}

#[test]
fn test_pipe_heater() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);

    let p_src = system.add_block(Box::new(Constant::new(vec![1e5])));
    let h_src = system.add_block(Box::new(Constant::new(vec![0.0])));
    let valve_pos = system.add_block(Box::new(Constant::new(vec![1.0])));
    // Calefactor potente: 10 MW
    let heater_q = system.add_block(Box::new(Constant::new(vec![1e7])));

    let pipe = system.add_block(Box::new(Pipe1D::new(1, 10.0, 0.1, 0.0, thermo.clone(), vec![], vec![])));
    let header = system.add_block(Box::new(Header::new(1.0, thermo.clone(), vec![], vec![0])));
    let q_ext_h = system.add_block(Box::new(Constant::new(vec![0.0])));

    system.connect(p_src, 0, pipe, 0);
    system.connect(h_src, 0, pipe, 1);
    system.connect(header, 1, pipe, 2);
    system.connect(header, 2, pipe, 3);
    system.connect(valve_pos, 0, pipe, 4);
    system.connect(heater_q, 0, pipe, 5);
    system.connect(pipe, 5, header, 0);
    system.connect(pipe, 7, header, 1);
    system.connect(q_ext_h, 0, header, 2);

    let mut solver = EulerSolver::new(&system).unwrap();
    let dt = 0.001;
    
    for _ in 0..100 {
        solver.step(&system, dt);
    }

    let x_pipe = solver.get_block_state(pipe, 3 + 1 + 2);
    let energy_cell = x_pipe[2]; // momentum(2) + mass(1) + energy(1)
    
    // La energía debería aumentar por el calefactor
    assert!(energy_cell > 0.0);
}
