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
    network::{Network, Node, Pipe},
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
    let mut level_a  = 3.0_f64;        // [m] nivel inicial tanque A
    let mut level_b  = 0.1_f64;        // [m] nivel inicial tanque B
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
    let p_a_init     = p_atm + rho_init * g * level_a;
    let p_b_init     = p_atm + rho_init * g * level_b;

    println!("┌─────────────────────────────────────────────────┐");
    println!("│   THNet — Dos Tanques Abiertos Interconectados   │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Área Tanques:    {:>8.4} m²                    │", area_tank);
    println!("│ Altura Tanques:  {:>8.1} m                     │", height_tank);
    println!("│ Nivel Inicial A: {:>8.2} m                     │", level_a);
    println!("│ Nivel Inicial B: {:>8.2} m                     │", level_b);
    println!("│ Cañería:         L = {:.1} m, D = {:.1} mm       │", pipe_length, pipe_diam * 1000.0);
    println!("│ Paso de tiempo:  {:>8.2} s                     │", dt);
    println!("└─────────────────────────────────────────────────┘");
    println!();

    // ─────────────────────────────────────────────────────────────────────
    // CONSTRUCCIÓN DE LA RED
    // ─────────────────────────────────────────────────────────────────────

    let mut net = Network::new();

    // Nodo 0: Base del Tanque A (Presión fija variable)
    let id_a = net.add_node(
        Node::new(t_init_k, p_a_init, 0.001).with_fixed_pressure(),
    );

    // Nodo 1: Base del Tanque B (Presión fija variable)
    let id_b = net.add_node(
        Node::new(t_init_k, p_b_init, 0.001).with_fixed_pressure(),
    );

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
        // 1. Actualizar las presiones de contorno en función del nivel hidrostático
        let rho_a = net.nodes[id_a].density();
        let rho_b = net.nodes[id_b].density();

        let p_a = p_atm + rho_a * g * level_a;
        let p_b = p_atm + rho_b * g * level_b;

        net.nodes[id_a].pressure = p_a;
        net.nodes[id_a].fixed_pressure = Some(p_a);

        net.nodes[id_b].pressure = p_b;
        net.nodes[id_b].fixed_pressure = Some(p_b);

        // 2. Dar paso de simulación
        solver.step(&mut net, dt);
        let t = solver.time;

        let flow = net.pipes[0].flow;

        // 3. Escribir en CSV
        writeln!(
            w_csv,
            "{:.2},{:.6},{:.6},{:.6},{:.4},{:.4}",
            t, level_a, level_b, flow, p_a / 1.0e5, p_b / 1.0e5
        )
        .unwrap();

        // 4. Imprimir en pantalla periódicamente
        if step % print_every == 0 || step == n_steps - 1 {
            println!(
                "{:>6.1} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>10.4}",
                t, level_a, level_b, flow, p_a / 1.0e5, p_b / 1.0e5
            );
        }

        // 5. Integrar niveles de los tanques para el siguiente paso
        level_a -= dt * flow / (rho_a * area_tank);
        level_b += dt * flow / (rho_b * area_tank);

        // Clampear niveles a límites físicos
        level_a = level_a.clamp(0.0, height_tank);
        level_b = level_b.clamp(0.0, height_tank);
    }

    w_csv.flush().unwrap();
    let elapsed = t_start.elapsed();

    println!("{}", "─".repeat(60));
    println!();
    println!("Simulación completada en {:.2?}", elapsed);
    println!("Estado final:");
    println!("  Nivel A: {:.4} m", level_a);
    println!("  Nivel B: {:.4} m", level_b);
    println!("  Diferencia: {:.4} m", (level_a - level_b).abs());
    println!("  Caudal final: {:.4} kg/s", net.pipes[0].flow);
    println!();
    println!("Resultados guardados en results/dos_tanques.csv");
}
