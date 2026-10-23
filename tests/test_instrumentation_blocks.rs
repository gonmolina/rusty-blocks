use bloques::{
    SensorBlock, SensorFault, SensorQuality,
    ActuatorBlock, ActuatorFault, FailMode,
    blocks::Block,
};

#[test]
fn test_sensor_first_order_lag_and_conversion() {
    let dt = 0.01;
    let tau = 0.1; // Constante de tiempo de 100 ms
    let sensor = SensorBlock::new(
        "1000-PT-0001.00",
        "Presion Plenum",
        "node.plenum_inferior.pressure",
        "kPa",
        0.0,
        500.0,
        tau,
        dt,
    );

    let mut state = vec![0.0];
    sensor.get_initial_conditions(&mut state);
    assert_eq!(state[0], 0.0);

    let mut outputs = vec![0.0, 0.0];
    sensor.outputs(0.0, &state, &[0.0], &mut outputs);
    assert_eq!(outputs[0], 0.0);
    assert_eq!(outputs[1], 4.0); // 4 mA para 0 kPa

    // Aplicar escalón de 250 kPa (50% de escala -> 12 mA)
    let step_input = [250.0];
    for t_step in 1..=50 {
        let t = t_step as f64 * dt;
        state = sensor.update(t, &state, &step_input);
    }

    sensor.outputs(0.5, &state, &step_input, &mut outputs);
    // Para t = 0.5 s (5 * tau), la salida debe haber alcanzado > 99% de 250 kPa
    assert!(outputs[0] > 247.0 && outputs[0] <= 250.0);
    assert!((outputs[1] - 12.0).abs() < 0.2); // ~12 mA
}

#[test]
fn test_sensor_fault_injection() {
    let dt = 0.01;
    let mut sensor = SensorBlock::new(
        "1000-PT-0001.00",
        "Presion Plenum",
        "node.plenum_inferior.pressure",
        "kPa",
        0.0,
        500.0,
        0.05,
        dt,
    );

    let state = vec![200.0];
    let mut outputs = vec![0.0, 0.0];

    // 1. Sin falla
    sensor.outputs(1.0, &state, &[200.0], &mut outputs);
    assert_eq!(outputs[0], 200.0);
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Good);

    // 2. Falla: Stuck en 50 kPa
    sensor.set_fault(SensorFault::Stuck(50.0));
    sensor.outputs(2.0, &state, &[200.0], &mut outputs);
    assert_eq!(outputs[0], 50.0);
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Faulted);

    // 3. Falla: Open Circuit (rotura de lazo)
    sensor.set_fault(SensorFault::OpenCircuit);
    sensor.outputs(3.0, &state, &[200.0], &mut outputs);
    assert_eq!(outputs[1], 0.0); // 0 mA
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Bad);

    // 4. Falla: Short Circuit
    sensor.set_fault(SensorFault::ShortCircuit);
    sensor.outputs(4.0, &state, &[200.0], &mut outputs);
    assert_eq!(outputs[1], 24.0); // 24 mA
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Bad);

    // 5. Falla: Deriva temporal (Drift)
    sensor.set_fault(SensorFault::Drift {
        rate_per_sec: 10.0,
        max_drift: 100.0,
        start_time: 10.0,
    });
    sensor.outputs(15.0, &state, &[200.0], &mut outputs); // dt_falla = 5s -> drift = 50 kPa
    assert!((outputs[0] - 250.0).abs() < 1e-3);
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Faulted);
}

#[test]
fn test_actuator_slew_rate_and_deadband() {
    let dt = 0.1;
    let travel_time = 10.0; // 10 segundos de 0 a 100% (0.1 por segundo)
    let actuator = ActuatorBlock::new(
        "1000-CV-0001.00",
        "Valvula de Control",
        "pipe.0.valve_opening",
        travel_time,
        dt,
    ).with_initial_pos(0.0).with_deadband(0.02); // 2% deadband

    let mut state = vec![0.0];
    actuator.get_initial_conditions(&mut state);

    // 1. Demanda pequeña (1% < deadband 2%) -> no debe moverse
    let demand_small = [0.01];
    state = actuator.update(0.1, &state, &demand_small);
    assert_eq!(state[0], 0.0);

    // 2. Demanda de 100% (1.0)
    let demand_full = [1.0];
    for t_step in 1..=50 {
        let t = t_step as f64 * dt;
        state = actuator.update(t, &state, &demand_full);
    }
    // A t = 5.0 s (50 pasos de 0.1s), la válvula debe haber recorrido exactamente el 50%
    let mut outputs = vec![0.0];
    actuator.outputs(5.0, &state, &demand_full, &mut outputs);
    assert!((outputs[0] - 0.50).abs() < 1e-3);

    // Continuar hasta 10 segundos
    for t_step in 51..=100 {
        let t = t_step as f64 * dt;
        state = actuator.update(t, &state, &demand_full);
    }
    actuator.outputs(10.0, &state, &demand_full, &mut outputs);
    // Debido a la zona muerta (2%), el actuador se detiene al entrar en [1.0 - deadband, 1.0]
    assert!(outputs[0] >= 1.0 - actuator.deadband && outputs[0] <= 1.0);
}

#[test]
fn test_actuator_fault_modes() {
    let dt = 0.1;
    let mut actuator = ActuatorBlock::new(
        "1000-CV-0001.00",
        "Valvula de Control",
        "pipe.0.valve_opening",
        10.0,
        dt,
    ).with_initial_pos(0.8);

    let mut state = vec![0.8];
    let demand = [1.0];

    // 1. Falla Jam (atascada al 35%)
    actuator.set_fault(ActuatorFault::Jam(0.35));
    state = actuator.update(0.1, &state, &demand);
    assert_eq!(state[0], 0.35);

    // 2. Falla PowerLoss (FailClose con resorte a 0.0)
    actuator.set_fault(ActuatorFault::PowerLoss(FailMode::FailClose));
    // Tiempo de retorno por resorte = 5.0 s (rate = 0.1 / 5.0 = 0.02 por paso)
    state = vec![0.50];
    for t_step in 1..=25 { // 2.5 s
        let t = t_step as f64 * dt;
        state = actuator.update(t, &state, &demand);
    }
    // Debe haber bajado de 0.50 a 0.0 (o estar en 0.0)
    assert!(state[0] <= 0.01);
}

#[test]
fn test_block_registry_json_instantiation() {
    use bloques::blocks::BlockRegistry;
    use serde_json::json;

    let registry = BlockRegistry::std();

    // 1. Build Sensor from JSON
    let sensor_json = json!({
        "tag": "1000-PT-0001.00",
        "description": "Presion Plenum",
        "source_signal": "node.plenum_inferior.pressure",
        "eu_unit": "kPa",
        "min_eu": 0.0,
        "max_eu": 500.0,
        "time_constant_tau": 0.1,
        "sample_time_dt": 0.01,
        "noise_std_dev": 0.5,
        "noise_enabled": true
    });
    let sensor_block = registry.build("Sensor", sensor_json).expect("Sensor block should build");
    assert_eq!(sensor_block.num_inputs(), 1);
    assert_eq!(sensor_block.num_outputs(), 2);
    assert_eq!(sensor_block.sample_time(), Some(0.01));

    // 2. Build Actuator from JSON
    let act_json = json!({
        "tag": "1000-CV-0001.00",
        "description": "Valvula Control",
        "target_signal": "pipe.hx_tubos_c.valve_opening",
        "travel_time_s": 5.0,
        "sample_time_dt": 0.01,
        "initial_position": 0.5,
        "deadband": 0.01
    });
    let act_block = registry.build("Actuator", act_json).expect("Actuator block should build");
    assert_eq!(act_block.num_inputs(), 1);
    assert_eq!(act_block.num_outputs(), 1);
    assert_eq!(act_block.sample_time(), Some(0.01));
}

#[test]
fn test_sensor_namur_ne43_clamping_and_saturation() {
    let dt = 0.01;
    let tau = 0.001; // respuesta casi instantánea
    let mut sensor = SensorBlock::new(
        "1090-FIT-103102",
        "Caudal entrada nucleo",
        "pipe.canal_nucleo.flow",
        "m3/h",
        0.0,
        3500.0,
        tau,
        dt,
    );

    // 1. Rango nominal: 0.0 m3/h -> 4.0 mA, Good
    assert_eq!(sensor.eu_to_ma(0.0), 4.0);
    assert_eq!(sensor.current_quality(0.0), SensorQuality::Good);

    // 2. Rango nominal: 1750.0 m3/h (50%) -> 12.0 mA, Good
    assert_eq!(sensor.eu_to_ma(1750.0), 12.0);
    assert_eq!(sensor.current_quality(1750.0), SensorQuality::Good);

    // 3. Fondo de escala exacto: 3500.0 m3/h (100%) -> 20.0 mA, Good
    assert_eq!(sensor.eu_to_ma(3500.0), 20.0);
    assert_eq!(sensor.current_quality(3500.0), SensorQuality::Good);

    // 4. Sobre-escala moderada: 3550.0 m3/h -> saturación lineal < 20.5 mA
    let ma_3550 = sensor.eu_to_ma(3550.0);
    assert!(ma_3550 > 20.0 && ma_3550 < 20.5);
    assert_eq!(sensor.current_quality(3550.0), SensorQuality::Bad);

    // 5. Sobre-escala severa: 4000.0 m3/h -> saturación fija a 20.5 mA (NAMUR NE 43), valor EU saturado en 3609.375 m3/h y calidad Bad
    assert_eq!(sensor.eu_to_ma(4000.0), 20.5);
    assert_eq!(sensor.current_quality(4000.0), SensorQuality::Bad);
    let mut outputs_4000 = vec![0.0, 0.0];
    let state_4000 = vec![4000.0];
    sensor.outputs(1.0, &state_4000, &[4000.0], &mut outputs_4000);
    assert_eq!(outputs_4000[1], 20.5); // Corriente saturada en 20.5 mA
    assert_eq!(outputs_4000[0], 3609.375); // El contador de entrada satura en el valor de 20.5 mA (3609.375 m3/h)
    assert_eq!(sensor.current_quality(outputs_4000[0]), SensorQuality::Bad);

    // 6. Sub-escala moderada: -20.0 m3/h -> saturación lineal > 3.8 mA
    let ma_neg = sensor.eu_to_ma(-20.0);
    assert!(ma_neg >= 3.8 && ma_neg < 4.0);
    assert_eq!(sensor.current_quality(-20.0), SensorQuality::Bad);

    // 7. Sub-escala severa: -200.0 m3/h -> saturación fija a 3.8 mA (NAMUR NE 43), valor EU saturado en -43.75 m3/h y calidad Bad
    assert_eq!(sensor.eu_to_ma(-200.0), 3.8);
    assert_eq!(sensor.current_quality(-200.0), SensorQuality::Bad);
    let mut outputs_neg = vec![0.0, 0.0];
    let state_neg = vec![-200.0];
    sensor.outputs(1.0, &state_neg, &[-200.0], &mut outputs_neg);
    assert_eq!(outputs_neg[1], 3.8);
    assert!((outputs_neg[0] - (-43.75)).abs() < 1e-4); // El contador de entrada satura en el valor de 3.8 mA (-43.75 m3/h)
    assert_eq!(sensor.current_quality(outputs_neg[0]), SensorQuality::Bad);

    // 8. Falla explícita OpenCircuit: conserva 0.0 mA (fuera de zona NAMUR < 3.6 mA)
    sensor.set_fault(SensorFault::OpenCircuit);
    let mut outputs = vec![0.0, 0.0];
    let state = vec![0.0];
    sensor.outputs(1.0, &state, &[0.0], &mut outputs);
    assert_eq!(outputs[1], 0.0);
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Bad);

    // 9. Falla explícita ShortCircuit: conserva 24.0 mA (fuera de zona NAMUR > 21.0 mA)
    sensor.set_fault(SensorFault::ShortCircuit);
    sensor.outputs(2.0, &state, &[0.0], &mut outputs);
    assert_eq!(outputs[1], 24.0);
    assert_eq!(sensor.current_quality(outputs[0]), SensorQuality::Bad);
}

