use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use bloques::blocks::BlockRegistry;
use bloques::solver::EulerSolver;
use bloques::system::{Subsystem, System, SystemConfig};
use bloques::{SimulationParams, SolverType};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::fs::File;

#[derive(Deserialize)]
struct SimulationRequest {
    system: SystemConfig,
    params: SimulationParams,
}

#[derive(Serialize)]
struct SimulationPoint {
    t: f64,
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Serialize)]
struct SimulationResponse {
    points: Vec<SimulationPoint>,
    y_offsets: HashMap<String, usize>,
    output_widths: HashMap<String, Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn get_result(Path(filename): Path<String>) -> Result<Json<Vec<HashMap<String, f64>>>, String> {
    let path = std::path::Path::new("sim_results").join(&filename);
    let file = File::open(path).map_err(|e| format!("Could not open file: {}", e))?;
    let reader = std::io::BufReader::new(file);
    let mut csv_reader = csv::Reader::from_reader(reader);
    
    let mut results = Vec::new();
    for result in csv_reader.deserialize() {
        let record: HashMap<String, f64> = result.map_err(|e| format!("CSV Error: {}", e))?;
        results.push(record);
    }
    
    Ok(Json(results))
}

async fn simulate(Json(req): Json<SimulationRequest>) -> Json<SimulationResponse> {
    println!("Recibida petición de simulación para: {}", req.system.name);
    let mut registry = BlockRegistry::std();
    registry.register("Subsystem", Subsystem::build);

    let system = System::from_config(req.system.clone(), &registry);

    // Validate: Discrete solver requires no continuous-state blocks
    if req.params.solver == SolverType::Discrete {
        let has_continuous = system.blocks.iter().any(|b| {
            b.num_states() > 0 && b.sample_time().is_none()
        });
        if has_continuous {
            return Json(SimulationResponse {
                points: vec![],
                y_offsets: HashMap::new(),
                output_widths: HashMap::new(),
                error: Some("El solver 'Discrete' solo puede usarse con sistemas puramente discretos (sin bloques con estados continuos como Integrator). Usá 'Hybrid' para sistemas mixtos.".into()),
            });
        }
    }

    let mut solver = match req.params.solver {
        SolverType::Hybrid | SolverType::Discrete => EulerSolver::new_hybrid(&system),
        _ => EulerSolver::new(&system),
    }
    .expect("Error initializing solver");

    // Map block string IDs to their global output offsets and output port widths
    let mut y_offsets = HashMap::new();
    let mut output_widths = HashMap::new();
    for (i, b_config) in req.system.blocks.iter().enumerate() {
        y_offsets.insert(b_config.id.clone(), solver.get_y_offset(i));
        let widths: Vec<usize> = (0..system.blocks[i].num_outputs())
            .map(|p| system.blocks[i].output_width(p))
            .collect();
        output_widths.insert(b_config.id.clone(), widths);
    }

    let mut points = Vec::new();
    let mut t = 0.0;
    let mut current_dt = req.params.dt;

    // Record initial state
    points.push(SimulationPoint { 
        t, 
        x: solver.x.clone(),
        y: solver.get_outputs().to_vec(),
    });

    match req.params.solver {
        SolverType::Euler => {
            while t < req.params.t_final {
                solver.step(&system, current_dt);
                t += current_dt;
                points.push(SimulationPoint { t, x: solver.x.clone(), y: solver.get_outputs().to_vec() });
            }
        }
        SolverType::RK4 => {
            while t < req.params.t_final {
                solver.step_rk4(&system, current_dt);
                t += current_dt;
                points.push(SimulationPoint { t, x: solver.x.clone(), y: solver.get_outputs().to_vec() });
            }
        }
        SolverType::RK45 => {
            while t < req.params.t_final {
                current_dt = solver.step_rk45(&system, current_dt, req.params.atol, req.params.rtol);
                t = solver.t;
                points.push(SimulationPoint { t, x: solver.x.clone(), y: solver.get_outputs().to_vec() });
            }
        }
        SolverType::Hybrid => {
            while t < req.params.t_final {
                let prev_t = solver.t;
                solver.step_hybrid(&system, current_dt);
                t = solver.t;
                if t <= prev_t { break; } // safety: no progress
                points.push(SimulationPoint { t, x: solver.x.clone(), y: solver.get_outputs().to_vec() });
            }
        }
        SolverType::Discrete => {
            while t < req.params.t_final {
                let prev_t = solver.t;
                solver.step_discrete(&system);
                t = solver.t;
                if t <= prev_t { break; } // safety: no progress
                points.push(SimulationPoint { t, x: solver.x.clone(), y: solver.get_outputs().to_vec() });
            }
        }
    }

    Json(SimulationResponse { points, y_offsets, output_widths, error: None })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/simulate", post(simulate))
        .route("/results/:filename", get(get_result))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
