pub mod blocks;
pub mod solver;
pub mod system;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum SolverType {
    Euler,
    RK4,
    RK45,
    Hybrid,
    Discrete,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationParams {
    pub dt: f64,
    pub t_final: f64,
    #[serde(default = "default_solver")]
    pub solver: SolverType,
    #[serde(default = "default_atol")]
    pub atol: f64,
    #[serde(default = "default_rtol")]
    pub rtol: f64,
}

fn default_solver() -> SolverType { SolverType::Euler }
fn default_atol() -> f64 { 1e-6 }
fn default_rtol() -> f64 { 1e-3 }

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
