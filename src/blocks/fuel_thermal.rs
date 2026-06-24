use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::blocks::{Block, BlockRegistry};

/// Parámetros de configuración del modelo térmico de combustible con perfil APD (Simulink RA-10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuelThermalParams {
    /// Potencia térmica total nominal [W] (p. ej. 28.5e6 o 30.0e6)
    pub nominal_power_w: f64,
    /// Fracciones de distribución axial de potencia (APD / FPD) para las 3 zonas
    pub apd_fractions: Vec<f64>,
    /// Masa total de la mezcla combustible + vaina [kg] (default: 115.5859)
    pub total_fuel_mass_kg: f64,
    /// Diámetro hidráulico del subcanal [m] (default: 0.0047)
    pub hydraulic_diameter_m: f64,
    /// Área total de transferencia de calor [m²] (default: 15.1905)
    pub total_heat_transfer_area_m2: f64,
    /// Caudal másico nominal del refrigerante [kg/s] (default: 881.33)
    pub nominal_flow_kg_s: f64,
    /// Velocidad nominal del refrigerante en el subcanal [m/s] (default: 12.5)
    pub nominal_velocity_m_s: f64,
    /// Constante de tiempo del filtro pasa-bajos de HTC [s] (default: 0.1)
    pub filter_time_constant_s: f64,
    /// Cota mínima para el HTC [W/K] (default: 1000.0 para convección natural)
    pub min_htc_w_k: f64,
    /// Tabla de temperaturas para Cpm [°C]
    pub cpm_table_temp_c: Vec<f64>,
    /// Tabla de calores específicos Cpm [J/(kg·K)]
    pub cpm_table_values: Vec<f64>,
    /// Temperatura nominal de entrada de refrigerante [°C] (default: 38.0)
    pub t_inlet_ref_c: f64,
    /// Paso de tiempo de muestreo dt [s] (default: 0.05)
    pub sample_time_dt: f64,
}

impl Default for FuelThermalParams {
    fn default() -> Self {
        Self {
            nominal_power_w: 28.5e6,
            apd_fractions: vec![0.2926748577205830, 0.4340588756881160, 0.2732662665913000],
            total_fuel_mass_kg: 115.5859,
            hydraulic_diameter_m: 0.0047,
            total_heat_transfer_area_m2: 15.1905,
            nominal_flow_kg_s: 881.33,
            nominal_velocity_m_s: 12.5,
            filter_time_constant_s: 0.1,
            min_htc_w_k: 1000.0,
            cpm_table_temp_c: vec![15.0, 60.0, 120.0, 400.0],
            cpm_table_values: vec![532.4689562, 544.5712990, 560.6269137, 635.6860432],
            t_inlet_ref_c: 38.0,
            sample_time_dt: 0.05,
        }
    }
}

/// Interpola linealmente el calor específico de la placa cpm [J/(kg·K)] en función de Tf [°C].
pub fn interpolate_cpm(table_t: &[f64], table_cpm: &[f64], t_c: f64) -> f64 {
    if table_t.is_empty() || table_cpm.is_empty() {
        return 544.57;
    }
    if t_c <= table_t[0] {
        return table_cpm[0];
    }
    let n = table_t.len();
    if t_c >= table_t[n - 1] {
        return table_cpm[n - 1];
    }
    for i in 0..n - 1 {
        let t0 = table_t[i];
        let t1 = table_t[i + 1];
        if t_c >= t0 && t_c <= t1 {
            let frac = (t_c - t0) / (t1 - t0);
            return table_cpm[i] + frac * (table_cpm[i + 1] - table_cpm[i]);
        }
    }
    table_cpm[n - 1]
}

/// Calcula los pesos cuadráticos normalizados (FPD2) para la temperatura Doppler representativa.
pub fn doppler_weights(apd: &[f64]) -> Vec<f64> {
    let sum_sq: f64 = apd.iter().map(|&f| f * f).sum();
    if sum_sq > 0.0 {
        apd.iter().map(|&f| (f * f) / sum_sq).collect()
    } else {
        vec![1.0 / apd.len() as f64; apd.len()]
    }
}

/// Interpola el calor específico del refrigerante Cpc [J/(kg·K)] en función de Tc [°C].
fn coolant_cp(tc: f64) -> f64 {
    const TAB: [(f64, f64); 10] = [
        (20.0, 4182.78),
        (30.0, 4178.71),
        (40.0, 4178.35),
        (50.0, 4180.61),
        (60.0, 4184.74),
        (70.0, 4190.29),
        (80.0, 4197.12),
        (90.0, 4205.40),
        (100.0, 4215.63),
        (110.0, 4228.58),
    ];
    if tc <= TAB[0].0 {
        return TAB[0].1;
    }
    let n = TAB.len();
    if tc >= TAB[n - 1].0 {
        return TAB[n - 1].1;
    }
    for i in 0..n - 1 {
        if tc >= TAB[i].0 && tc <= TAB[i + 1].0 {
            let frac = (tc - TAB[i].0) / (TAB[i + 1].0 - TAB[i].0);
            return TAB[i].1 + frac * (TAB[i + 1].1 - TAB[i].1);
        }
    }
    TAB[n - 1].1
}

/// Bloque dinámico de temperatura de combustible del núcleo (3 zonas axiales).
#[derive(Debug, Clone)]
pub struct FuelThermalBlock {
    pub params: FuelThermalParams,
    pub doppler_weights: Vec<f64>,
    pub f_hd: f64,
    pub ha_i: f64,
    pub mass_node_kg: f64,
}

impl FuelThermalBlock {
    pub fn new(params: FuelThermalParams) -> Self {
        let n_zones = params.apd_fractions.len().max(1);
        let doppler_weights = doppler_weights(&params.apd_fractions);
        let f_hd = 1e4 / (1000.0 * params.hydraulic_diameter_m).powf(0.2);
        let ha_i = params.total_heat_transfer_area_m2 / n_zones as f64;
        let mass_node_kg = params.total_fuel_mass_kg / n_zones as f64;

        Self {
            params,
            doppler_weights,
            f_hd,
            ha_i,
            mass_node_kg,
        }
    }

    /// Calcula el estado estacionario iterativo para una potencia, caudal y temperatura de entrada dados.
    pub fn calculate_steady_state(
        &self,
        power_w: f64,
        flow_kg_s: f64,
        t_in_c: f64,
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = self.params.apd_fractions.len();
        let vel = if self.params.nominal_flow_kg_s > 0.0 {
            (flow_kg_s.abs() / self.params.nominal_flow_kg_s) * self.params.nominal_velocity_m_s
        } else {
            self.params.nominal_velocity_m_s
        };
        let vel_eff = vel.max(0.01);
        let v_term = vel_eff.powf(0.8);

        let pot: Vec<f64> = self.params.apd_fractions.iter().map(|&f| f * power_w).collect();

        // Semilla inicial
        let mut rti0 = vec![3.3e-6; n];
        let mut tc0 = vec![t_in_c + 2.0, t_in_c + 5.0, t_in_c + 7.7];
        let mut tf0 = vec![t_in_c + 30.0; n];
        let mut htc_ai = vec![300000.0; n];

        let w_eff = flow_kg_s.abs().max(0.1);

        for _ in 0..500 {
            let mut tc_prev = t_in_c;
            for i in 0..n {
                let cpc = coolant_cp(tc0[i]);
                tc0[i] = tc_prev + pot[i] / (w_eff * cpc);
                tc_prev = tc0[i];
                tf0[i] = tc0[i] + pot[i] * rti0[i];

                let t_film = (0.5 * (tf0[i] + tc0[i])).abs().max(0.1);
                let htc = v_term * (0.255 * t_film.powf(0.355)) * self.f_hd * self.ha_i;
                htc_ai[i] = htc.max(self.params.min_htc_w_k);
                rti0[i] = 1.0 / htc_ai[i];
            }
        }

        (tf0, tc0, htc_ai)
    }

    /// Builder a partir de JSON
    pub fn build(params: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        let mut p = FuelThermalParams::default();
        if let Some(v) = params.get("nominal_power_w").and_then(|v| v.as_f64()) {
            p.nominal_power_w = v;
        } else if let Some(v) = params.get("nominal_power_mw").and_then(|v| v.as_f64()) {
            p.nominal_power_w = v * 1e6;
        }
        if let Some(arr) = params.get("apd_fractions").and_then(|v| v.as_array()) {
            let parsed: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
            if parsed.len() == 3 {
                p.apd_fractions = parsed;
            }
        }
        if let Some(v) = params.get("total_fuel_mass_kg").and_then(|v| v.as_f64()) {
            p.total_fuel_mass_kg = v;
        }
        if let Some(v) = params.get("hydraulic_diameter_m").and_then(|v| v.as_f64()) {
            p.hydraulic_diameter_m = v;
        }
        if let Some(v) = params.get("total_heat_transfer_area_m2").and_then(|v| v.as_f64()) {
            p.total_heat_transfer_area_m2 = v;
        }
        if let Some(v) = params.get("nominal_flow_kg_s").and_then(|v| v.as_f64()) {
            p.nominal_flow_kg_s = v;
        }
        if let Some(v) = params.get("nominal_velocity_m_s").and_then(|v| v.as_f64()) {
            p.nominal_velocity_m_s = v;
        }
        if let Some(v) = params.get("filter_time_constant_s").and_then(|v| v.as_f64()) {
            p.filter_time_constant_s = v;
        }
        if let Some(v) = params.get("min_htc_w_k").and_then(|v| v.as_f64()) {
            p.min_htc_w_k = v;
        }
        if let Some(v) = params.get("t_inlet_ref_c").and_then(|v| v.as_f64()) {
            p.t_inlet_ref_c = v;
        }
        if let Some(v) = params.get("sample_time_dt").and_then(|v| v.as_f64()) {
            p.sample_time_dt = v;
        }

        Ok(Box::new(Self::new(p)))
    }
}

impl Block for FuelThermalBlock {
    fn num_states(&self) -> usize {
        // 3 temperaturas de combustible Tf[0..3] + 3 HTCs filtrados HTC[0..3] = 6 estados
        self.params.apd_fractions.len() * 2
    }

    fn num_inputs(&self) -> usize {
        // Puerto 0: Potencia total P [W] (o kW/MW auto-convertido)
        // Puerto 1: Caudal másico de refrigerante W [kg/s]
        // Puerto 2: Temperatura de refrigerante de entrada Tin [°C]
        // Puerto 3: Temperatura de refrigerante de salida Tout [°C] (opcional)
        4
    }

    fn num_outputs(&self) -> usize {
        // 0: T_fuel_doppler (ponderada con FPD2) [°C]
        // 1: T_fuel_max (pico de combustible) [°C]
        // 2: T_fuel_avg (promedio aritmético) [°C]
        // 3: T_fuel_zone_1 [°C]
        // 4: T_fuel_zone_2 [°C]
        // 5: T_fuel_zone_3 [°C]
        // 6: HTC_avg [W/K]
        // 7: Velocidad de refrigerante [m/s]
        // 8: T_coolant_avg [°C]
        9
    }

    fn input_width(&self, _port: usize) -> usize {
        1
    }

    fn output_width(&self, _port: usize) -> usize {
        1
    }

    fn has_direct_feedthrough(&self) -> bool {
        true
    }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {
        // Bloque discreto (actualizado con integrador exponencial en update)
    }

    fn get_initial_conditions(&self, x0: &mut [f64]) {
        let (tf0, _, htc0) = self.calculate_steady_state(
            self.params.nominal_power_w,
            self.params.nominal_flow_kg_s,
            self.params.t_inlet_ref_c,
        );
        let n = self.params.apd_fractions.len();
        for i in 0..n {
            if x0.len() > i {
                x0[i] = tf0[i];
            }
            if x0.len() > n + i {
                x0[n + i] = htc0[i];
            }
        }
    }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let n = self.params.apd_fractions.len();
        let dt = self.params.sample_time_dt;

        // 1. Interpretar Entradas
        let p_in = if u.is_empty() { self.params.nominal_power_w } else { u[0] };
        // Auto-conversión si entra en MW o kW:
        let power_w = if p_in < 500.0 {
            p_in * 1e6 // vino en MW
        } else if p_in < 500_000.0 {
            p_in * 1e3 // vino en kW
        } else {
            p_in // vino en W
        };

        let flow_kg_s = if u.len() > 1 { u[1] } else { self.params.nominal_flow_kg_s };
        let t_in_c = if u.len() > 2 {
            let val = u[2];
            if val > 200.0 { val - 273.15 } else { val }
        } else {
            self.params.t_inlet_ref_c
        };
        let t_out_c_opt = if u.len() > 3 && !u[3].is_nan() {
            let val = u[3];
            Some(if val > 200.0 { val - 273.15 } else { val })
        } else {
            None
        };

        // 2. Velocidad de refrigerante
        let vel = if self.params.nominal_flow_kg_s > 0.0 {
            (flow_kg_s.abs() / self.params.nominal_flow_kg_s) * self.params.nominal_velocity_m_s
        } else {
            self.params.nominal_velocity_m_s
        };
        let vel_eff = vel.max(0.01);
        let v_term = vel_eff.powf(0.8);

        // 3. Distribución de temperatura de refrigerante
        let mut tc = vec![t_in_c; n];
        if let Some(t_out) = t_out_c_opt {
            let delta_t = (t_out - t_in_c).max(0.0);
            let mut accum = 0.0;
            for i in 0..n {
                let f = self.params.apd_fractions[i];
                accum += f;
                tc[i] = t_in_c + accum * delta_t;
            }
        } else {
            let w_eff = flow_kg_s.abs().max(0.1);
            let mut tc_prev = t_in_c;
            for i in 0..n {
                let qi = self.params.apd_fractions[i] * power_w;
                let cpc = coolant_cp(tc_prev);
                let tc_exit = tc_prev + qi / (w_eff * cpc);
                tc[i] = tc_exit;
                tc_prev = tc_exit;
            }
        }

        // 4. Actualizar HTC y Temperatura de Combustible con Integrador Exponencial A-estable
        let alpha = dt / (self.params.filter_time_constant_s + dt);
        let mut x_next = vec![0.0; self.num_states()];

        for i in 0..n {
            let tf_curr = x.get(i).copied().unwrap_or(self.params.t_inlet_ref_c + 30.0);
            let htc_curr = x.get(n + i).copied().unwrap_or(300000.0);

            let t_film = (0.5 * (tf_curr + tc[i])).abs().max(0.1);
            let htc_target = (v_term * (0.255 * t_film.powf(0.355)) * self.f_hd * self.ha_i)
                .max(self.params.min_htc_w_k);

            // Filtro pasa-bajos
            let htc_next = htc_curr + alpha * (htc_target - htc_curr);

            // Dinámica térmica del combustible:
            let qi = self.params.apd_fractions[i] * power_w;
            let cpm = interpolate_cpm(
                &self.params.cpm_table_temp_c,
                &self.params.cpm_table_values,
                tf_curr,
            );
            let lambda_th = (htc_next / (self.mass_node_kg * cpm)).max(1e-4);
            let tf_inf = tc[i] + qi / htc_next;

            let exp_term = (-lambda_th * dt).exp();
            let tf_next = tf_inf + (tf_curr - tf_inf) * exp_term;

            x_next[i] = tf_next;
            x_next[n + i] = htc_next;
        }

        x_next
    }

    fn outputs(&self, _t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        let n = self.params.apd_fractions.len();
        let mut tf = vec![0.0; n];
        let mut htc = vec![0.0; n];

        for i in 0..n {
            tf[i] = x.get(i).copied().unwrap_or(0.0);
            htc[i] = x.get(n + i).copied().unwrap_or(0.0);
        }

        let tf_doppler: f64 = tf.iter().zip(&self.doppler_weights).map(|(&t, &w)| t * w).sum();
        let tf_max = tf.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let tf_avg: f64 = tf.iter().sum::<f64>() / n as f64;
        let htc_avg: f64 = htc.iter().sum::<f64>() / n as f64;

        let flow_kg_s = if u.len() > 1 { u[1] } else { self.params.nominal_flow_kg_s };
        let vel = if self.params.nominal_flow_kg_s > 0.0 {
            (flow_kg_s.abs() / self.params.nominal_flow_kg_s) * self.params.nominal_velocity_m_s
        } else {
            self.params.nominal_velocity_m_s
        };

        let t_in_c = if u.len() > 2 {
            let val = u[2];
            if val > 200.0 { val - 273.15 } else { val }
        } else {
            self.params.t_inlet_ref_c
        };
        let t_out_c = if u.len() > 3 && !u[3].is_nan() {
            let val = u[3];
            if val > 200.0 { val - 273.15 } else { val }
        } else {
            t_in_c + 7.7
        };
        let tc_avg = 0.5 * (t_in_c + t_out_c);

        if !y.is_empty() { y[0] = tf_doppler; }
        if y.len() > 1 { y[1] = tf_max; }
        if y.len() > 2 { y[2] = tf_avg; }
        if y.len() > 3 { y[3] = tf[0]; }
        if y.len() > 4 { y[4] = tf[1]; }
        if y.len() > 5 { y[5] = tf[2]; }
        if y.len() > 6 { y[6] = htc_avg; }
        if y.len() > 7 { y[7] = vel; }
        if y.len() > 8 { y[8] = tc_avg; }
    }
}
