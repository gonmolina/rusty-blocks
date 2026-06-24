use bloques::blocks::fuel_thermal::{FuelThermalBlock, FuelThermalParams, interpolate_cpm, doppler_weights};
use bloques::blocks::Block;

#[test]
fn test_cpm_interpolation() {
    let t_tab = [15.0, 60.0, 120.0, 400.0];
    let cpm_tab = [532.4689562, 544.5712990, 560.6269137, 635.6860432];

    assert_eq!(interpolate_cpm(&t_tab, &cpm_tab, 10.0), 532.4689562);
    assert_eq!(interpolate_cpm(&t_tab, &cpm_tab, 15.0), 532.4689562);
    assert_eq!(interpolate_cpm(&t_tab, &cpm_tab, 60.0), 544.5712990);
    assert_eq!(interpolate_cpm(&t_tab, &cpm_tab, 450.0), 635.6860432);

    let mid = interpolate_cpm(&t_tab, &cpm_tab, 37.5);
    let expected = 0.5 * (532.4689562 + 544.5712990);
    assert!((mid - expected).abs() < 1e-4);
}

#[test]
fn test_doppler_weights() {
    let apd = [0.2926748577205830, 0.4340588756881160, 0.2732662665913000];
    let w = doppler_weights(&apd);
    assert_eq!(w.len(), 3);
    assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-6);
    // El nodo central (pico) debe tener el mayor peso (~54.0%)
    assert!(w[1] > 0.53 && w[1] < 0.55);
}

#[test]
fn test_fuel_thermal_steady_state_equilibrium() {
    let params = FuelThermalParams::default();
    let block = FuelThermalBlock::new(params.clone());

    assert_eq!(block.num_states(), 6);
    assert_eq!(block.num_inputs(), 4);
    assert_eq!(block.num_outputs(), 9);

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);

    // Comprobar temperaturas iniciales calculadas con el script de Simulink
    assert!((state[0] - 68.52).abs() < 0.1, "Tf0 esperado ~68.52, dio {}", state[0]);
    assert!((state[1] - 83.29).abs() < 0.1, "Tf1 esperado ~83.29, dio {}", state[1]);
    assert!((state[2] - 71.43).abs() < 0.1, "Tf2 esperado ~71.43, dio {}", state[2]);

    // Comprobar HTCs iniciales (~2.9e5 a 3.1e5 W/K)
    assert!(state[3] > 2.8e5 && state[3] < 3.2e5);
    assert!(state[4] > 2.8e5 && state[4] < 3.3e5);
    assert!(state[5] > 2.8e5 && state[5] < 3.2e5);

    let inputs = [params.nominal_power_w, params.nominal_flow_kg_s, params.t_inlet_ref_c, f64::NAN];
    let mut outputs = vec![0.0; block.num_outputs()];
    block.outputs(0.0, &state, &inputs, &mut outputs);

    let t_doppler = outputs[0];
    let t_max = outputs[1];
    let vel = outputs[7];

    assert!((t_doppler - 77.12).abs() < 0.2, "T_doppler inicial esperada ~77.12, dio {}", t_doppler);
    assert!((t_max - 83.29).abs() < 0.1, "T_max esperado ~83.29, dio {}", t_max);
    assert!((vel - 12.5).abs() < 1e-3, "Velocidad nominal debe ser 12.5 m/s");

    // Simular durante 100 pasos en estado estacionario: debe mantenerse perfectamente invariante
    let mut current_state = state.clone();
    for step in 1..=100 {
        current_state = block.update(step as f64 * params.sample_time_dt, &current_state, &inputs);
    }

    for i in 0..3 {
        assert!(
            (current_state[i] - state[i]).abs() < 1e-3,
            "Tf[{}] divergió en estacionario: inicial {}, final {}",
            i, state[i], current_state[i]
        );
    }
    for i in 3..6 {
        assert!(
            (current_state[i] - state[i]).abs() / state[i] < 1e-4,
            "HTC[{}] divergió en estacionario: inicial {}, final {}",
            i - 3, state[i], current_state[i]
        );
    }
}

#[test]
fn test_fuel_thermal_power_step_response() {
    let params = FuelThermalParams::default();
    let block = FuelThermalBlock::new(params.clone());

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);

    let t_doppler_init = {
        let mut out = vec![0.0; 9];
        let u = [params.nominal_power_w, params.nominal_flow_kg_s, params.t_inlet_ref_c, f64::NAN];
        block.outputs(0.0, &state, &u, &mut out);
        out[0]
    };

    // Aplicar escalón de potencia de +10% (31.35 MW)
    let p_step = params.nominal_power_w * 1.10;
    let u_step = [p_step, params.nominal_flow_kg_s, params.t_inlet_ref_c, f64::NAN];

    let mut current_state = state;
    for step in 1..=200 { // 10 segundos
        current_state = block.update(step as f64 * params.sample_time_dt, &current_state, &u_step);
    }

    let mut out = vec![0.0; 9];
    block.outputs(10.0, &current_state, &u_step, &mut out);
    let t_doppler_new = out[0];

    // La temperatura Doppler debe haber aumentado
    assert!(t_doppler_new > t_doppler_init + 3.0, "T_doppler debe subir ante escalón de potencia");
    assert!(t_doppler_new < t_doppler_init + 6.0, "T_doppler debe estabilizarse cerca de +3 a +5 °C");
}

#[test]
fn test_fuel_thermal_flow_coastdown() {
    let params = FuelThermalParams::default();
    let block = FuelThermalBlock::new(params.clone());

    let mut state = vec![0.0; block.num_states()];
    block.get_initial_conditions(&mut state);

    // Reducir caudal a 100 kg/s (coastdown severo)
    let u_coast = [params.nominal_power_w, 100.0, params.t_inlet_ref_c, f64::NAN];

    let mut current_state = state;
    for step in 1..=100 { // 5 segundos
        current_state = block.update(step as f64 * params.sample_time_dt, &current_state, &u_coast);
    }

    let mut out = vec![0.0; 9];
    block.outputs(5.0, &current_state, &u_coast, &mut out);

    let htc_coast = out[6];
    let vel_coast = out[7];

    assert!((vel_coast - (100.0 / 881.33 * 12.5)).abs() < 0.1);
    // El HTC promedio debe haberse reducido drásticamente (a menos de la mitad)
    assert!(htc_coast < 1.5e5, "HTC debe caer notablemente por bajo caudal");
    // La temperatura del combustible debe haber aumentado por la mayor resistencia térmica
    assert!(out[1] > 100.0, "T_max debe superar los 100 °C con 100 kg/s a plena potencia");
}
