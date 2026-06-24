/// Ejemplo THNet: Dos Tanques Abiertos Interconectados por una Cañería
///
/// ## Sistema simulado
///
/// ```text
///      [Tanque A: L_init = 3.0 m] ═════════(Caño 5m, D=25cm)═════════> [Tanque B: L_init = 0.1 m]
///      Area = 0.2825 m², Altura = 4.0 m                               Area = 0.2825 m², Altura = 4.0 m
/// ```
///
/// El nivel de agua en cada tanque varía dinámicamente según el caudal neto.
/// Las presiones hidrostáticas en la base de los tanques actúan como condiciones de contorno variables.

use bloques::thnet::{
    network::{Network, Node, Pipe, OpenTank},
    solver::Solver,
};
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() {
    // ─────────────────────────────────────────────────────────────────────
    // PARÁMETROS DEL SISTEMA
    // ─────────────────────────────────────────────────────────────────────

    let area_tank    = 0.2825_f64;     // [m²] área transversal de ambos tanques
    let height_tank  = 4.0_f64;        // [m] altura total de los tanques
    let pipe_length  = 5.0_f64;        // [m] longitud de la tubería
    let pipe_diam    = 0.01_f64;       // [m] 1 cm diámetro interior
    let roughness    = 1.5e-5_f64;     // [m] acero inoxidable 304L

    // Condiciones iniciales
    let level_a_init = 3.0_f64;        // [m] nivel inicial tanque A
    let level_b_init = 0.1_f64;        // [m] nivel inicial tanque B
    let t_init_k     = 20.0 + 273.15;  // [K] 20 °C temperatura constante
    let p_atm        = 1.0e5_f64;      // [Pa] 1 bar presión de gas (abierto)
    let g            = 9.80665_f64;    // [m/s²] gravedad

    // Parámetros de simulación
    let dt           = 0.2_f64;        // [s] paso de tiempo
    let t_final      = 8000.0_f64;     // [s] 8000 s para permitir estabilización completa
    let n_steps      = (t_final / dt) as usize;
    let print_every  = (500.0 / dt) as usize; // imprimir cada 500 s

    // Densidad teórica del agua a 20 °C para la hidrostática inicial
    let rho_init     = 998.2_f64;      // [kg/m³]
    let p_a_init     = p_atm + rho_init * g * level_a_init;
    let p_b_init     = p_atm + rho_init * g * level_b_init;

    println!("┌─────────────────────────────────────────────────┐");
    println!("│   THNet — Dos Tanques Abiertos Interconectados   │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Área Tanques:    {:>8.4} m²                    │", area_tank);
    println!("│ Altura Tanques:  {:>8.1} m                     │", height_tank);
    println!("│ Nivel Inicial A: {:>8.2} m                     │", level_a_init);
    println!("│ Nivel Inicial B: {:>8.2} m                     │", level_b_init);
    println!("│ Cañería:         L = {:.1} m, D = {:.1} mm       │", pipe_length, pipe_diam * 1000.0);
    println!("│ Paso de tiempo:  {:>8.2} s                     │", dt);
    println!("└─────────────────────────────────────────────────┘");
    println!();

    // ─────────────────────────────────────────────────────────────────────
    // CONSTRUCCIÓN DE LA RED
    // ─────────────────────────────────────────────────────────────────────

    let mut net = Network::new();

    // Nodo 0: Base del Tanque A
    let id_a = net.add_node(
        Node::new(t_init_k, p_a_init, 0.001),
    );

    // Nodo 1: Base del Tanque B
    let id_b = net.add_node(
        Node::new(t_init_k, p_b_init, 0.001),
    );

    // Tanque A
    let tank_a_id = net.add_open_tank(OpenTank::new(
        id_a,
        area_tank,
        level_a_init,
        0.0,
        height_tank,
        0.0,
        p_atm,
    ));

    // Tanque B
    let tank_b_id = net.add_open_tank(OpenTank::new(
        id_b,
        area_tank,
        level_b_init,
        0.0,
        height_tank,
        0.0,
        p_atm,
    ));

    // Cañería interconectora (Rama 0)
    net.add_pipe(
        Pipe::new(id_a, id_b, pipe_diam, pipe_length, roughness, 0.0, 10, t_init_k),
    );

    // ─────────────────────────────────────────────────────────────────────
    // SOLVEDOR Y SIMULACIÓN
    // ─────────────────────────────────────────────────────────────────────

    let mut solver = Solver::new();

    std::fs::create_dir_all("results").expect("No se pudo crear results/");
    let file = File::create("results/dos_tanques.csv").unwrap();
    let mut w_csv = BufWriter::new(file);

    writeln!(w_csv, "t_s,level_a_m,level_b_m,flow_kg_s,p_a_bar,p_b_bar").unwrap();

    println!(
        "{:>6} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "t[s]", "Nivel A[m]", "Nivel B[m]", "W[kg/s]", "P_A[bar]", "P_B[bar]"
    );
    println!("{}", "─".repeat(60));

    let t_start = std::time::Instant::now();

    for step in 0..n_steps {
        // 1. Dar paso de simulación (la presión y niveles se actualizan internamente)
        solver.step(&mut net, dt);
        let t = solver.time;

        let flow = net.pipes[0].flow;
        let level_a = net.open_tanks[tank_a_id].level;
        let level_b = net.open_tanks[tank_b_id].level;
        let p_a = net.nodes[id_a].pressure;
        let p_b = net.nodes[id_b].pressure;

        // 2. Escribir en CSV
        writeln!(
            w_csv,
            "{:.2},{:.6},{:.6},{:.6},{:.4},{:.4}",
            t, level_a, level_b, flow, p_a / 1.0e5, p_b / 1.0e5
        )
        .unwrap();

        // 3. Imprimir en pantalla periódicamente
        if step % print_every == 0 || step == n_steps - 1 {
            println!(
                "{:>6.1} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>10.4}",
                t, level_a, level_b, flow, p_a / 1.0e5, p_b / 1.0e5
            );
        }
    }

    w_csv.flush().unwrap();
    let elapsed = t_start.elapsed();

    let level_a_fin = net.open_tanks[tank_a_id].level;
    let level_b_fin = net.open_tanks[tank_b_id].level;

    println!("{}", "─".repeat(60));
    println!();
    println!("Simulación completada en {:.2?}", elapsed);
    println!("Estado final:");
    println!("  Nivel A: {:.4} m", level_a_fin);
    println!("  Nivel B: {:.4} m", level_b_fin);
    println!("  Diferencia: {:.4} m", (level_a_fin - level_b_fin).abs());
    println!("  Caudal final: {:.4} kg/s", net.pipes[0].flow);
    println!();
    println!("Resultados guardados en results/dos_tanques.csv");
}
