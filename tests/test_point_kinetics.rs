use bloques::{
    PointKineticsBlock, PointKineticsParams,
    blocks::{Block, BlockRegistry},
};
use serde_json::json;

#[test]
fn test_point_kinetics_steady_state_equilibrium() {
    let dt = 0.01;
    let mut params = PointKineticsParams::default();
    params.nominal_power_kw = 30.0; // 30 kW
    params.sample_time_dt = dt;

    let block = PointKineticsBlock::new(params);

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);
    assert_eq!(state[0], 30.0); // Potencia inicial 30 kW

    let mut outputs = vec![0.0; block.num_outputs()];
    // Entradas: [rho_ext = 0, T_fuel = 65°C, T_coolant = 45°C] (en equilibrio)
    let inputs = [0.0, 65.0, 45.0];

    block.outputs(0.0, &state, &inputs, &mut outputs);
    assert!((outputs[0] - 30.0).abs() < 1e-4, "Potencia térmica debe ser 30 kW");
    assert!((outputs[1] - 30.0).abs() < 1e-4, "Potencia pronta debe ser 30 kW");
    assert_eq!(outputs[2], 0.0, "Reactividad total debe ser 0 pcm");
    assert_eq!(outputs[3], 0.0, "Reactividad de realimentación debe ser 0 pcm");

    // Simular 100 pasos (1 segundo) sin perturbación
    for step in 1..=100 {
        let t = step as f64 * dt;
        state = block.update(t, &state, &inputs);
    }

    block.outputs(1.0, &state, &inputs, &mut outputs);
    assert!((outputs[0] - 30.0).abs() < 1e-3, "Potencia debe permanecer exactamente constante en 30 kW");
}

#[test]
fn test_point_kinetics_positive_reactivity_step() {
    let dt = 0.01;
    let mut params = PointKineticsParams::default();
    params.nominal_power_kw = 30.0;
    params.sample_time_dt = dt;

    let block = PointKineticsBlock::new(params);
    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);

    // Inserción de reactividad positiva de +50 pcm sin realimentación térmica (temperaturas fijas)
    let inputs_step = [50.0, 65.0, 45.0];

    // Simular 50 pasos (0.5 s)
    for step in 1..=50 {
        let t = step as f64 * dt;
        state = block.update(t, &state, &inputs_step);
    }

    let mut outputs = vec![0.0; block.num_outputs()];
    block.outputs(0.5, &state, &inputs_step, &mut outputs);

    // La potencia debe haber crecido por encima de los 30 kW
    assert!(outputs[0] > 32.0, "La potencia debe haber crecido tras +50 pcm (actual: {})", outputs[0]);
    assert_eq!(outputs[2], 50.0, "Reactividad total debe ser +50 pcm");
}

#[test]
fn test_point_kinetics_thermal_feedbacks() {
    let dt = 0.01;
    let mut params = PointKineticsParams::default();
    params.nominal_power_kw = 30.0;
    params.alpha_fuel_pcm = -2.2;     // -2.2 pcm/°C
    params.alpha_coolant_pcm = -14.2;  // -14.2 pcm/°C
    params.sample_time_dt = dt;

    let block = PointKineticsBlock::new(params);
    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);

    // Perturbación térmica: Combustible sube +10°C (65 -> 75°C), Refrigerante sube +5°C (45 -> 50°C)
    // Delta rho_fb = -2.2 * 10 + -14.2 * 5 = -22 - 71 = -93 pcm
    let inputs_heat = [0.0, 75.0, 50.0];

    let mut outputs = vec![0.0; block.num_outputs()];
    block.outputs(0.0, &state, &inputs_heat, &mut outputs);
    assert!((outputs[3] - (-93.0)).abs() < 1e-4, "Realimentación debe ser -93 pcm");
    assert!((outputs[2] - (-93.0)).abs() < 1e-4, "Reactividad total debe ser -93 pcm");

    // Simular 50 pasos (0.5 s) con realimentación negativa
    for step in 1..=50 {
        let t = step as f64 * dt;
        state = block.update(t, &state, &inputs_heat);
    }

    block.outputs(0.5, &state, &inputs_heat, &mut outputs);
    // La potencia debe haber disminuido por el efecto autorregulador negativo
    assert!(outputs[0] < 28.0, "La potencia debe haber disminuido por la realimentación negativa (actual: {})", outputs[0]);
}

#[test]
fn test_point_kinetics_5_groups_equivalence() {
    let dt = 0.01;
    let params = PointKineticsParams::default();
    let block_5 = PointKineticsBlock::new(params).with_5_precursor_groups();

    // 1 potencia + 5 precursores + 6 gammas = 12 estados
    assert_eq!(block_5.num_states(), 12);
    assert_eq!(block_5.params.beta_i.len(), 5);
    assert_eq!(block_5.params.lambda_i.len(), 5);

    let mut state = vec![0.0; block_5.num_states()];
    block_5.get_initial_conditions(&mut state);
    assert_eq!(state[0], 30.0);

    let inputs = [0.0, 65.0, 45.0];
    for step in 1..=100 {
        let t = step as f64 * dt;
        state = block_5.update(t, &state, &inputs);
    }
    let mut outputs = vec![0.0; block_5.num_outputs()];
    block_5.outputs(1.0, &state, &inputs, &mut outputs);
    assert!((outputs[0] - 30.0).abs() < 1e-3, "5 grupos debe mantener el equilibrio a 30 kW");
}

#[test]
fn test_point_kinetics_json_builder() {
    let registry = BlockRegistry::std();
    let json_config = json!({
        "nominal_power_kw": 30.0,
        "sample_time_dt": 0.01,
        "alpha_fuel_pcm": -2.2,
        "alpha_coolant_pcm": -14.2,
        "num_precursor_groups": 5
    });

    let block = registry.build("PointKinetics", json_config).expect("PointKinetics should build from JSON");
    assert_eq!(block.num_inputs(), 3);
    assert_eq!(block.num_outputs(), 4);
    assert_eq!(block.sample_time(), Some(0.01));
    assert_eq!(block.num_states(), 12); // 1 + 5 precursores + 6 gammas
}

#[test]
fn test_point_kinetics_xenon_steady_state_and_reset() {
    let dt = 0.05;
    let params = PointKineticsParams::default();
    let block = PointKineticsBlock::new(params)
        .with_5_precursor_groups()
        .with_xenon();

    // 1 potencia + 5 precursores + 6 gammas + 2 (Iodo y Xenon) = 14 estados
    assert_eq!(block.num_states(), 14);
    assert_eq!(block.num_inputs(), 4);
    assert_eq!(block.num_outputs(), 7);

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);
    assert_eq!(state[0], 30.0);

    // Salidas iniciales
    let inputs = [0.0, 65.0, 45.0, 0.0]; // sin reset
    let mut outputs = vec![0.0; block.num_outputs()];
    block.outputs(0.0, &state, &inputs, &mut outputs);

    assert!((outputs[0] - 30.0).abs() < 1e-3, "Potencia térmica 30 kW");
    assert!((outputs[2] - 0.0).abs() < 1e-4, "Reactividad total 0 pcm en equilibrio");
    assert!((outputs[4] - (-3650.0)).abs() < 1e-3, "Reactividad de Xenón en equilibrio debe ser -3650 pcm");
    assert!((outputs[5] - 1.0).abs() < 1e-3, "Concentración relativa de Xenón debe ser 1.0");
    assert!((outputs[6] - 1.0).abs() < 1e-3, "Concentración relativa de Iodo debe ser 1.0");

    // Simular 20 pasos (1 segundo) en equilibrio
    for step in 1..=20 {
        let t = step as f64 * dt;
        state = block.update(t, &state, &inputs);
    }
    block.outputs(1.0, &state, &inputs, &mut outputs);
    assert!((outputs[0] - 30.0).abs() < 1e-2, "Potencia debe permanecer en 30 kW");
    assert!((outputs[4] - (-3650.0)).abs() < 1e-2, "Xenón permanece en -3650 pcm");

    // Activar señal de reset externo con modo por defecto ("nominal_power")
    let inputs_reset = [0.0, 65.0, 45.0, 1.0];
    state = block.update(1.0 + dt, &state, &inputs_reset);
    block.outputs(1.0 + dt, &state, &inputs_reset, &mut outputs);

    // Por defecto resetea al equilibrio de potencia nominal
    assert!((outputs[4] - (-3650.0)).abs() < 1e-2, "Reactividad de Xenón en reset nominal_power debe ser -3650 pcm");
    assert!((outputs[5] - 1.0).abs() < 1e-3, "Concentración relativa de Xenón debe ser 1.0");
    assert!((outputs[6] - 1.0).abs() < 1e-3, "Concentración relativa de Iodo debe ser 1.0");
}

#[test]
fn test_point_kinetics_xenon_reset_clean_and_current_power() {
    let dt = 0.05;
    let mut params = PointKineticsParams::default();
    params.xenon_enabled = true;
    params.xenon_reset_mode = "clean".to_string();

    let block_clean = PointKineticsBlock::new(params.clone());
    let mut state = vec![0.0; block_clean.num_states()];
    block_clean.get_initial_conditions(&mut state);

    let inputs_reset = [0.0, 65.0, 45.0, 1.0];
    state = block_clean.update(dt, &state, &inputs_reset);
    let mut outputs = vec![0.0; block_clean.num_outputs()];
    block_clean.outputs(dt, &state, &inputs_reset, &mut outputs);

    assert_eq!(outputs[4], 0.0, "Reactividad Xenón en modo clean debe ser 0 pcm");
    assert_eq!(outputs[5], 0.0, "Xenón relativo en modo clean debe ser 0");
    assert_eq!(outputs[6], 0.0, "Iodo relativo en modo clean debe ser 0");

    // Modo current_power operando a media potencia (15 kW)
    params.xenon_reset_mode = "current_power".to_string();
    let block_curr = PointKineticsBlock::new(params);
    let mut state_curr = vec![0.0; block_curr.num_states()];
    block_curr.get_initial_conditions(&mut state_curr);
    state_curr[0] = 15.0; // 50% potencia

    state_curr = block_curr.update(dt, &state_curr, &inputs_reset);
    block_curr.outputs(dt, &state_curr, &inputs_reset, &mut outputs);

    assert!((outputs[6] - 0.5).abs() < 1e-2, "Iodo relativo a 50% potencia debe ser 0.5, dio {}", outputs[6]);
    assert!(outputs[5] < 1.0 && outputs[5] > 0.0, "Xenón relativo a 50% potencia debe ser intermedio");
}

#[test]
fn test_point_kinetics_xenon_json_builder() {
    let registry = BlockRegistry::std();
    let json_config = json!({
        "nominal_power_kw": 30000.0,
        "sample_time_dt": 0.05,
        "num_precursor_groups": 5,
        "xenon_enabled": true,
        "yield_iodine": 0.0628,
        "yield_xenon": 0.0026,
        "lambda_iodine": 2.9e-5,
        "lambda_xenon": 2.1e-5,
        "xenon_burnout_rate": 7.41e-4,
        "xenon_equilibrium_reactivity_pcm": -3650.0,
        "xenon_initial_state": "equilibrium"
    });

    let block = registry.build("PointKinetics", json_config).expect("PointKinetics with Xenon should build");
    assert_eq!(block.num_inputs(), 4);
    assert_eq!(block.num_outputs(), 7);
    assert_eq!(block.num_states(), 14);
}

#[test]
fn test_point_kinetics_custom_precursors_and_gamma_json() {
    let registry = BlockRegistry::std();
    let json_config = json!({
        "nominal_power_kw": 30000.0,
        "sample_time_dt": 0.01,
        "precursor_betas_pcm": [30.0, 150.0, 130.0, 290.0],
        "precursor_decay_constants": [0.012, 0.031, 0.11, 0.30],
        "gamma_decay_constants": [1.0, 0.08, 0.05],
        "gamma_energy_fractions": [0.01, 0.02, 0.04]
    });

    let block = registry.build("PointKinetics", json_config).expect("Should build with custom precursors and gamma");
    // 1 potencia + 4 precursores + 3 gammas = 8 estados
    assert_eq!(block.num_states(), 8);

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);
    assert_eq!(state[0], 30000.0);

    let inputs = [0.0, 65.0, 45.0];
    let mut outputs = vec![0.0; 5]; // 4 básicos + 1 decay_heat
    block.outputs(0.0, &state, &inputs, &mut outputs);

    // Suma f_gamma = 0.07 -> 7% de 30 MW = 2100 kW de decay heat
    assert!((outputs[0] - 30000.0).abs() < 1e-2, "Potencia térmica total 30 MW");
    assert!((outputs[4] - 2100.0).abs() < 1e-2, "Potencia de decaimiento gamma 2100 kW");
}

#[test]
fn test_point_kinetics_gamma_reset() {
    let dt = 0.01;
    let params = PointKineticsParams::default();
    let block = PointKineticsBlock::new(params);

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);

    // En equilibrio, verificar que decay heat > 0 (7% de 30 kW es ~2.1 kW)
    let inputs = [0.0, 65.0, 45.0, 0.0, 0.0];
    let mut outputs = vec![0.0; 5];
    block.outputs(0.0, &state, &inputs, &mut outputs);
    assert!(outputs[4] > 1.0, "Decay heat inicial > 1 kW, dio {}", outputs[4]);

    // Activar reset_gamma con modo por defecto ("nominal_power")
    let inputs_reset = [0.0, 65.0, 45.0, 0.0, 1.0];
    state = block.update(dt, &state, &inputs_reset);
    block.outputs(dt, &state, &inputs_reset, &mut outputs);
    assert!((outputs[4] - 2.1).abs() < 0.1, "Decay heat tras reset_gamma nominal_power debe ser ~2.1 kW");

    // Probar modo "clean"
    let mut params_clean = PointKineticsParams::default();
    params_clean.gamma_reset_mode = "clean".to_string();
    let block_clean = PointKineticsBlock::new(params_clean);
    let mut state_clean = vec![0.0; block_clean.num_states()];
    block_clean.get_initial_conditions(&mut state_clean);
    state_clean = block_clean.update(dt, &state_clean, &inputs_reset);
    block_clean.outputs(dt, &state_clean, &inputs_reset, &mut outputs);
    assert_eq!(outputs[4], 0.0, "Decay heat tras reset_gamma clean debe ser 0 kW");
}

#[test]
fn test_interpolate_burnup_reactivity_pcm() {
    use bloques::interpolate_burnup_reactivity_pcm;

    let days = vec![0.0, 3.0, 8.0, 14.0, 19.0, 27.0, 29.5];
    let rho = vec![0.0, 0.0, -304.5, -602.0, -883.0, -1153.0, -1159.0];

    // Puntos exactos de la tabla
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 0.0), 0.0);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 3.0), 0.0);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 8.0), -304.5);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 14.0), -602.0);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 19.0), -883.0);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 27.0), -1153.0);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 29.5), -1159.0);

    // Clamping por debajo y por encima
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, -5.0), 0.0);
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 35.0), -1159.0);

    // Puntos intermedios
    // Entre 0 y 3 días: constante en 0.0
    assert_eq!(interpolate_burnup_reactivity_pcm(&days, &rho, 1.5), 0.0);

    // Entre 3 y 8 días: punto medio en 5.5 días: (0.0 + -304.5)/2 = -152.25 pcm
    let r_5_5 = interpolate_burnup_reactivity_pcm(&days, &rho, 5.5);
    assert!((r_5_5 - (-152.25)).abs() < 1e-4);

    // Entre 8 y 14 días: día 11: midpoint entre -304.5 y -602.0 = -453.25 pcm
    let r_11 = interpolate_burnup_reactivity_pcm(&days, &rho, 11.0);
    assert!((r_11 - (-453.25)).abs() < 1e-4);
}

#[test]
fn test_point_kinetics_burnup_integration_and_feedback() {
    let dt = 1.0; // 1 s
    let mut params = PointKineticsParams::default();
    params.nominal_power_kw = 30000.0; // 30 MW
    params.sample_time_dt = dt;
    params.burnup_enabled = true;
    params.initial_burnup_days = 14.0; // Iniciamos a mitad de ciclo

    let block = PointKineticsBlock::new(params);
    // 1 potencia + 6 precursores + 6 gammas + 1 DPP = 14 estados
    assert_eq!(block.num_states(), 14);
    assert_eq!(block.num_inputs(), 4); // 3 base + 1 burnup
    assert_eq!(block.num_outputs(), 6); // 4 base + 2 burnup

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);
    // DPP inicial debe ser 14.0 días
    assert_eq!(state[13], 14.0);

    let inputs = [0.0, 65.0, 45.0, 0.0, 0.0, f64::NAN];
    let mut outputs = vec![0.0; 6];
    block.outputs(0.0, &state, &inputs, &mut outputs);

    // En 14 días, reactividad por quemado debe ser -602.0 pcm
    let rho_burnup = outputs[4];
    let dpp = outputs[5];
    assert_eq!(dpp, 14.0);
    assert_eq!(rho_burnup, -602.0);
    // Reactividad total inicial con rho_ext = 0 y T = T_ref debe reflejar los -602 pcm
    let rho_total = outputs[2];
    assert!((rho_total - (-602.0)).abs() < 1e-3);

    // Simular integración durante 8640 s (0.1 día) a plena potencia nominal compensando dinámicamente el quemado con barras
    for t in 1..=8640 {
        let u_crit = [-outputs[4], 65.0, 45.0, 0.0, 0.0, f64::NAN];
        state = block.update(t as f64, &state, &u_crit);
        block.outputs(t as f64, &state, &u_crit, &mut outputs);
    }
    let dpp_after = outputs[5];
    // DPP debe haber avanzado exactamente 0.1 días -> 14.1 días
    assert!((dpp_after - 14.1).abs() < 1e-3, "DPP esperado ~14.1, dio {}", dpp_after);

    // Probar override externo de DPP mediante u[5] = 27.0
    let inputs_override = [0.0, 65.0, 45.0, 0.0, 0.0, 27.0];
    state = block.update(8641.0, &state, &inputs_override);
    block.outputs(8641.0, &state, &inputs_override, &mut outputs);
    assert!((outputs[5] - 27.0).abs() < 1e-3, "DPP tras override debe ser ~27.0");
    assert!((outputs[4] - (-1153.0)).abs() < 1e-1, "Reactividad tras override a 27 días debe ser -1153 pcm");
}

#[test]
fn test_point_kinetics_burnup_json_builder() {
    let registry = BlockRegistry::std();
    let json_config = json!({
        "nominal_power_kw": 30000.0,
        "sample_time_dt": 0.1,
        "burnup_enabled": true,
        "initial_burnup_days": 8.0,
        "burnup_table_days": [0.0, 3.0, 8.0, 14.0, 19.0, 27.0, 29.5],
        // Especificada en delta k/k adimensional (* 1e-5)
        "burnup_table_reactivity": [0.0, 0.0, -0.003045, -0.00602, -0.00883, -0.01153, -0.01159]
    });

    let block = registry.build("PointKinetics", json_config).expect("Debe construir PointKinetics con quemado");
    assert_eq!(block.num_states(), 14); // 1 + 6 precursores + 6 gammas + 1 DPP

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);
    assert_eq!(state[13], 8.0); // 8 días iniciales

    let inputs = [0.0, 65.0, 45.0];
    let mut outputs = vec![0.0; 6];
    block.outputs(0.0, &state, &inputs, &mut outputs);
    assert_eq!(outputs[5], 8.0);
    // Reactividad a 8 días: -304.5 pcm
    assert!((outputs[4] - (-304.5)).abs() < 1e-2);
}

