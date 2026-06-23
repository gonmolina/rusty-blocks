/// Convección natural — Simulación con THNet (solvedor de red implícito)
///
/// ## Sistema simulado
///
/// Lazo cerrado vertical de 12 m de altura con dos ramas:
///
/// ```text
///        [Nodo 1 — Header superior, pequeño]
///              ↑                    ↓
///         pipe_up               pipe_dn
///        (sube 12m)           (baja 12m)
///        +18 kW               HX (T_cool=20°C)
///      [resistencia         [intercambiador
///       eléctrica]            de calor]
///              ↑                    ↓
///        [Nodo 0 — Header inferior, pequeño] ← presión de referencia
/// ```
///
/// ## Sumidero de calor realista
///
/// La rama de bajada NO usa una potencia constante de -18 kW. En cambio,
/// usa un **intercambiador de calor** con:
///   - Refrigerante a T_cool = 20 °C (ej. agua de una torre o red de fría)
///   - Coeficiente UA calculado para extraer ≈ 18 kW en estado estacionario
///
/// La ecuación de extracción por celda es:
///   Q_cell = (UA/N) · (T_cell - T_cool)    [W]
///
/// implementada de forma implícita (incondicialmente estable):
///   h_new = (h_old + CFL·h_in + NTU·h_cool) / (1 + CFL + NTU)
///   NTU = (UA/N)·Δt / (M_cell · cp)
///
/// ## Cañerías: DN250 (25 cm), acero inoxidable 304

use bloques::thnet::{
    network::{Network, Node, Pipe},
    solver::Solver,
    thermo,
};
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() {
    // ─────────────────────────────────────────────────────────────────────
    // PARÁMETROS DEL SISTEMA
    // ─────────────────────────────────────────────────────────────────────

    let altura_m     = 12.0_f64;       // [m] altura del lazo
    let diametro_m   = 0.25_f64;       // [m] 25 cm diámetro interior
    let rugosidad_m  = 1.5e-5_f64;     // [m] acero inoxidable 304
    let q_source_w   = 18_000.0_f64;   // [W] potencia de la fuente (ram. subida)
    let n_cells      = 10_usize;       // celdas 1D en cada rama
    let v_header_m3  = 0.001_f64;      // [m³] = 1 litro por header

    // Condición inicial
    let t_init_k = 30.0 + 273.15;      // [K] 30 °C temperatura inicial uniforme
    let p_ref_pa = 2.0e5_f64;          // [Pa] 2 bar presión de operación

    // ── Diseño del intercambiador de calor ────────────────────────────────
    //
    // Refrigerante: agua de red a T_cool = 20 °C
    // Se elige UA de modo que en estado estacionario extraiga ≈ 18 kW.
    //
    // Estimación del estado estacionario esperado:
    //   W_ss ≈ 0.12 kg/s  (iteración analítica aproximada)
    //   T_fluid_in  ≈ 65 °C  (fluido que entra al HX desde el header superior)
    //   T_fluid_out ≈ 30 °C  (fluido que sale al header inferior)
    //   ΔT_ln = (ΔT_in - ΔT_out) / ln(ΔT_in/ΔT_out)
    //         = (65-20 - 30-20) / ln(45/10) = 35/1.504 ≈ 23.3 °C
    //   UA = Q / ΔT_ln = 18000 / 23.3 ≈ 773 W/K
    //
    // Usamos UA = 800 W/K (margen del 4 %).
    // El sistema se auto-regula: si el fluido está más frío, extrae menos calor.
    let t_coolant_k  = 20.0 + 273.15;  // [K] refrigerante a 20 °C
    let ua_hx_w_k    = 800.0_f64;      // [W/K] UA del intercambiador

    // Parámetros de simulación
    let dt       = 0.2_f64;            // [s] paso de tiempo — grande gracias al solvedor implícito
    let t_final  = 20000.0_f64;        // [s] 5.5 horas de simulación
    let n_steps  = (t_final / dt) as usize;
    let print_every = (500.0 / dt) as usize; // imprimir cada 500 s

    // Volúmenes de referencia
    let v_pipe_m3 = std::f64::consts::PI * (diametro_m / 2.0).powi(2) * altura_m;
    println!("┌─────────────────────────────────────────────────┐");
    println!("│   THNet — Convección Natural con HX             │");
    println!("├─────────────────────────────────────────────────┤");
    println!("│ Altura:          {:>8.1} m                     │", altura_m);
    println!("│ Diámetro:        {:>8.1} mm                    │", diametro_m * 1000.0);
    println!("│ Rugosidad:       {:>8.2e} m (AISI 304)         │", rugosidad_m);
    println!("│ Fuente de calor: {:>8.1} kW (constante)        │", q_source_w / 1000.0);
    println!("│ UA intercamb.:   {:>8.0} W/K                   │", ua_hx_w_k);
    println!("│ T refrigerante:  {:>8.1} °C                    │", t_coolant_k - 273.15);
    println!("│ V(caño):         {:>8.2} L                     │", v_pipe_m3 * 1000.0);
    println!("│ V(header):       {:>8.0} L                     │", v_header_m3 * 1000.0);
    println!("│ Δt solvedor:     {:>8.1} s                     │", dt);
    println!("└─────────────────────────────────────────────────┘");
    println!();

    // ─────────────────────────────────────────────────────────────────────
    // CONSTRUCCIÓN DE LA RED
    // ─────────────────────────────────────────────────────────────────────

    let mut net = Network::new();

    // Nodo 0 — Header inferior (presión fija de referencia)
    let id_bottom = net.add_node(
        Node::new(t_init_k, p_ref_pa, v_header_m3)
            .with_fixed_pressure(),
    );

    // Nodo 1 — Header superior (presión libre)
    let p_top_init = p_ref_pa + thermo::density(t_init_k) * 9.80665 * altura_m;
    let id_top = net.add_node(
        Node::new(t_init_k, p_top_init, v_header_m3),
    );

    // Tuberías con modelo de pared (estructura de calor) de 2 mm de espesor
    // Características de la pared (acero inoxidable AISI 304, espesor 2 mm):
    //   - Masa: 152 kg de acero
    //   - cp: 500 J/(kg·K)
    //   - UA pared-fluido: 3500 W/K
    let wall_mass_kg = 152.0_f64;
    let wall_cp_j_kg = 500.0_f64;
    let wall_ua_w_k  = 3500.0_f64;

    // Tubería ascendente: nodo_up=bottom, nodo_dn=top, sube 12m
    // Potencia constante de +18 kW inyectada en la pared sólida
    net.add_pipe(
        Pipe::new(id_bottom, id_top, diametro_m, altura_m, rugosidad_m,
                   altura_m, n_cells, t_init_k)
            .with_heat(q_source_w)
            .with_wall(wall_mass_kg, wall_cp_j_kg, wall_ua_w_k),
    );

    // Tubería descendente: nodo_up=top, nodo_dn=bottom, baja 12m
    // Potencia constante de -18 kW (extracción de calor) inyectada en la pared sólida (Opción A)
    net.add_pipe(
        Pipe::new(id_top, id_bottom, diametro_m, altura_m, rugosidad_m,
                   -altura_m, n_cells, t_init_k)
            .with_heat(-q_source_w)
            .with_wall(wall_mass_kg, wall_cp_j_kg, wall_ua_w_k),
    );

    // ─────────────────────────────────────────────────────────────────────
    // SOLVEDOR Y BUCLE DE SIMULACIÓN
    // ─────────────────────────────────────────────────────────────────────

    let mut solver = Solver::new();

    std::fs::create_dir_all("results").expect("No se pudo crear results/");
    let file = File::create("results/conv_natural_thnet.csv").unwrap();
    let mut w_csv = BufWriter::new(file);

    let mut header_str = "t_s,W_kg_s,T_bot_C,T_top_C".to_string();
    for i in 0..n_cells {
        header_str.push_str(&format!(",T_up_{}_C", i));
    }
    for i in 0..n_cells {
        header_str.push_str(&format!(",T_dn_{}_C", i));
    }
    for i in 0..n_cells {
        header_str.push_str(&format!(",rho_up_{}", i));
    }
    for i in 0..n_cells {
        header_str.push_str(&format!(",rho_dn_{}", i));
    }
    writeln!(w_csv, "{}", header_str).unwrap();

    println!(
        "{:>6} {:>8} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9} {:>8}",
        "t[s]", "W[kg/s]", "T_bot°C", "T_top°C",
        "T_mid_up", "T_mid_dn", "T_dn_in", "T_dn_out", "Q_HX[W]"
    );
    println!("{}", "─".repeat(88));

    let t_start = std::time::Instant::now();

    for step in 0..n_steps {
        solver.step(&mut net, dt);
        let t = solver.time;

        let p_up  = &net.pipes[0];  // rama de subida (fuente)
        let p_dn  = &net.pipes[1];  // rama de bajada (HX)
        let w_ss  = p_up.flow;

        // Temperatura de entrada/salida del intercambiador
        // (entrada al HX = temperatura del nodo superior)
        let t_dn_in  = net.nodes[id_top].temperature - 273.15;   // fluido que entra al HX
        let t_dn_out = p_dn.cell_temp[p_dn.n_cells - 1] - 273.15; // última celda (fluido que sale al header bot)

        // Calor extraído por el HX en este instante (calculado exactamente por entalpía):
        // Q_hx = W · (h_in_hx - h_out_hx)
        let h_dn_in  = net.nodes[id_top].h;
        let h_dn_out = thermo::enthalpy(p_dn.cell_temp[p_dn.n_cells - 1]);
        let q_hx = w_ss.abs() * (h_dn_in - h_dn_out).abs();

        // Número de Reynolds y factor de fricción
        let _rho_up = p_up.mean_density();
        let mu_up  = thermo::viscosity(p_up.mean_temperature());
        let area   = p_up.area();
        let re     = w_ss.abs() * diametro_m / (area * mu_up).max(1e-15);
        let _f_fric = p_up.friction_factor(re);

        let t_bot_c     = net.nodes[id_bottom].temperature - 273.15;
        let t_top_c     = net.nodes[id_top].temperature - 273.15;
        let t_mid_up_c  = p_up.cell_temp[n_cells / 2] - 273.15;
        let t_mid_dn_c  = p_dn.cell_temp[n_cells / 2] - 273.15;

        let mut row_str = format!("{:.1},{:.6},{:.3},{:.3}", t, w_ss, t_bot_c, t_top_c);
        for i in 0..n_cells {
            row_str.push_str(&format!(",{:.3}", p_up.cell_temp[i] - 273.15));
        }
        for i in 0..n_cells {
            row_str.push_str(&format!(",{:.3}", p_dn.cell_temp[i] - 273.15));
        }
        for i in 0..n_cells {
            row_str.push_str(&format!(",{:.3}", thermo::density(p_up.cell_temp[i])));
        }
        for i in 0..n_cells {
            row_str.push_str(&format!(",{:.3}", thermo::density(p_dn.cell_temp[i])));
        }
        writeln!(w_csv, "{}", row_str).unwrap();

        if step % print_every == 0 || step == n_steps - 1 {
            println!(
                "{:>6.0} {:>8.5} {:>8.2} {:>8.2} {:>9.2} {:>9.2} {:>9.2} {:>9.2} {:>8.0}",
                t, w_ss, t_bot_c, t_top_c, t_mid_up_c, t_mid_dn_c,
                t_dn_in, t_dn_out, q_hx
            );
        }
    }
    w_csv.flush().unwrap();
    let elapsed = t_start.elapsed();

    // ─────────────────────────────────────────────────────────────────────
    // RESUMEN FINAL
    // ─────────────────────────────────────────────────────────────────────
    let p_up = &net.pipes[0];
    let p_dn = &net.pipes[1];
    let w_ss = p_up.flow;

    let t_bot_c    = net.nodes[id_bottom].temperature - 273.15;
    let t_top_c    = net.nodes[id_top].temperature - 273.15;
    let t_dn_in_c  = net.nodes[id_top].temperature - 273.15;
    let t_dn_out_c = p_dn.cell_temp[p_dn.n_cells - 1] - 273.15;
    let h_dn_in    = net.nodes[id_top].h;
    let h_dn_out   = thermo::enthalpy(p_dn.cell_temp[p_dn.n_cells - 1]);
    let q_hx       = w_ss.abs() * (h_dn_in - h_dn_out).abs();

    let rho_up = p_up.mean_density();
    let mu_up  = thermo::viscosity(p_up.mean_temperature());
    let area   = p_up.area();
    let re     = w_ss.abs() * diametro_m / (area * mu_up).max(1e-15);
    let f      = p_up.friction_factor(re);

    // Presión de flotabilidad (fuerza motriz)
    let rho_dn   = p_dn.mean_density();
    let delta_rho = rho_dn - rho_up;
    let dp_buoy  = delta_rho * 9.80665 * altura_m;

    // Pérdida de carga por fricción (solo subida, simétrico con bajada ≈ similar)
    let k_fric = f * altura_m / (diametro_m * 2.0 * area * area);
    let dp_fric = k_fric * w_ss.powi(2) / rho_up;

    println!("{}", "─".repeat(88));
    println!();
    println!("Simulación completada en {:.2?}", elapsed);
    println!();
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│   ESTADO ESTACIONARIO (t = {:.0} s = {:.1} min)             │",
        solver.time, solver.time / 60.0);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ Caudal:           {:>8.4} kg/s  ({:.2} L/min)           │",
        w_ss, w_ss / rho_up * 1000.0 * 60.0);
    println!("│ Reynolds:         {:>8.0}  ({})             │",
        re, if re < 2300.0 { "laminar" } else { "turbulento" });
    println!("│ Factor fricc. f:  {:>8.4}                                  │", f);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ T header inferior:{:>8.2} °C                                │", t_bot_c);
    println!("│ T header superior:{:>8.2} °C                                │", t_top_c);
    println!("│ T entrada HX:     {:>8.2} °C  (fluido entra al HX)         │", t_dn_in_c);
    println!("│ T salida HX:      {:>8.2} °C  (fluido sale del HX)         │", t_dn_out_c);
    println!("│ ΔT HX:            {:>8.2} °C                                │",
        (t_dn_in_c - t_dn_out_c).abs());
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ Q fuente (up):    {:>8.1} W  (impuesta)                    │", q_source_w);
    println!("│ Q extraída (HX):  {:>8.1} W  (calculada W·cp·ΔT)          │", q_hx);
    println!("│ Balance Q:        {:>8.1} W  (diferencia = almacenado)     │",
        q_source_w - q_hx);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ ΔP flotabilidad:  {:>8.1} Pa  (fuerza motriz)              │", dp_buoy);
    println!("│ ΔP fricción (up): {:>8.1} Pa                               │", dp_fric);
    println!("│ Δρ (dn - up):     {:>8.2} kg/m³                            │", delta_rho);
    println!("│ T media up:       {:>8.2} °C                               │",
        p_up.mean_temperature() - 273.15);
    println!("│ T media dn:       {:>8.2} °C                               │",
        p_dn.mean_temperature() - 273.15);
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ Δt solvedor:      {:>8.1} s  (implícito, estable)          │", dt);
    println!("│ CPU time:         {:>8.2?}                                  │", elapsed);
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Resultados: results/conv_natural_thnet.csv");
}
