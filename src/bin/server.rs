use axum::{
    routing::post,
    Json, Router,
};
use bloques::blocks::BlockRegistry;
use bloques::solver::EulerSolver;
use bloques::system::{Subsystem, System, SystemConfig};
use bloques::{SimulationParams, SolverType};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct SimulationRequest {
    system: SystemConfig,
    params: SimulationParams,
}

#[derive(Serialize)]
struct SimulationPoint {
    t: f64,
    x: Vec<f64>,
}

#[derive(Serialize)]
struct SimulationResponse {
    points: Vec<SimulationPoint>,
}

async fn simulate(Json(req): Json<SimulationRequest>) -> Json<SimulationResponse> {
    let mut registry = BlockRegistry::std();
    registry.register("Subsystem", Subsystem::build);

    let system = System::from_config(req.system, &registry);
    let mut solver = EulerSolver::new(&system).expect("Error initializing solver");

    let mut points = Vec::new();
    let mut t = 0.0;
    let mut current_dt = req.params.dt;

    // Record initial state
    points.push(SimulationPoint { t, x: solver.x.clone() });

    match req.params.solver {
        SolverType::Euler => {
            while t < req.params.t_final {
                solver.step(&system, current_dt);
                t += current_dt;
                points.push(SimulationPoint { t, x: solver.x.clone() });
            }
        }
        SolverType::RK4 => {
            while t < req.params.t_final {
                solver.step_rk4(&system, current_dt);
                t += current_dt;
                points.push(SimulationPoint { t, x: solver.x.clone() });
            }
        }
        SolverType::RK45 => {
            while t < req.params.t_final {
                current_dt = solver.step_rk45(&system, current_dt, req.params.atol, req.params.rtol);
                t = solver.t;
                points.push(SimulationPoint { t, x: solver.x.clone() });
            }
        }
    }

    Json(SimulationResponse { points })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/simulate", post(simulate))
        .layer(CorsLayer::permissive()); // Enable CORS for the frontend

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
