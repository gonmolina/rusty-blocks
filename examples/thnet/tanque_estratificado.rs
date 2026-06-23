/// Ejemplo THNet: Tanque Estratificado con Calefacción y Caudal de Fuga Variable
///
/// ## Sistema simulado
///
/// ```text
///                     [Entrada: 10 kg/s, 20°C] (Pipe a h = 18m)
///                              │
///                              ▼
///              ┌───────────────────────────────┐  ▲
///              │                               │  │
///              │   Tanque Estratificado        │  │
///              │   Area = 4.9 m²               │  │ Nivel máximo: 20 m
///              │   Nivel inicial: 19 m         │  │
///              │                               │  │
///              │   Calefactor a h = 2m (10 kW) │  │
///              │                               │  │
///              └───────────────────────────────┘  ▼
///                  │                       │
///                  ▼                       ▼
///          [Salida 1: 10 kg/s]      [Salida 2: Fuga con Válvula]
///                                   (Apertura a t=1000s, inicial 10 kg/s)
/// ```

use bloques::thnet::{
    network::{Network, Node, Pipe},
    solver::Solver,
};
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() {
    // ─────────────────────────────────────────────────────────────────────
    // PARÁMETROS DEL TANQUE Y LA RED
    // ─────────────────────────────────────────────────────────────────────

    let area_tank    = 4.9_f64;        // [m²] sección transversal del tanque
    let height_max   = 20.0_f64;       // [m] altura máxima del tanque
    let mut level    = 19.0_f64;       // [m] nivel inicial de agua
    let t_init_k     = 20.0 + 273.15;  // [K] 20 °C temperatura inicial uniforme
    let t_inlet_k    = 20.0 + 273.15;  // [K] 20 °C temperatura de entrada
    let p_atm        = 1.0e5_f64;      // [Pa] 1 bar presión de gas (abierto)
    let g            = 9.80665_f64;    // [m/s²] gravedad

    // Caudales nominales
    let flow_in      = 10.0_f64;       // [kg/s] caudal constante de entrada
    let flow_out1    = 10.0_f64;       // [kg/s] caudal constante de salida 1

    // Discretización de capas térmicas en el tanque
    let n_layers     = 20_usize;
    let dz           = height_max / n_layers as f64; // [m] 1.0 m por capa
    let mut tank_temp = vec![t_init_k; n_layers];    // perfil de temperaturas [K]

    // Características de la válvula en la línea de fuga
    // Se calcula para dar un caudal de 10 kg/s a t = 1000s (L = 19.0m)
    // dP = rho * g * L_1000 = 998.2 * 9.80665 * 19.0 = 185987 Pa
    // W = Cv * 2.4026e-5 * sqrt(rho * dP) => Cv = W / (2.4026e-5 * sqrt(rho * dP))
    let rho_init     = 998.2_f64;
    let dp_valve_init = rho_init * g * level;
    let valve_cv     = 10.0_f64 / (2.4026e-5_f64 * (rho_init * dp_valve_init).sqrt());
    let mut valve_opening = 0.0_f64;   // Inicialmente cerrada

    // Parámetros de simulación
    let dt           = 0.2_f64;        // [s] paso de tiempo
    let t_final      = 2000.0_f64;     // [s] simulamos hasta 2000 segundos
    let n_steps      = (t_final / dt) as usize;
    let print_every  = (100.0 / dt) as usize; // imprimir cada 100 s de tiempo de simulación

    println!("┌─────────────────────────────────────────────────┐");
    println!("│   THNet — Tanque Estratificado con Fuga         │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Área Tanque:     {:>8.2} m²                    │", area_tank);
    println!("│ Altura Máxima:   {:>8.1} m                     │", height_max);
    println!("│ Nivel Inicial:   {:>8.2} m                     │", level);
    println!("│ Temp. Inicial:   {:>8.1} °C                     │", t_init_k - 273.15);
    println!("│ Calefactor:      10.0 kW a h = 2.0 m            │");
    println!("│ Entrada:         10.0 kg/s a h = 18.0 m         │");
    println!("│ Salida 1:        10.0 kg/s a la base            │");
    println!("│ Válvula Fuga Cv: {:>8.2} (abre a t=1000s)       │", valve_cv);
    println!("│ Paso de tiempo:  {:>8.2} s                     │", dt);
    println!("└─────────────────────────────────────────────────┘");
    println!();

    // ─────────────────────────────────────────────────────────────────────
    // CONSTRUCCIÓN DE LA RED
    // ─────────────────────────────────────────────────────────────────────

    let mut net = Network::new();

    // Nodo 0: Descarga 1 (Presión de contorno a 1 bar)
    let id_disch1 = net.add_node(
        Node::new(t_init_k, p_atm, 0.001).with_fixed_pressure(),
    );

    // Nodo 1: Base del Tanque (Presión de contorno hidrostática variable)
    let p_bottom_init = p_atm + rho_init * g * level;
    let id_tank_bottom = net.add_node(
        Node::new(t_init_k, p_bottom_init, 0.001).with_fixed_pressure(),
    );

    // Nodo 2: Descarga 2 (Fuga) (Presión de contorno a 1 bar)
    let id_disch2 = net.add_node(
        Node::new(t_init_k, p_atm, 0.001).with_fixed_pressure(),
    );

    // Cañería 1: Base del Tanque -> Descarga 1 (Caudal controlado, nominal 10 kg/s)
    let id_pipe1 = net.add_pipe(
        Pipe::new(id_tank_bottom, id_disch1, 0.25, 5.0, 1.5e-5, 0.0, 1, t_init_k),
    );

    // Cañería 2: Base del Tanque -> Descarga 2 (Línea de fuga con Válvula de Control)
    let id_pipe2 = net.add_pipe(
        Pipe::new(id_tank_bottom, id_disch2, 0.25, 5.0, 1.5e-5, 0.0, 1, t_init_k)
            .with_valve(valve_cv, valve_opening),
    );

    // ─────────────────────────────────────────────────────────────────────
    // SOLVEDOR Y SIMULACIÓN
    // ─────────────────────────────────────────────────────────────────────

    let mut solver = Solver::new();

    std::fs::create_dir_all("results").expect("No se pudo crear results/");
    let file = File::create("results/tanque_estratificado.csv").unwrap();
    let mut w_csv = BufWriter::new(file);

    writeln!(
        w_csv,
        "t_s,level_m,flow_in_kg_s,flow_out1_kg_s,flow_leak_kg_s,T_layer0_C,T_layer1_C,T_layer2_C,T_layer5_C,T_layer8_C,T_layer11_C,T_layer14_C,T_layer17_C,T_layer19_C"
    )
    .unwrap();

    println!(
        "{:>6} {:>8} {:>8} {:>8} {:>8} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "t[s]", "Nivel[m]", "W_in", "W_out1", "W_fuga", "T_c0", "T_c1", "T_c2", "T_c5", "T_c8", "T_c11", "T_c14", "T_c17", "T_c19"
    );
    println!("{}", "─".repeat(110));

    let t_start = std::time::Instant::now();

    for step in 0..n_steps {
        let t = solver.time;

        // 1. Manejo del estado de la válvula
        if t >= 1000.0_f64 {
            valve_opening = 1.0_f64;
        } else {
            valve_opening = 1e-6_f64; // Prácticamente cerrada
        }
        net.pipes[id_pipe2].valve_opening = valve_opening;

        // 2. Actualizar la temperatura y presión hidrostática del nodo de base del tanque
        let t_bottom = tank_temp[0];
        net.nodes[id_tank_bottom].temperature = t_bottom;
        net.nodes[id_tank_bottom].h = bloques::thnet::thermo::enthalpy(t_bottom);
        let rho_bottom = net.nodes[id_tank_bottom].density();
        let p_bottom = p_atm + rho_bottom * g * level;
        net.nodes[id_tank_bottom].pressure = p_bottom;
        net.nodes[id_tank_bottom].fixed_pressure = Some(p_bottom);

        // 3. Ejecutar paso hidráulico y térmico de la red
        solver.step(&mut net, dt);

        // Forzar caudal de la salida 1 a ser 10 kg/s si el nivel es > 0, o 0 si está vacío
        if level > 0.001 {
            net.pipes[id_pipe1].flow = flow_out1;
        } else {
            net.pipes[id_pipe1].flow = 0.0;
        }

        let w_out1 = net.pipes[id_pipe1].flow;
        let w_leak = net.pipes[id_pipe2].flow.max(0.0);
        let w_out_total = w_out1 + w_leak;

        // 4. Integrar nivel de agua del tanque
        let level_prev = level;
        let dm = (flow_in - w_out_total) * dt;
        let current_mass = level * area_tank * rho_bottom;
        let new_mass = (current_mass + dm).max(0.0);
        level = new_mass / (area_tank * rho_bottom);
        level = level.clamp(0.0, height_max);

        // 5. Actualizar el perfil estratificado de temperaturas mediante balance advectivo 1D
        let k_top = ((level / dz).floor() as usize).min(n_layers - 1);

        // Caudal vertical descendente en cada capa (balance de arriba hacia abajo)
        let mut w_down = vec![0.0; n_layers];
        for i in 0..n_layers {
            if i > 17 {
                w_down[i] = (w_out_total - flow_in).max(0.0);
            } else {
                w_down[i] = w_out_total;
            }
        }

        // Masas de control en cada capa térmica en paso k y k+1
        let mut m_old = vec![0.0; n_layers];
        let mut m_new = vec![0.0; n_layers];

        for i in 0..n_layers {
            let z_i = i as f64 * dz;
            if level_prev > z_i {
                let h_wet = (level_prev - z_i).min(dz);
                m_old[i] = h_wet * area_tank * bloques::thnet::thermo::density(tank_temp[i]);
            }
            if level > z_i {
                let h_wet = (level - z_i).min(dz);
                m_new[i] = h_wet * area_tank * bloques::thnet::thermo::density(tank_temp[i]);
            }
        }

        // Balance térmico implícito upwind de arriba hacia abajo
        let tank_h = tank_temp.iter().map(|&temp| bloques::thnet::thermo::enthalpy(temp)).collect::<Vec<_>>();
        let mut tank_h_new = tank_h.clone();

        for i in (0..=k_top).rev() {
            let m_o = m_old[i];

            let w_in_from_top = if i == k_top { 0.0 } else { w_down[i + 1] };
            let h_in_from_top = if i == k_top { 0.0 } else { tank_h_new[i + 1] };

            // Calor del calefactor (10 kW inyectados en la capa 1, correspondiente a h = 1m a 2m)
            let mut q_src = 0.0;
            if i == 1 && level > 1.0 {
                q_src += 10000.0; // 10 kW
            } else if i == k_top && level <= 1.0 && level > 0.001 {
                q_src += 10000.0; // Si el nivel baja mucho, el calor se concentra en la última capa mojada
            }

            // Aporte de la entrada de agua (10 kg/s a h = 18m, capa 17)
            let mut w_src = 0.0;
            let mut h_src = 0.0;
            if level >= 18.0 && i == 17 {
                w_src = flow_in;
                h_src = bloques::thnet::thermo::enthalpy(t_inlet_k);
            } else if level < 18.0 && i == k_top {
                // Si el nivel está por debajo de 18m, el agua cae en la capa libre superior
                w_src = flow_in;
                h_src = bloques::thnet::thermo::enthalpy(t_inlet_k);
            }

            // Ecuación de conservación de entalpía implícita (normalizada con flujos de entrada)
            let denom = m_o + dt * (w_in_from_top + w_src);
            let num = m_o * tank_h[i] + dt * (w_in_from_top * h_in_from_top + q_src + w_src * h_src);
            tank_h_new[i] = num / denom;
        }

        // Actualizar el perfil de temperaturas a partir de las entalpías
        for i in 0..=k_top {
            tank_temp[i] = bloques::thnet::thermo::temperature_from_enthalpy(tank_h_new[i]);
        }
        // Capas expuestas / secas vuelven a la temperatura ambiente inicial
        for i in (k_top + 1)..n_layers {
            tank_temp[i] = t_init_k;
        }

        // 6. Escribir resultados en CSV
        writeln!(
            w_csv,
            "{:.2},{:.6},{:.6},{:.6},{:.6},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}",
            t,
            level,
            flow_in,
            w_out1,
            w_leak,
            tank_temp[0] - 273.15,
            tank_temp[1] - 273.15,
            tank_temp[2] - 273.15,
            tank_temp[5] - 273.15,
            tank_temp[8] - 273.15,
            tank_temp[11] - 273.15,
            tank_temp[14] - 273.15,
            tank_temp[17] - 273.15,
            tank_temp[19] - 273.15
        )
        .unwrap();

        // 7. Imprimir en pantalla periódicamente
        if step % print_every == 0 || step == n_steps - 1 {
            println!(
                "{:>6.1} {:>8.4} {:>8.4} {:>8.4} {:>8.4} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2}",
                t,
                level,
                flow_in,
                w_out1,
                w_leak,
                tank_temp[0] - 273.15,
                tank_temp[1] - 273.15,
                tank_temp[2] - 273.15,
                tank_temp[5] - 273.15,
                tank_temp[8] - 273.15,
                tank_temp[11] - 273.15,
                tank_temp[14] - 273.15,
                tank_temp[17] - 273.15,
                tank_temp[19] - 273.15
            );
        }
    }

    w_csv.flush().unwrap();
    let elapsed = t_start.elapsed();

    println!("{}", "─".repeat(95));
    println!();
    println!("Simulación completada en {:.2?}", elapsed);
    println!("Estado final (t = {:.1} s):", solver.time);
    println!("  Nivel final: {:.4} m", level);
    println!("  Temperatura Capa 0 (Base): {:.4} °C", tank_temp[0] - 273.15);
    println!("  Temperatura Capa 1 (Calefactor 2m): {:.4} °C", tank_temp[1] - 273.15);
    println!("  Temperatura Capa 17 (Nozzle 18m): {:.4} °C", tank_temp[17] - 273.15);
    println!("  Caudal de fuga final: {:.4} kg/s", net.pipes[id_pipe2].flow);
    println!();
    println!("Resultados guardados en results/tanque_estratificado.csv");
}
