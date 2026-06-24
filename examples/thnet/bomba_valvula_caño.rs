/// Ejemplo THNet: Circuito Abierto con Bomba, Válvula de Control y Tubería Calefaccionada
///
/// ## Sistema simulado
///
/// ```text
///  [Nodo 0: Entrada] ──(Bomba)──> [Nodo 1] ──(Válvula)──> [Nodo 2] ──(Tubería Calentada)──> [Nodo 3: Salida]
///     (1e5 Pa, 47.5°C)                                                      (10 kW a la pared)     (1e5 Pa)
/// ```
///
/// ## Componentes:
/// - **Bomba Centrífuga**: Curva H-Q cuadrática con H_max = 22.4m (220 kPa) y W_max = 10.0 kg/s.
/// - **Válvula de Control**: Cv = 5.0, apertura = 1.0 (totalmente abierta).
/// - **Tubería**: Diámetro 25 cm, longitud 26 m, pared de inox 304L de 1 mm (masa = 164.0 kg, cp = 500 J/kgK, UA = 1500 W/K).
/// - **Calentamiento**: 10 kW constantes inyectados en la pared de la tubería.

use bloques::thnet::{
    network::{Network, Node, Pipe},
    output::{CsvRecorder, RecordSelector},
    solver::Solver,
};

fn main() {
    // ─────────────────────────────────────────────────────────────────────
    // PARÁMETROS DEL SISTEMA
    // ─────────────────────────────────────────────────────────────────────

    let diametro_m   = 0.25_f64;       // [m] 25 cm diámetro interior
    let rugosidad_m  = 1.5e-5_f64;     // [m] acero inoxidable 304L
    let q_source_w   = 10_000.0_f64;   // [W] potencia del calentador en la pared
    let n_cells      = 10_usize;       // celdas 1D en el caño
    let v_header_m3  = 0.001_f64;      // [m³] headers de interconexión (1 L)

    // Condiciones de contorno
    let t_init_k     = 47.5 + 273.15;  // [K] 47.5 °C temperatura inicial y de entrada
    let p_boundary   = 1.0e5_f64;      // [Pa] 1 bar presión de entrada y salida

    // Características de la pared sólida (1 mm de espesor, 26 m de largo, inox 304L)
    let wall_mass_kg = 164.0_f64;
    let wall_cp_j_kg = 500.0_f64;
    let wall_ua_w_k  = 1500.0_f64;

    // Características de la bomba (H_max = 22.4 m col. agua, W_max = 10.0 kg/s)
    let pump_dp_max  = 220_000.0_f64;  // [Pa] ~ 22.4 m
    let pump_w_max   = 10.0_f64;       // [kg/s]

    // Características de la válvula (Cv = 5.0, apertura = 1.0)
    let valve_cv     = 5.0_f64;
    let valve_opening = 1.0_f64;

    // Parámetros de simulación
    let dt       = 0.2_f64;            // [s] paso de tiempo
    let t_final  = 1800.0_f64;         // [s] 30 min de simulación
    let n_steps  = (t_final / dt) as usize;
    let print_every = (60.0 / dt) as usize; // imprimir cada 60 s

    println!("┌─────────────────────────────────────────────────┐");
    println!("│   THNet — Bomba, Válvula de Control y Caño       │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Entrada/Salida:  {:>8.1} bar                    │", p_boundary / 1e5);
    println!("│ Temp. Entrada:   {:>8.1} °C                     │", t_init_k - 273.15);
    println!("│ Diámetro caño:   {:>8.1} mm                    │", diametro_m * 1000.0);
    println!("│ Longitud caño:   {:>8.1} m                     │", 26.0);
    println!("│ Espesor pared:   {:>8.1} mm                    │", 1.0);
    println!("│ Masa pared:      {:>8.1} kg (Inox 304L)         │", wall_mass_kg);
    println!("│ UA pared-fluido: {:>8.1} W/K                    │", wall_ua_w_k);
    println!("│ Potencia pared:  {:>8.1} kW                    │", q_source_w / 1000.0);
    println!("│ Bomba H_max:     {:>8.1} m (220 kPa)            │", 22.4);
    println!("│ Válvula Cv:      {:>8.1} (Apertura 1.0)         │", valve_cv);
    println!("└─────────────────────────────────────────────────┘");
    println!();

    // ─────────────────────────────────────────────────────────────────────
    // CONSTRUCCIÓN DE LA RED
    // ─────────────────────────────────────────────────────────────────────

    let mut net = Network::new();

    // Nodo 0 — Entrada (Condición de contorno Dirichlet de Presión y Temp)
    let id_in = net.add_node(
        Node::new(t_init_k, p_boundary, v_header_m3).with_fixed_pressure(),
    );

    // Nodo 1 — Entre bomba y válvula (Presión libre)
    let id_node1 = net.add_node(
        Node::new(t_init_k, p_boundary + pump_dp_max, v_header_m3),
    );

    // Nodo 2 — Entre válvula y caño (Presión libre)
    let id_node2 = net.add_node(
        Node::new(t_init_k, p_boundary + 10_000.0, v_header_m3),
    );

    // Nodo 3 — Salida (Condición de contorno Dirichlet de Presión)
    let id_out = net.add_node(
        Node::new(t_init_k, p_boundary, v_header_m3).with_fixed_pressure(),
    );

    // Rama 0: Conexión Nodo 0 -> Nodo 1 con Bomba
    // Usamos un tramo corto de caño para representar la bomba
    net.add_pipe(
        Pipe::new(id_in, id_node1, diametro_m, 0.5, rugosidad_m, 0.0, 1, t_init_k)
            .with_pump(pump_dp_max, pump_w_max),
    );

    // Rama 1: Conexión Nodo 1 -> Nodo 2 con Válvula de Control
    net.add_pipe(
        Pipe::new(id_node1, id_node2, diametro_m, 0.5, rugosidad_m, 0.0, 1, t_init_k)
            .with_valve(valve_cv, valve_opening),
    );

    // Rama 2: Conexión Nodo 2 -> Nodo 3, Tubería de 26m calentada por la pared
    net.add_pipe(
        Pipe::new(id_node2, id_out, diametro_m, 26.0, rugosidad_m, 0.0, n_cells, t_init_k)
            .with_heat(q_source_w)
            .with_wall(wall_mass_kg, wall_cp_j_kg, wall_ua_w_k),
    );

    // ─────────────────────────────────────────────────────────────────────
    // SOLVEDOR Y SIMULACIÓN
    // ─────────────────────────────────────────────────────────────────────

    let mut solver = Solver::new();

    // Pipe IDs asignados en orden de inserción:
    //   pipe 0 = Bomba (nodo_in → nodo1)
    //   pipe 1 = Válvula (nodo1 → nodo2)
    //   pipe 2 = Tubería calentada (nodo2 → nodo_out)
    let pipe_bomba  = 0_usize;
    let pipe_valvula = 1_usize;
    let pipe_cano    = 2_usize;

    // ── Definir los selectores de variables a registrar ──────────────────
    //
    // Presiones nodales, caudales y temperatura de cada celda del caño
    // (fluido + pared). Los selectores se construyen programáticamente para
    // que el número de celdas sea arbitrario (n_cells).
    let mut selectors: Vec<RecordSelector> = vec![
        // Nodos de presión (bar → Pa, se convierte al leer el CSV)
        RecordSelector::NodePressure(id_in),
        RecordSelector::NodePressure(id_node1),
        RecordSelector::NodePressure(id_node2),
        RecordSelector::NodePressure(id_out),
        // Temperaturas nodales
        RecordSelector::NodeTemperature(id_in),
        RecordSelector::NodeTemperature(id_out),
        // Caudales de cada rama
        RecordSelector::PipeFlow(pipe_bomba),
        RecordSelector::PipeFlow(pipe_valvula),
        RecordSelector::PipeFlow(pipe_cano),
    ];
    // Temperatura del fluido en cada celda de la tubería calentada
    for i in 0..n_cells {
        selectors.push(RecordSelector::PipeCellTemperature(pipe_cano, i));
    }
    // Temperatura de la pared en cada celda
    for i in 0..n_cells {
        selectors.push(RecordSelector::PipeWallTemperature(pipe_cano, i));
    }

    let mut recorder = CsvRecorder::new("results/pump_loop.csv", selectors)
        .expect("No se pudo crear results/pump_loop.csv");

    println!(
        "{:>6} {:>8} {:>10} {:>10} {:>9} {:>9} {:>9}",
        "t[s]", "W[kg/s]", "P_node1[b]", "P_node2[b]", "T_inlet°C", "T_outlet°C", "T_wall_mid"
    );
    println!("{}", "─".repeat(78));

    let t_start = std::time::Instant::now();

    for step in 0..n_steps {
        solver.step(&mut net, dt);
        let t = solver.time;

        // Registrar fila completa en el CSV
        recorder.record(t, &net).expect("Error escribiendo CSV");

        // Imprimir resumen en consola cada `print_every` pasos
        if step % print_every == 0 || step == n_steps - 1 {
            let w_flow   = net.pipes[pipe_cano].flow;
            let p1       = net.nodes[id_node1].pressure / 1.0e5;
            let p2       = net.nodes[id_node2].pressure / 1.0e5;
            let t_inlet  = net.nodes[id_node2].temperature - 273.15;
            let t_outlet = net.nodes[id_out].temperature - 273.15;
            let t_wall_mid = net.pipes[pipe_cano].wall_temp[n_cells / 2] - 273.15;
            println!(
                "{:>6.0} {:>8.4} {:>10.3} {:>10.3} {:>9.2} {:>9.2} {:>9.2}",
                t, w_flow, p1, p2, t_inlet, t_outlet, t_wall_mid
            );
        }
    }

    recorder.flush().expect("Error flusheando CSV");
    let elapsed = t_start.elapsed();

    // Resumen final de resultados
    let w_flow = net.pipes[2].flow;
    let dp_pump = (net.nodes[id_node1].pressure - net.nodes[id_in].pressure) / 1.0e5;
    let dp_valve = (net.nodes[id_node1].pressure - net.nodes[id_node2].pressure) / 1.0e5;
    let dp_pipe = (net.nodes[id_node2].pressure - net.nodes[id_out].pressure) / 1.0e5;
    let t_in = net.nodes[id_in].temperature - 273.15;
    let t_out = net.nodes[id_out].temperature - 273.15;
    let dt_fluid = t_out - t_in;
    let t_wall_mean = net.pipes[2].wall_temp.iter().copied().sum::<f64>() / n_cells as f64 - 273.15;
    let t_fluid_mean = net.pipes[2].cell_temp.iter().copied().sum::<f64>() / n_cells as f64 - 273.15;

    println!("{}", "─".repeat(78));
    println!();
    println!("Simulación completada en {:.2?}", elapsed);
    println!();
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│   ESTADO ESTACIONARIO (t = {:.0} s = {:.1} min)             │",
        solver.time, solver.time / 60.0);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ Caudal:           {:>8.4} kg/s  ({:.2} L/min)           │",
        w_flow, w_flow * 60.0);
    println!("│ ΔP Bomba:         {:>8.4} bar   ({:.2} m col. agua)     │",
        dp_pump, dp_pump * 10.197);
    println!("│ ΔP Válvula Cv:    {:>8.4} bar                               │",
        dp_valve);
    println!("│ ΔP Tubería 26m:   {:>8.4} bar                               │",
        dp_pipe);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ T entrada fluido: {:>8.2} °C                                │", t_in);
    println!("│ T salida fluido:  {:>8.2} °C                                │", t_out);
    println!("│ ΔT del fluido:    {:>8.2} °C                                │", dt_fluid);
    println!("│ T media de pared: {:>8.2} °C                                │", t_wall_mean);
    println!("│ T media de fluido:{:>8.2} °C                                │", t_fluid_mean);
    println!("│ ΔT pared-fluido:  {:>8.2} °C                                │", t_wall_mean - t_fluid_mean);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ Q inyectado:      {:>8.1} kW                                │", q_source_w / 1000.0);
    println!("│ CPU time:         {:>8.2?}                                  │", elapsed);
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Resultados: results/pump_loop.csv");
}
