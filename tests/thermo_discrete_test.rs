use bloques::system::System;
use bloques::solver::EulerSolver;
use bloques::blocks::{Constant, DiscreteHeader, DiscretePipe1D, DiscreteCentrifugalPump};
use bloques::thermo::LinearWater;
use std::sync::Arc;

#[test]
fn test_discrete_pipe_flow() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);
    let ts = 0.05;

    // Sources
    let p_src = system.add_block(Box::new(Constant::new(vec![2e5])));
    let h_src = system.add_block(Box::new(Constant::new(vec![83700.0])));
    let valve_pos = system.add_block(Box::new(Constant::new(vec![1.0])));
    let heater_q = system.add_block(Box::new(Constant::new(vec![0.0])));

    let pipe = system.add_block(Box::new(DiscretePipe1D::new(
        1,
        10.0,
        0.1,
        0.0,
        thermo.clone(),
        vec![],
        vec![],
        ts,
    )));
    let header = system.add_block(Box::new(DiscreteHeader::new(
        1.0,
        thermo.clone(),
        vec![],
        vec![0],
        ts,
        0.9,
    )));
    let q_ext_h = system.add_block(Box::new(Constant::new(vec![0.0])));

    // Connections
    system.connect(p_src, 0, pipe, 0);
    system.connect(h_src, 0, pipe, 1);
    system.connect(header, 1, pipe, 2);
    system.connect(header, 2, pipe, 3);
    system.connect(valve_pos, 0, pipe, 4);
    system.connect(heater_q, 0, pipe, 5);

    system.connect(pipe, 5, header, 0);
    system.connect(pipe, 7, header, 1);
    system.connect(q_ext_h, 0, header, 2);

    let mut solver = EulerSolver::new_hybrid(&system).unwrap();
    
    // Simulate discrete-time steps
    for _ in 0..100 {
        solver.step_hybrid(&system, ts);
    }

    let x_pipe = solver.get_block_state(pipe, 4);
    let w_flow = x_pipe[0];
    println!("Discrete pipe flow after 5s: {:.4} kg/s", w_flow);

    // Flow should be positive and converged to a physical value
    assert!(w_flow > 0.0);
    assert!(w_flow < 300.0);
}

#[test]
fn test_discrete_pump() {
    let mut system = System::new();
    let thermo = Arc::new(LinearWater);
    let ts = 0.1;

    // A simple test with DiscreteCentrifugalPump
    let p_in = system.add_block(Box::new(Constant::new(vec![1e5])));
    let p_out = system.add_block(Box::new(Constant::new(vec![1e5])));
    let speed = system.add_block(Box::new(Constant::new(vec![1.0])));
    let rho = system.add_block(Box::new(Constant::new(vec![1000.0])));
    let h_in = system.add_block(Box::new(Constant::new(vec![83700.0])));
    let h_out = system.add_block(Box::new(Constant::new(vec![83700.0])));

    // Coeffs: A0=3e5 Pa (3 bar shutoff), A1=0.0, A2=0.0
    // geom_inertia: 0.1
    // fric_pasivo: 100.0
    let pump = system.add_block(Box::new(DiscreteCentrifugalPump::new(
        [3e5, 0.0, 0.0],
        0.1,
        100.0,
        ts,
    )));

    // Inputs of DiscreteCentrifugalPump:
    // 0: p_in, 1: h_in, 2: p_out, 3: h_out, 4: speed, 5: rho
    system.connect(p_in, 0, pump, 0);
    system.connect(h_in, 0, pump, 1);
    system.connect(p_out, 0, pump, 2);
    system.connect(h_out, 0, pump, 3);
    system.connect(speed, 0, pump, 4);
    system.connect(rho, 0, pump, 5);

    let mut solver = EulerSolver::new_hybrid(&system).unwrap();
    
    for _ in 0..100 {
        solver.step_hybrid(&system, ts);
    }

    let x_pump = solver.get_block_state(pump, 1);
    let w_pump = x_pump[0];
    println!("Discrete pump flow after 10s: {:.4} kg/s", w_pump);

    // Pump flow should settle to the expected steady state (~1732 kg/s)
    assert!(w_pump > 1500.0 && w_pump < 1800.0);
}
