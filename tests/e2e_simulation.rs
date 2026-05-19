use bloques::blocks::BlockRegistry;
use bloques::solver::EulerSolver;
use bloques::system::{Subsystem, System, SystemConfig};
use bloques::{SimulationParams, SolverType};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct E2EFixture {
    description: String,
    system: SystemConfig,
    params: SimulationParams,
    expect: Expectations,
}

#[derive(Deserialize)]
struct Expectations {
    min_points: usize,
    final_t: f64,
    #[serde(default)]
    final_state_lt: Option<f64>,
    #[serde(default)]
    check_t_1_5: Option<PointCheck>,
    #[serde(default)]
    check_t_0_5_int_approx_0_5: Option<bool>,
    #[serde(default)]
    check_t_1_int_approx_1: Option<bool>,
}

#[derive(Deserialize)]
struct PointCheck {
    block: String,
    output: usize,
    value: f64,
}

fn load_fixtures(dir: &str) -> Vec<(String, E2EFixture)> {
    let mut fixtures = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            fixtures.extend(load_fixtures(path.to_str().unwrap()));
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = fs::read_to_string(&path).unwrap();
            let fixture: E2EFixture = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));
            fixtures.push((path.display().to_string(), fixture));
        }
    }
    fixtures
}

fn run_fixture(fixture: &E2EFixture) -> (Vec<f64>, Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut registry = BlockRegistry::std();
    registry.register("Subsystem", Subsystem::build);
    let system = System::from_config(fixture.system.clone(), &registry);

    let mut solver = match fixture.params.solver {
        SolverType::Hybrid | SolverType::Discrete => EulerSolver::new_hybrid(&system),
        _ => EulerSolver::new(&system),
    }
    .expect("Error initializing solver");

    let mut points_y: Vec<Vec<f64>> = Vec::new();
    let mut points_t: Vec<f64> = Vec::new();
    let mut points_x: Vec<Vec<f64>> = Vec::new();

    points_t.push(0.0);
    points_x.push(solver.x.clone());
    points_y.push(solver.get_outputs().to_vec());

    let mut t = 0.0;
    let mut current_dt = fixture.params.dt;

    match fixture.params.solver {
        SolverType::Euler | SolverType::RK4 => {
            while t < fixture.params.t_final {
                if fixture.params.solver == SolverType::Euler {
                    solver.step(&system, current_dt);
                } else {
                    solver.step_rk4(&system, current_dt);
                }
                t += current_dt;
                points_t.push(t);
                points_x.push(solver.x.clone());
                points_y.push(solver.get_outputs().to_vec());
            }
        }
        SolverType::RK45 => {
            while t < fixture.params.t_final {
                current_dt = solver.step_rk45(&system, current_dt, fixture.params.atol, fixture.params.rtol);
                t = solver.t;
                points_t.push(t);
                points_x.push(solver.x.clone());
                points_y.push(solver.get_outputs().to_vec());
            }
        }
        SolverType::Hybrid => {
            while t < fixture.params.t_final {
                let prev_t = solver.t;
                solver.step_hybrid(&system, current_dt);
                t = solver.t;
                if t <= prev_t { break; }
                points_t.push(t);
                points_x.push(solver.x.clone());
                points_y.push(solver.get_outputs().to_vec());
            }
        }
        SolverType::Discrete => {
            while t < fixture.params.t_final {
                let prev_t = solver.t;
                solver.step_discrete(&system);
                t = solver.t;
                if t <= prev_t { break; }
                points_t.push(t);
                points_x.push(solver.x.clone());
                points_y.push(solver.get_outputs().to_vec());
            }
        }
    }

    (points_t, points_x, points_y)
}

fn find_block_offset(fixture: &E2EFixture, block_id: &str) -> usize {
    let mut offset = 0;
    for b in &fixture.system.blocks {
        if b.id == block_id { return offset; }
        offset += 1; // assumes width=1
    }
    panic!("Block {} not found", block_id);
}

#[test]
fn e2e_all_fixtures() {
    let fixtures = load_fixtures("tests/e2e");
    assert!(!fixtures.is_empty(), "No E2E fixtures found in tests/e2e/");

    for (path, fixture) in &fixtures {
        println!("\n=== {} ===", fixture.description);
        println!("  file: {}", path);

        let (points_t, points_x, points_y) = run_fixture(fixture);

        let final_t = points_t.last().copied().unwrap_or(0.0);
        let eps = fixture.params.dt.max(1e-6);

        // --- Universal checks ---
        assert!(
            points_t.len() >= fixture.expect.min_points,
            "[{}] Expected >= {} points, got {}",
            fixture.description, fixture.expect.min_points, points_t.len()
        );
        assert!(
            (final_t - fixture.expect.final_t).abs() < eps || final_t >= fixture.expect.final_t - eps,
            "[{}] Expected final t ≈ {}, got {}",
            fixture.description, fixture.expect.final_t, final_t
        );

        // --- Continuous: feedback loop ---
        if let Some(threshold) = fixture.expect.final_state_lt {
            let final_state = points_x.last().and_then(|x| x.first().copied()).unwrap_or(f64::INFINITY);
            assert!(
                final_state < threshold,
                "[{}] Expected final state < {}, got {}",
                fixture.description, threshold, final_state
            );
        }

        // --- Discrete: specific point check ---
        if let Some(ref check) = fixture.expect.check_t_1_5 {
            let target_t = 1.5;
            let mut best_idx = 0;
            let mut best_dist = f64::INFINITY;
            for (i, &t) in points_t.iter().enumerate() {
                let dist = (t - target_t).abs();
                if dist < best_dist { best_dist = dist; best_idx = i; }
            }
            let offset = find_block_offset(fixture, &check.block);
            let val = points_y[best_idx].get(offset + check.output).copied().unwrap_or(f64::NAN);
            assert!(
                (val - check.value).abs() < 0.02,
                "[{}] At t≈{}, block {} output {}: expected {}, got {}",
                fixture.description, target_t, check.block, check.output, check.value, val
            );
        }

        // --- Hybrid: integrator ramp ---
        if fixture.expect.check_t_0_5_int_approx_0_5 == Some(true) {
            let int_id = fixture.system.blocks.iter()
                .find(|b| b.r#type == "Integrator").map(|b| b.id.clone()).unwrap();
            let offset = find_block_offset(fixture, &int_id);
            let target_t = 0.5;
            let mut best_idx = 0;
            let mut best_dist = f64::INFINITY;
            for (i, &t) in points_t.iter().enumerate() {
                let dist = (t - target_t).abs();
                if dist < best_dist { best_dist = dist; best_idx = i; }
            }
            let val = points_y[best_idx].get(offset).copied().unwrap_or(f64::NAN);
            assert!(
                (val - 0.5).abs() < 0.1,
                "[{}] At t≈0.5, integrator should be ≈0.5, got {}",
                fixture.description, val
            );
        }

        if fixture.expect.check_t_1_int_approx_1 == Some(true) {
            let int_id = fixture.system.blocks.iter()
                .find(|b| b.r#type == "Integrator").map(|b| b.id.clone()).unwrap();
            let offset = find_block_offset(fixture, &int_id);
            let target_t = 1.0;
            let mut best_idx = 0;
            let mut best_dist = f64::INFINITY;
            for (i, &t) in points_t.iter().enumerate() {
                let dist = (t - target_t).abs();
                if dist < best_dist { best_dist = dist; best_idx = i; }
            }
            let val = points_y[best_idx].get(offset).copied().unwrap_or(f64::NAN);
            assert!(
                (val - 1.0).abs() < 0.15,
                "[{}] At t≈1.0, integrator should be ≈1.0, got {}",
                fixture.description, val
            );
        }

        println!("  \u{2705} PASSED ({} points, final t={:.3})", points_t.len(), final_t);
    }
}
