mod block;
mod solver;
mod system;

use serde::{Deserialize, Serialize};
use solver::EulerSolver;
use std::env;
use std::fs;
use system::{System, SystemConfig};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SolverType {
    Euler,
    RK4,
    RK45, // Adaptive Runge-Kutta 4-5
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationParams {
    pub dt: f64, // For Euler/RK4 it's the fixed step. For RK45 it's the initial/max step.
    pub t_final: f64,
    #[serde(default = "default_solver")]
    pub solver: SolverType,
    #[serde(default = "default_atol")]
    pub atol: f64,
    #[serde(default = "default_rtol")]
    pub rtol: f64,
}

fn default_solver() -> SolverType {
    SolverType::Euler
}
fn default_atol() -> f64 {
    1e-6
}
fn default_rtol() -> f64 {
    1e-3
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            dt: 0.01,
            t_final: 10.0,
            solver: SolverType::Euler,
            atol: 1e-6,
            rtol: 1e-3,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Uso: {} <archivo_sistema.json> [archivo_simulacion.json]",
            args[0]
        );
        std::process::exit(1);
    }

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

    let system = System::from_config(system_config);
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
