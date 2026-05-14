use bloques::blocks::BlockRegistry;
use bloques::solver::EulerSolver;
use bloques::system::{Subsystem, System, SystemConfig};
use bloques::{SimulationParams, SolverType};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Uso: {} <archivo_sistema.json> [archivo_simulacion.json]",
            args[0]
        );
        std::process::exit(1);
    }

    // Initialize Block Registry
    let mut registry = BlockRegistry::std();
    registry.register("Subsystem", Subsystem::build);

    // 1. Cargar Sistema
    let system_path = &args[1];
    let system_content =
        fs::read_to_string(system_path).expect("No se pudo leer el archivo de sistema");
    let system_config: SystemConfig =
        serde_json::from_str(&system_content).expect("Error al parsear el sistema JSON");

    // 2. Cargar Parámetros de Simulación (o usar default)
    let sim_params = if args.len() >= 3 {
        let sim_path = &args[2];
        let sim_content =
            fs::read_to_string(sim_path).expect("No se pudo leer el archivo de simulación");
        serde_json::from_str(&sim_content).expect("Error al parsear la simulación JSON")
    } else {
        println!("Aviso: No se proporcionó archivo de simulación. Usando valores por defecto.");
        SimulationParams::default()
    };

    println!("Simulando sistema: {}", system_config.name);
    println!(
        "Solver: {:?}, dt: {}, t_final: {}, atol: {}, rtol: {}\n",
        sim_params.solver, sim_params.dt, sim_params.t_final, sim_params.atol, sim_params.rtol
    );

    let system = System::from_config(system_config, &registry);
    let mut solver = EulerSolver::new(&system).expect("Error al inicializar el solver");

    let mut t = 0.0;
    let mut current_dt = sim_params.dt;
    println!("t\tEstados");
    println!("{:.3}\t{:?}", t, solver.x);

    let mut last_print_t = 0.0;

    match sim_params.solver {
        SolverType::Euler => {
            while t < sim_params.t_final {
                solver.step(&system, current_dt);
                t += current_dt;
                if t >= last_print_t + 1.0 || t >= sim_params.t_final {
                    println!("{:.3}\t{:?}", t, solver.x);
                    last_print_t = t;
                }
            }
        }
        SolverType::RK4 => {
            while t < sim_params.t_final {
                solver.step_rk4(&system, current_dt);
                t += current_dt;
                if t >= last_print_t + 1.0 || t >= sim_params.t_final {
                    println!("{:.3}\t{:?}", t, solver.x);
                    last_print_t = t;
                }
            }
        }
        SolverType::RK45 => {
            while t < sim_params.t_final {
                current_dt = solver.step_rk45(&system, current_dt, sim_params.atol, sim_params.rtol);
                t = solver.t;
                if t >= last_print_t + 1.0 || t >= sim_params.t_final {
                    println!("{:.3}\t{:?}", t, solver.x);
                    last_print_t = t;
                }
            }
        }
    }

    println!("\nSimulación completada.");
    println!("Estado final en t={:.3}: {:?}", t, solver.x);
}
