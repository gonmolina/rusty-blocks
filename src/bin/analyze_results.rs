use std::fs::File;
use std::io::{BufRead, BufReader};

struct Record {
    t: f64,
    m_bot: f64,
    m_top: f64,
    t_bot: f64,
    t_top: f64,
    p_bot: f64,
    p_top: f64,
    w_up_in: f64,
    w_up_out: f64,
    w_down_in: f64,
    w_down_out: f64,
    rho_up_avg: f64,
    rho_down_avg: f64,
}

fn main() {
    let file = File::open("natural_convection_results.csv").expect("No se pudo abrir el archivo CSV");
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (idx, line) in reader.lines().enumerate() {
        if idx == 0 {
            continue; // Skip header
        }
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 13 {
            continue;
        }
        records.push(Record {
            t: parts[0].parse().unwrap(),
            m_bot: parts[1].parse().unwrap(),
            m_top: parts[2].parse().unwrap(),
            t_bot: parts[3].parse().unwrap(),
            t_top: parts[4].parse().unwrap(),
            p_bot: parts[5].parse().unwrap(),
            p_top: parts[6].parse().unwrap(),
            w_up_in: parts[7].parse().unwrap(),
            w_up_out: parts[8].parse().unwrap(),
            w_down_in: parts[9].parse().unwrap(),
            w_down_out: parts[10].parse().unwrap(),
            rho_up_avg: parts[11].parse().unwrap(),
            rho_down_avg: parts[12].parse().unwrap(),
        });
    }

    println!("Total registros leídos: {}", records.len());

    // Analizar la última parte de la simulación (t >= 4000 s)
    let last_records: Vec<&Record> = records.iter().filter(|r| r.t >= 4000.0).collect();
    if last_records.is_empty() {
        println!("No hay suficientes datos en el rango t >= 4000 s");
        return;
    }

    let n = last_records.len() as f64;
    
    // Medias y rangos
    let mut w_up_in_sum = 0.0;
    let (mut w_up_in_min, mut w_up_in_max) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut w_up_out_sum = 0.0;
    let mut w_down_in_sum = 0.0;
    let mut w_down_out_sum = 0.0;
    
    let mut t_bot_sum = 0.0;
    let mut t_top_sum = 0.0;
    let mut m_bot_sum = 0.0;
    let mut m_top_sum = 0.0;
    
    let mut dp_sum = 0.0;
    let mut dp_hydro_sum = 0.0;
    let mut rho_up_sum = 0.0;
    let mut rho_down_sum = 0.0;

    for r in &last_records {
        w_up_in_sum += r.w_up_in;
        w_up_in_min = w_up_in_min.min(r.w_up_in);
        w_up_in_max = w_up_in_max.max(r.w_up_in);
        w_up_out_sum += r.w_up_out;
        w_down_in_sum += r.w_down_in;
        w_down_out_sum += r.w_down_out;
        
        t_bot_sum += r.t_bot;
        t_top_sum += r.t_top;
        m_bot_sum += r.m_bot;
        m_top_sum += r.m_top;
        
        dp_sum += r.p_bot - r.p_top;
        rho_up_sum += r.rho_up_avg;
        rho_down_sum += r.rho_down_avg;
        
        // Buoyancy head: (rho_down - rho_up) * g * H
        // g = 9.80665, H = 5.0
        let dp_hydro = (r.rho_down_avg - r.rho_up_avg) * 9.80665 * 5.0;
        dp_hydro_sum += dp_hydro;
    }

    let w_up_in_mean = w_up_in_sum / n;
    let w_up_out_mean = w_up_out_sum / n;
    let w_down_in_mean = w_down_in_sum / n;
    let w_down_out_mean = w_down_out_sum / n;
    let t_bot_mean = t_bot_sum / n;
    let t_top_mean = t_top_sum / n;
    let m_bot_mean = m_bot_sum / n;
    let m_top_mean = m_top_sum / n;
    let dp_mean = dp_sum / n;
    let dp_hydro_mean = dp_hydro_sum / n;
    let rho_up_mean = rho_up_sum / n;
    let rho_down_mean = rho_down_sum / n;

    println!("\n=== ANÁLISIS DE ESTADO ESTACIONARIO (t >= 4000s) ===");
    println!("Flujo másico entrada Pipe Up: mean = {:.4} kg/s (min = {:.4}, max = {:.4}, rango = {:.4})", 
             w_up_in_mean, w_up_in_min, w_up_in_max, w_up_in_max - w_up_in_min);
    println!("Flujo másico salida Pipe Up:  mean = {:.4} kg/s", w_up_out_mean);
    println!("Flujo másico entrada Pipe Down: mean = {:.4} kg/s", w_down_in_mean);
    println!("Flujo másico salida Pipe Down:  mean = {:.4} kg/s", w_down_out_mean);
    println!("Temperatura Header Bottom:    mean = {:.2} °C", t_bot_mean);
    println!("Temperatura Header Top:       mean = {:.2} °C", t_top_mean);
    println!("Masa Header Bottom:           mean = {:.1} kg", m_bot_mean);
    println!("Masa Header Top:              mean = {:.1} kg", m_top_mean);
    println!("Presión Bottom - Top (DP):    mean = {:.1} Pa", dp_mean);
    println!("Densidad promedio Pipe Up:    mean = {:.2} kg/m³", rho_up_mean);
    println!("Densidad promedio Pipe Down:  mean = {:.2} kg/m³", rho_down_mean);
    println!("Salto de Presión Boyante calculada (drho * g * H): mean = {:.1} Pa", dp_hydro_mean);

    println!("\n=== VERIFICACIONES FÍSICAS ===");
    
    // 1. Conservación de Masa
    let mass_balance_loop = (w_up_in_mean - w_down_in_mean).abs() / w_up_in_mean.max(1e-6) * 100.0;
    println!("1. Error de balance de masa en el lazo (W_up - W_down): {:.2}%", mass_balance_loop);
    let pipe_up_mass_diff = (w_up_in_mean - w_up_out_mean).abs() / w_up_in_mean.max(1e-6) * 100.0;
    println!("   Error de balance de masa en Pipe Up (inlet vs outlet): {:.2}%", pipe_up_mass_diff);

    // 2. Conservación de Energía
    // Q_heat = W * Cp * (T_out - T_in)
    // Cp = 4184.0 J/kg/K. Q_heat = 10000.0 W.
    let cp = 4184.0;
    let q_heat = 10000.0;
    let expected_dt = q_heat / (w_up_in_mean.abs() * cp);
    let actual_dt = t_top_mean - t_bot_mean;
    let energy_error = (actual_dt - expected_dt).abs() / expected_dt * 100.0;
    println!("2. Salto de temperatura teórico en Pipe Up (Q / (W * Cp)): {:.2} °C", expected_dt);
    println!("   Salto de temperatura real (T_top - T_bot):              {:.2} °C", actual_dt);
    println!("   Error en balance de energía:                            {:.2}%", energy_error);

    // 3. Balance de Presión y Cantidad de Movimiento
    let dp_hydro_up = rho_up_mean * 9.80665 * 5.0;
    let dp_hydro_down = rho_down_mean * 9.80665 * 5.0;
    println!("3. Rango de presiones hidrostáticas esperadas:");
    println!("   Columna caliente (Pipe Up):  {:.1} Pa", dp_hydro_up);
    println!("   Columna fría (Pipe Down):    {:.1} Pa", dp_hydro_down);
    println!("   Diferencia de presión real:  {:.1} Pa", dp_mean);
    
    // 4. Masa total del sistema
    let total_mass = m_bot_mean + m_top_mean;
    println!("4. Masa total en headers: {:.2} kg (Inicial = 2000 kg)", total_mass);
}
