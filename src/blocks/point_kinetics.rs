use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::blocks::{Block, BlockRegistry};

/// Parámetros nucleares y cinéticos del modelo de punto cinético.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointKineticsParams {
    /// Vida media de neutrones prontos Lambda [s]
    pub prompt_lifetime_lambda: f64,
    /// Fracción efectiva total de neutrones retardados Beta [-]
    pub beta_total: f64,
    /// Fracciones de precursores Beta_i [adimensional]
    pub beta_i: Vec<f64>,
    /// Constantes de decaimiento de precursores Lambda_i [1/s]
    pub lambda_i: Vec<f64>,
    /// Coeficiente de realimentación por temperatura de combustible [pcm/°C]
    pub alpha_fuel_pcm: f64,
    /// Coeficiente de realimentación por temperatura de refrigerante [pcm/°C]
    pub alpha_coolant_pcm: f64,
    /// Temperatura de referencia de combustible [°C]
    pub t_fuel_ref: f64,
    /// Temperatura de referencia de refrigerante [°C]
    pub t_coolant_ref: f64,
    /// Constantes de decaimiento para grupos gamma (Decay Heat) [1/s]
    pub lambda_gamma: Vec<f64>,
    /// Fracciones de energía para grupos gamma [-]
    pub f_gamma: Vec<f64>,
    /// Potencia neutrónica nominal [kW]
    pub nominal_power_kw: f64,
    /// Fuente externa de neutrones S [kW/s]
    pub neutron_source_s: f64,
    /// Paso de tiempo de integración [s]
    pub sample_time_dt: f64,

    /// Envenenamiento por Xenón-135 e Iodo-135 habilitado
    #[serde(default)]
    pub xenon_enabled: bool,
    /// Rendimiento de fisión directo de Iodo-135 (tasa de creación por flujo) [-]
    #[serde(default = "default_yield_iodine")]
    pub yield_iodine: f64,
    /// Rendimiento de fisión directo de Xenón-135 (tasa de creación por flujo) [-]
    #[serde(default = "default_yield_xenon")]
    pub yield_xenon: f64,
    /// Constante de decaimiento de Iodo-135 en Xenón-135 [1/s]
    #[serde(default = "default_lambda_iodine")]
    pub lambda_iodine: f64,
    /// Constante de decaimiento de Xenón-135 [1/s] (1 / tiempo de vida)
    #[serde(default = "default_lambda_xenon")]
    pub lambda_xenon: f64,
    /// Tasa de quemado de Xenón por el flujo neutrónico a potencia nominal (sigma_a * Phi_0) [1/s]
    #[serde(default = "default_xenon_burnout")]
    pub xenon_burnout_rate: f64,
    /// Reactividad por Xenón en equilibrio a potencia nominal [pcm]
    #[serde(default = "default_xenon_equilibrium_reactivity")]
    pub xenon_equilibrium_reactivity_pcm: f64,
    /// Estado inicial del xenón: "equilibrium" (por defecto) o "clean"
    #[serde(default = "default_xenon_initial_state")]
    pub xenon_initial_state: String,

    /// Habilita el modelo de decaimiento gamma (Decay Heat)
    #[serde(default = "default_true")]
    pub gamma_decay_enabled: bool,
    /// Estado inicial del decaimiento gamma: "equilibrium" (por defecto) o "clean"/"zero"
    #[serde(default = "default_gamma_initial_state")]
    pub gamma_initial_state: String,

    /// Modo de reseteo de Xenón: "nominal_power" (por defecto), "current_power", o "clean"
    #[serde(default = "default_reset_mode")]
    pub xenon_reset_mode: String,
    /// Modo de reseteo de decaimiento gamma: "nominal_power" (por defecto), "current_power", o "clean"
    #[serde(default = "default_reset_mode")]
    pub gamma_reset_mode: String,

    /// Habilita el modelo de quemado del núcleo por días a plena potencia (DPP / EFPD)
    #[serde(default)]
    pub burnup_enabled: bool,
    /// Días de quemado iniciales [días]
    #[serde(default)]
    pub initial_burnup_days: f64,
    /// Tabla de días de quemado [días]
    #[serde(default = "default_burnup_table_days")]
    pub burnup_table_days: Vec<f64>,
    /// Tabla de reactividad por quemado [pcm]
    #[serde(default = "default_burnup_table_reactivity_pcm")]
    pub burnup_table_reactivity_pcm: Vec<f64>,
}

fn default_true() -> bool { true }
fn default_gamma_initial_state() -> String { "equilibrium".to_string() }
fn default_reset_mode() -> String { "nominal_power".to_string() }
fn default_yield_iodine() -> f64 { 0.0628 }
fn default_yield_xenon() -> f64 { 0.0026 }
fn default_lambda_iodine() -> f64 { 2.9e-5 }
fn default_lambda_xenon() -> f64 { 2.1e-5 }
fn default_xenon_burnout() -> f64 { 7.41e-4 }
fn default_xenon_equilibrium_reactivity() -> f64 { -3650.0 }
fn default_xenon_initial_state() -> String { "equilibrium".to_string() }
fn default_burnup_table_days() -> Vec<f64> {
    vec![0.0, 3.0, 8.0, 14.0, 19.0, 27.0, 29.5]
}
fn default_burnup_table_reactivity_pcm() -> Vec<f64> {
    vec![0.0, 0.0, -304.5, -602.0, -883.0, -1153.0, -1159.0]
}

/// Interpola linealmente la reactividad por quemado del núcleo a partir de los días a plena potencia (DPP / EFPD).
pub fn interpolate_burnup_reactivity_pcm(days: &[f64], rho_pcm: &[f64], dpp: f64) -> f64 {
    if days.is_empty() || rho_pcm.is_empty() {
        return 0.0;
    }
    let n = days.len().min(rho_pcm.len());
    if n == 1 || dpp <= days[0] {
        return rho_pcm[0];
    }
    if dpp >= days[n - 1] {
        return rho_pcm[n - 1];
    }
    for i in 0..n - 1 {
        if dpp >= days[i] && dpp <= days[i + 1] {
            let d0 = days[i];
            let d1 = days[i + 1];
            let dt = d1 - d0;
            if dt.abs() < 1e-12 {
                return rho_pcm[i];
            }
            let r0 = rho_pcm[i];
            let r1 = rho_pcm[i + 1];
            let frac = (dpp - d0) / dt;
            return r0 + frac * (r1 - r0);
        }
    }
    rho_pcm[n - 1]
}

impl Default for PointKineticsParams {
    fn default() -> Self {
        // Datos oficiales RA-10 (Tabla 2 SGI IT/48/RA10/4601-1-004)
        // Convertidos a 5 o 6 grupos de precursores
        let beta_pcm = vec![26.9608, 153.1447, 135.1596, 290.9512, 92.8247, 19.3965];
        let beta_i: Vec<f64> = beta_pcm.iter().map(|&b| b * 1e-5).collect();
        let beta_total: f64 = beta_i.iter().sum();

        Self {
            prompt_lifetime_lambda: 86.0e-6,
            beta_total,
            beta_i,
            lambda_i: vec![0.0124, 0.0305, 0.1110, 0.3010, 1.1400, 3.0100],
            alpha_fuel_pcm: -2.2,
            alpha_coolant_pcm: -14.2,
            t_fuel_ref: 65.0,
            t_coolant_ref: 45.0,
            lambda_gamma: vec![1.0, 0.08, 0.05, 0.0025, 1.0e-6, 5.0e-8],
            f_gamma: vec![0.003, 0.00825, 0.0105, 0.026, 0.0137, 0.0085],
            nominal_power_kw: 30.0, // 30 kW nominal de potencia de baja potencia / calibración
            neutron_source_s: 0.0,
            sample_time_dt: 0.01, // 0.01s (TBC)
            xenon_enabled: false,
            yield_iodine: default_yield_iodine(),
            yield_xenon: default_yield_xenon(),
            lambda_iodine: default_lambda_iodine(),
            lambda_xenon: default_lambda_xenon(),
            xenon_burnout_rate: default_xenon_burnout(),
            xenon_equilibrium_reactivity_pcm: default_xenon_equilibrium_reactivity(),
            xenon_initial_state: default_xenon_initial_state(),
            gamma_decay_enabled: true,
            gamma_initial_state: default_gamma_initial_state(),
            xenon_reset_mode: default_reset_mode(),
            gamma_reset_mode: default_reset_mode(),
            burnup_enabled: false,
            initial_burnup_days: 0.0,
            burnup_table_days: default_burnup_table_days(),
            burnup_table_reactivity_pcm: default_burnup_table_reactivity_pcm(),
        }
    }
}

/// Bloque de Cinética Puntual Neutrónica con realimentaciones térmicas y Decay Heat.
///
/// Modela:
/// - 6 (o 5) grupos de precursores de neutrones retardados
/// - 6 grupos de productos de fisión para calor de decaimiento (Gamma Decay Heat)
/// - Realimentación Doppler de combustible ($\alpha_F = -2.2\text{ pcm/}^\circ\text{C}$)
/// - Realimentación de moderador/refrigerante ($\alpha_C = -14.2\text{ pcm/}^\circ\text{C}$)
/// - Integrador Hansen exponencial en tiempo discreto incondicionalmente estable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointKineticsBlock {
    pub params: PointKineticsParams,
    pub tag: String,
}

impl PointKineticsBlock {
    pub fn new(params: PointKineticsParams) -> Self {
        Self {
            params,
            tag: "CORE_POINT_KINETICS".to_string(),
        }
    }

    /// Configura el modelo para 5 grupos de precursores (colapsando los grupos 5 y 6).
    pub fn with_5_precursor_groups(mut self) -> Self {
        let b5_pcm = 92.8247 + 19.3965; // 112.2212 pcm
        let l5 = 1.1400;
        let l6 = 3.0100;
        // Promedio armónico ponderado por reactividad
        let l5_eff = (92.8247 + 19.3965) / (92.8247 / l5 + 19.3965 / l6);

        self.params.beta_i = vec![
            26.9608 * 1e-5,
            153.1447 * 1e-5,
            135.1596 * 1e-5,
            290.9512 * 1e-5,
            b5_pcm * 1e-5,
        ];
        self.params.lambda_i = vec![0.0124, 0.0305, 0.1110, 0.3010, l5_eff];
        self.params.beta_total = self.params.beta_i.iter().sum();
        self
    }

    pub fn with_nominal_power(mut self, power_kw: f64) -> Self {
        self.params.nominal_power_kw = power_kw;
        self
    }

    pub fn with_sample_time(mut self, dt: f64) -> Self {
        self.params.sample_time_dt = dt;
        self
    }

    /// Habilita el modelo de envenenamiento por Xenón-135 e Iodo-135
    pub fn with_xenon(mut self) -> Self {
        self.params.xenon_enabled = true;
        self
    }

    /// Configura los parámetros completos del modelo de Xenón e Iodo
    pub fn with_xenon_params(
        mut self,
        yield_iodine: f64,
        yield_xenon: f64,
        lambda_iodine: f64,
        lambda_xenon: f64,
        xenon_burnout_rate: f64,
        xenon_equilibrium_reactivity_pcm: f64,
        initial_state: impl Into<String>,
    ) -> Self {
        self.params.xenon_enabled = true;
        self.params.yield_iodine = yield_iodine;
        self.params.yield_xenon = yield_xenon;
        self.params.lambda_iodine = lambda_iodine;
        self.params.lambda_xenon = lambda_xenon;
        self.params.xenon_burnout_rate = xenon_burnout_rate;
        self.params.xenon_equilibrium_reactivity_pcm = xenon_equilibrium_reactivity_pcm;
        self.params.xenon_initial_state = initial_state.into();
        self
    }

    /// Configura precursores de neutrones retardados personalizados.
    /// Si la suma de `beta_pcm` > 0.1, se asume que los valores están en pcm y se convierten a adimensionales (* 1e-5).
    pub fn with_precursors(mut self, beta_pcm: Vec<f64>, lambda_i: Vec<f64>) -> Self {
        let is_pcm = beta_pcm.iter().sum::<f64>() > 0.1;
        self.params.beta_i = if is_pcm {
            beta_pcm.iter().map(|&b| b * 1e-5).collect()
        } else {
            beta_pcm
        };
        self.params.lambda_i = lambda_i;
        self.params.beta_total = self.params.beta_i.iter().sum();
        self
    }

    /// Configura grupos gamma de calor de decaimiento (Decay Heat).
    pub fn with_gamma_decay_params(mut self, lambda_gamma: Vec<f64>, f_gamma: Vec<f64>) -> Self {
        self.params.gamma_decay_enabled = true;
        self.params.lambda_gamma = lambda_gamma;
        self.params.f_gamma = f_gamma;
        self
    }

    /// Configura parámetros del modelo de quemado del núcleo por días a plena potencia (DPP / EFPD).
    pub fn with_burnup_params(
        mut self,
        initial_days: f64,
        table_days: Vec<f64>,
        table_reactivity_pcm: Vec<f64>,
    ) -> Self {
        self.params.burnup_enabled = true;
        self.params.initial_burnup_days = initial_days;
        let is_pcm = table_reactivity_pcm.iter().any(|&val| val.abs() > 0.1);
        self.params.burnup_table_reactivity_pcm = if is_pcm {
            table_reactivity_pcm
        } else {
            table_reactivity_pcm.iter().map(|&val| val * 1e5).collect()
        };
        self.params.burnup_table_days = table_days;
        self
    }

    pub fn build(params: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        let mut p = PointKineticsParams::default();
        if let Some(p_nom) = params.get("nominal_power_kw").and_then(|v| v.as_f64()) {
            p.nominal_power_kw = p_nom;
        }
        if let Some(dt) = params.get("sample_time_dt").and_then(|v| v.as_f64()) {
            p.sample_time_dt = dt;
        }
        if let Some(a_f) = params.get("alpha_fuel_pcm").and_then(|v| v.as_f64()) {
            p.alpha_fuel_pcm = a_f;
        }
        if let Some(a_c) = params.get("alpha_coolant_pcm").and_then(|v| v.as_f64()) {
            p.alpha_coolant_pcm = a_c;
        }
        if let Some(b) = params.get("xenon_enabled").and_then(|v| v.as_bool()) {
            p.xenon_enabled = b;
        }
        if let Some(y) = params.get("yield_iodine").and_then(|v| v.as_f64()) {
            p.yield_iodine = y;
            p.xenon_enabled = true;
        }
        if let Some(y) = params.get("yield_xenon").and_then(|v| v.as_f64()) {
            p.yield_xenon = y;
            p.xenon_enabled = true;
        }
        if let Some(l) = params.get("lambda_iodine").and_then(|v| v.as_f64()) {
            p.lambda_iodine = l;
            p.xenon_enabled = true;
        }
        if let Some(l) = params.get("lambda_xenon").and_then(|v| v.as_f64()) {
            p.lambda_xenon = l;
            p.xenon_enabled = true;
        }
        if let Some(s) = params.get("xenon_burnout_rate").and_then(|v| v.as_f64()) {
            p.xenon_burnout_rate = s;
            p.xenon_enabled = true;
        }
        if let Some(r) = params.get("xenon_equilibrium_reactivity_pcm").and_then(|v| v.as_f64()) {
            p.xenon_equilibrium_reactivity_pcm = r;
        }
        if let Some(init) = params.get("xenon_initial_state").and_then(|v| v.as_str()) {
            p.xenon_initial_state = init.to_string();
        }
        if let Some(m) = params.get("xenon_reset_mode").and_then(|v| v.as_str()) {
            p.xenon_reset_mode = m.to_string();
        }

        // Parámetros de Decaimiento Gamma (Decay Heat)
        if let Some(en) = params.get("gamma_decay_enabled").and_then(|v| v.as_bool()) {
            p.gamma_decay_enabled = en;
        }
        if let Some(init) = params.get("gamma_initial_state").and_then(|v| v.as_str()) {
            p.gamma_initial_state = init.to_string();
        }
        if let Some(m) = params.get("gamma_reset_mode").and_then(|v| v.as_str()) {
            p.gamma_reset_mode = m.to_string();
        }
        let custom_lambda_gamma = params.get("gamma_decay_constants")
            .or_else(|| params.get("lambda_gamma"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>());
        let custom_f_gamma = params.get("gamma_energy_fractions")
            .or_else(|| params.get("f_gamma"))
            .or_else(|| params.get("beta_gamma"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>());

        if let (Some(l_g), Some(f_g)) = (custom_lambda_gamma, custom_f_gamma) {
            if l_g.len() == f_g.len() {
                p.gamma_decay_enabled = true;
                p.lambda_gamma = l_g;
                p.f_gamma = f_g;
            }
        } else if !p.gamma_decay_enabled {
            p.lambda_gamma.clear();
            p.f_gamma.clear();
        }

        // Parámetros de Quemado del Núcleo (Burnup / DPP)
        if let Some(en) = params.get("burnup_enabled").and_then(|v| v.as_bool()) {
            p.burnup_enabled = en;
        }
        if let Some(init) = params.get("initial_burnup_days")
            .or_else(|| params.get("burnup_days"))
            .or_else(|| params.get("initial_dpp"))
            .or_else(|| params.get("extra_dpp"))
            .and_then(|v| v.as_f64())
        {
            p.initial_burnup_days = init;
            p.burnup_enabled = true;
        }
        let custom_burnup_days = params.get("burnup_table_days")
            .or_else(|| params.get("table_days"))
            .or_else(|| params.get("dpp_days"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>());
        let custom_burnup_reac = params.get("burnup_table_reactivity_pcm")
            .or_else(|| params.get("burnup_table_reactivity"))
            .or_else(|| params.get("table_reactivity_pcm"))
            .or_else(|| params.get("table_reactivity"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>());

        if let (Some(d), Some(r)) = (custom_burnup_days, custom_burnup_reac) {
            if d.len() == r.len() && !d.is_empty() {
                p.burnup_enabled = true;
                p.burnup_table_days = d;
                let is_pcm = r.iter().any(|&val| val.abs() > 0.1);
                p.burnup_table_reactivity_pcm = if is_pcm {
                    r
                } else {
                    r.iter().map(|&val| val * 1e5).collect()
                };
            }
        }

        // Parámetros de Precursores
        let custom_beta_i = params.get("precursor_betas_pcm")
            .or_else(|| params.get("beta_i_pcm"))
            .or_else(|| params.get("beta_i"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>());
        let custom_lambda_i = params.get("precursor_decay_constants")
            .or_else(|| params.get("lambda_i"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<f64>>());

        let block = if let (Some(b_i), Some(l_i)) = (custom_beta_i, custom_lambda_i) {
            let is_pcm = b_i.iter().sum::<f64>() > 0.1;
            p.beta_i = if is_pcm { b_i.iter().map(|&b| b * 1e-5).collect() } else { b_i };
            p.lambda_i = l_i;
            p.beta_total = p.beta_i.iter().sum();
            Self::new(p)
        } else {
            let n_groups = params.get("num_precursor_groups").and_then(|v| v.as_u64()).unwrap_or(6);
            if n_groups == 5 {
                Self::new(p).with_5_precursor_groups()
            } else {
                Self::new(p)
            }
        };

        Ok(Box::new(block))
    }
}

impl Block for PointKineticsBlock {
    fn num_states(&self) -> usize {
        // Estado 0: Potencia pronta P_n
        // Estados 1..M: Concentraciones de precursores C_i
        // Estados M+1..M+G: Estados de energía gamma H_k
        // Estados M+G+1..M+G+2 (si xenon_enabled): Iodo I(t) y Xenón X(t)
        // Estado final (si burnup_enabled): Días de quemado a plena potencia DPP(t)
        1 + self.params.beta_i.len()
            + self.params.lambda_gamma.len()
            + if self.params.xenon_enabled { 2 } else { 0 }
            + if self.params.burnup_enabled { 1 } else { 0 }
    }

    fn num_inputs(&self) -> usize {
        // Puerto 0: Reactividad externa rho_ext [pcm]
        // Puerto 1: Temperatura de combustible T_fuel [°C]
        // Puerto 2: Temperatura promedio de refrigerante T_coolant [°C]
        // Puerto 3 (si xenon_enabled): Señal externa de reset de Xenón (>0.5 activa reset)
        // Puerto +1 (si burnup_enabled): Entrada de días de quemado EXTRA-DPP [días]
        let base = if self.params.xenon_enabled { 4 } else { 3 };
        if self.params.burnup_enabled { base + 1 } else { base }
    }

    fn num_outputs(&self) -> usize {
        // Puerto 0: Potencia térmica total P_thermal [kW]
        // Puerto 1: Potencia neutrónica pronta P_n [kW]
        // Puerto 2: Reactividad total rho_total [pcm]
        // Puerto 3: Reactividad de realimentaciones rho_feedback [pcm]
        // Si xenon_enabled:
        // Puerto 4: Reactividad por Xenón actual rho_xenon [pcm]
        // Puerto 5: Concentración relativa de Xenón X(t) / X_eq [-]
        // Puerto 6: Concentración relativa de Iodo I(t) / I_eq [-]
        // Si burnup_enabled:
        // Puerto +1: Reactividad por quemado [pcm]
        // Puerto +2: Días de quemado DPP [días]
        let base = if self.params.xenon_enabled { 7 } else { 4 };
        if self.params.burnup_enabled { base + 2 } else { base }
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

    fn sample_time(&self) -> Option<f64> {
        Some(self.params.sample_time_dt)
    }

    fn get_initial_conditions(&self, x: &mut [f64]) {
        let p0 = self.params.nominal_power_kw;
        let lambda = self.params.prompt_lifetime_lambda;
        x[0] = p0; // P_n(0) = P_0

        // Precursores en equilibrio estacionario: C_i(0) = (beta_i / (lambda_i * Lambda)) * P_0
        let m = self.params.beta_i.len();
        for i in 0..m {
            let bi = self.params.beta_i[i];
            let li = self.params.lambda_i[i];
            x[1 + i] = (bi / (li * lambda)) * p0;
        }

        // Grupos Gamma en equilibrio estacionario: H_k(0) = (f_k / lambda_gamma_k) * P_0
        let g = self.params.lambda_gamma.len();
        for k in 0..g {
            let fk = self.params.f_gamma[k];
            let lk = self.params.lambda_gamma[k];
            if self.params.gamma_initial_state == "clean" || self.params.gamma_initial_state == "zero" {
                x[1 + m + k] = 0.0;
            } else {
                x[1 + m + k] = if lk > 0.0 { (fk / lk) * p0 } else { 0.0 };
            }
        }

        // Condiciones iniciales de Iodo y Xenón
        if self.params.xenon_enabled && x.len() >= 1 + m + g + 2 {
            let li = self.params.lambda_iodine;
            let lx_eff_eq = self.params.lambda_xenon + self.params.xenon_burnout_rate;
            let i_eq = if li > 0.0 { self.params.yield_iodine / li } else { 0.0 };
            let x_eq = if lx_eff_eq > 0.0 {
                (self.params.yield_iodine + self.params.yield_xenon) / lx_eff_eq
            } else {
                0.0
            };

            if self.params.xenon_initial_state == "clean" {
                x[1 + m + g] = 0.0;
                x[1 + m + g + 1] = 0.0;
            } else {
                x[1 + m + g] = i_eq;
                x[1 + m + g + 1] = x_eq;
            }
        }

        // Condiciones iniciales de Quemado del Núcleo (DPP)
        if self.params.burnup_enabled {
            let idx_b = 1 + m + g + if self.params.xenon_enabled { 2 } else { 0 };
            if x.len() > idx_b {
                x[idx_b] = self.params.initial_burnup_days;
            }
        }
    }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {
        // Bloque puramente discreto (resuelto por Hansen Exponential Integrator)
    }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let dt = self.params.sample_time_dt;
        let lambda_prompt = self.params.prompt_lifetime_lambda;
        let beta = self.params.beta_total;
        let s = self.params.neutron_source_s;

        let rho_ext_pcm = if u.is_empty() { 0.0 } else { u[0] };
        let t_fuel = if u.len() > 1 { u[1] } else { self.params.t_fuel_ref };
        let t_coolant = if u.len() > 2 { u[2] } else { self.params.t_coolant_ref };
        let reset_xenon = if u.len() > 3 { u[3] > 0.5 } else { false };
        let reset_gamma = if u.len() > 4 { u[4] > 0.5 } else { false };

        // 1. Calcular Realimentaciones Térmicas de Reactividad [pcm]
        let delta_tf = t_fuel - self.params.t_fuel_ref;
        let delta_tc = t_coolant - self.params.t_coolant_ref;
        let rho_thermal_fb_pcm = self.params.alpha_fuel_pcm * delta_tf + self.params.alpha_coolant_pcm * delta_tc;

        let p_curr = x[0];
        let m = self.params.beta_i.len();
        let g = self.params.lambda_gamma.len();

        // 2. Dinámica de Xenón e Iodo
        let phi = if self.params.nominal_power_kw > 0.0 {
            (p_curr / self.params.nominal_power_kw).max(0.0)
        } else {
            1.0
        };

        let (i_next, x_next_val, rho_xe_delta_pcm) = if self.params.xenon_enabled {
            let idx_i = 1 + m + g;
            let idx_x = 1 + m + g + 1;
            let i_curr = x.get(idx_i).copied().unwrap_or(0.0);
            let x_curr = x.get(idx_x).copied().unwrap_or(0.0);

            let li = self.params.lambda_iodine;
            let lx_eff_eq = self.params.lambda_xenon + self.params.xenon_burnout_rate;
            let x_eq = if lx_eff_eq > 0.0 {
                (self.params.yield_iodine + self.params.yield_xenon) / lx_eff_eq
            } else {
                1.0
            };

            if reset_xenon {
                let rho_xe_ref = if self.params.xenon_initial_state == "clean" {
                    0.0
                } else {
                    self.params.xenon_equilibrium_reactivity_pcm
                };

                let (i_res, x_res, rho_xe_res) = match self.params.xenon_reset_mode.as_str() {
                    "clean" | "zero" => (0.0, 0.0, 0.0),
                    "current_power" => {
                        let i_p = if li > 0.0 { (self.params.yield_iodine / li) * phi } else { 0.0 };
                        let lx_p = self.params.lambda_xenon + self.params.xenon_burnout_rate * phi;
                        let x_p = if lx_p > 0.0 {
                            ((self.params.yield_iodine + self.params.yield_xenon) / lx_p) * phi
                        } else {
                            0.0
                        };
                        let rho_p = if x_eq > 0.0 {
                            self.params.xenon_equilibrium_reactivity_pcm * (x_p / x_eq)
                        } else {
                            0.0
                        };
                        (i_p, x_p, rho_p)
                    }
                    _ => {
                        // "nominal_power" (por defecto)
                        let i_nom = if li > 0.0 { self.params.yield_iodine / li } else { 0.0 };
                        let rho_nom = self.params.xenon_equilibrium_reactivity_pcm;
                        (i_nom, x_eq, rho_nom)
                    }
                };

                let delta_rho = rho_xe_res - rho_xe_ref;
                (i_res, x_res, delta_rho)
            } else {
                let exp_i = (-li * dt).exp();
                let int_i = if li.abs() > 1e-12 { (1.0 - exp_i) / li } else { dt };
                let i_n = i_curr * exp_i + self.params.yield_iodine * phi * int_i;

                let lx_eff = self.params.lambda_xenon + self.params.xenon_burnout_rate * phi;
                let exp_x = (-lx_eff * dt).exp();
                let int_x = if lx_eff.abs() > 1e-12 { (1.0 - exp_x) / lx_eff } else { dt };
                let source_x = self.params.yield_xenon * phi + li * i_curr;
                let x_n = x_curr * exp_x + source_x * int_x;

                let rho_xe_curr = if x_eq > 0.0 {
                    self.params.xenon_equilibrium_reactivity_pcm * (x_curr / x_eq)
                } else {
                    0.0
                };
                let rho_xe_ref = if self.params.xenon_initial_state == "clean" {
                    0.0
                } else {
                    self.params.xenon_equilibrium_reactivity_pcm
                };
                let delta_rho = rho_xe_curr - rho_xe_ref;

                (i_n, x_n, delta_rho)
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        let (b_next, rho_burnup_pcm) = if self.params.burnup_enabled {
            let idx_b = 1 + m + g + if self.params.xenon_enabled { 2 } else { 0 };
            let b_curr = x.get(idx_b).copied().unwrap_or(self.params.initial_burnup_days);

            let dpp_in = if u.len() > 5 {
                Some(u[5])
            } else {
                let port = 3 + if self.params.xenon_enabled { 1 } else { 0 };
                u.get(port).copied()
            };

            let b_base = match dpp_in {
                Some(val) if !val.is_nan() && (val - b_curr).abs() > 1e-6 && val >= 0.0 => val,
                _ => b_curr,
            };

            let dt_days = dt / 86400.0;
            let b_n = b_base + phi * dt_days;

            let rho_b = interpolate_burnup_reactivity_pcm(
                &self.params.burnup_table_days,
                &self.params.burnup_table_reactivity_pcm,
                b_n,
            );

            (b_n, rho_b)
        } else {
            (0.0, 0.0)
        };

        let rho_fb_pcm = rho_thermal_fb_pcm + rho_xe_delta_pcm + rho_burnup_pcm;
        let rho_total_pcm = rho_ext_pcm + rho_fb_pcm;
        let rho_total = rho_total_pcm * 1e-5; // Convertir pcm a adimensional

        let total_states = 1 + m + g + if self.params.xenon_enabled { 2 } else { 0 }
            + if self.params.burnup_enabled { 1 } else { 0 };
        let mut x_next = vec![0.0; total_states];

        // 3. Actualizar Precursores con el Integrador Exponencial de Hansen
        let mut sum_lambda_c_next = 0.0;
        for i in 0..m {
            let bi = self.params.beta_i[i];
            let li = self.params.lambda_i[i];
            let c_curr = x[1 + i];

            let exp_term = (-li * dt).exp();
            let c_next = c_curr * exp_term + (bi / lambda_prompt) * p_curr * ((1.0 - exp_term) / li);
            x_next[1 + i] = c_next;
            sum_lambda_c_next += li * c_next;
        }

        // 4. Actualizar Potencia Pronta P_n
        let denom = 1.0 - dt * (rho_total - beta) / lambda_prompt;
        let p_next = if denom.abs() > 1e-12 {
            ((p_curr + dt * (sum_lambda_c_next + s)) / denom).max(0.0)
        } else {
            p_curr
        };
        x_next[0] = p_next;

        // 5. Actualizar Grupos Gamma de Decay Heat
        if reset_gamma {
            match self.params.gamma_reset_mode.as_str() {
                "clean" | "zero" => {
                    for k in 0..g {
                        x_next[1 + m + k] = 0.0;
                    }
                }
                "current_power" => {
                    for k in 0..g {
                        let fk = self.params.f_gamma[k];
                        let lk = self.params.lambda_gamma[k];
                        x_next[1 + m + k] = if lk > 0.0 { (fk / lk) * p_curr } else { 0.0 };
                    }
                }
                _ => {
                    // "nominal_power" (por defecto)
                    let p0 = self.params.nominal_power_kw;
                    for k in 0..g {
                        let fk = self.params.f_gamma[k];
                        let lk = self.params.lambda_gamma[k];
                        x_next[1 + m + k] = if lk > 0.0 { (fk / lk) * p0 } else { 0.0 };
                    }
                }
            }
        } else {
            for k in 0..g {
                let fk = self.params.f_gamma[k];
                let lk = self.params.lambda_gamma[k];
                let h_curr = x[1 + m + k];

                let exp_term = (-lk * dt).exp();
                let h_next = h_curr * exp_term + fk * p_curr * ((1.0 - exp_term) / lk);
                x_next[1 + m + k] = h_next;
            }
        }

        // 6. Guardar estados de Iodo y Xenón
        if self.params.xenon_enabled {
            x_next[1 + m + g] = i_next;
            x_next[1 + m + g + 1] = x_next_val;
        }

        // 7. Guardar estado de Quemado (DPP)
        if self.params.burnup_enabled {
            let idx_b = 1 + m + g + if self.params.xenon_enabled { 2 } else { 0 };
            x_next[idx_b] = b_next;
        }

        x_next
    }

    fn outputs(&self, _t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        let p_n = x[0];
        let m = self.params.beta_i.len();
        let g = self.params.lambda_gamma.len();

        let sum_f_gamma: f64 = self.params.f_gamma.iter().sum();
        let f_prompt = (1.0 - sum_f_gamma).max(0.0);

        let mut decay_heat = 0.0;
        for k in 0..g {
            let lk = self.params.lambda_gamma[k];
            let hk = x[1 + m + k];
            decay_heat += lk * hk;
        }

        let p_thermal = f_prompt * p_n + decay_heat;

        let rho_ext_pcm = if u.is_empty() { 0.0 } else { u[0] };
        let t_fuel = if u.len() > 1 { u[1] } else { self.params.t_fuel_ref };
        let t_coolant = if u.len() > 2 { u[2] } else { self.params.t_coolant_ref };

        let delta_tf = t_fuel - self.params.t_fuel_ref;
        let delta_tc = t_coolant - self.params.t_coolant_ref;
        let rho_thermal_fb_pcm = self.params.alpha_fuel_pcm * delta_tf + self.params.alpha_coolant_pcm * delta_tc;

        let (rho_xe_pcm, rho_xe_delta_pcm, x_rel, i_rel) = if self.params.xenon_enabled {
            let idx_i = 1 + m + g;
            let idx_x = 1 + m + g + 1;
            let i_curr = x.get(idx_i).copied().unwrap_or(0.0);
            let x_curr = x.get(idx_x).copied().unwrap_or(0.0);

            let li = self.params.lambda_iodine;
            let lx_eff_eq = self.params.lambda_xenon + self.params.xenon_burnout_rate;
            let i_eq = if li > 0.0 { self.params.yield_iodine / li } else { 1.0 };
            let x_eq = if lx_eff_eq > 0.0 {
                (self.params.yield_iodine + self.params.yield_xenon) / lx_eff_eq
            } else {
                1.0
            };

            let rho_xe = if x_eq > 0.0 {
                self.params.xenon_equilibrium_reactivity_pcm * (x_curr / x_eq)
            } else {
                0.0
            };
            let rho_xe_ref = if self.params.xenon_initial_state == "clean" {
                0.0
            } else {
                self.params.xenon_equilibrium_reactivity_pcm
            };

            (
                rho_xe,
                rho_xe - rho_xe_ref,
                if x_eq > 0.0 { x_curr / x_eq } else { 0.0 },
                if i_eq > 0.0 { i_curr / i_eq } else { 0.0 },
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        let rho_burnup_pcm = if self.params.burnup_enabled {
            let idx_b = 1 + m + g + if self.params.xenon_enabled { 2 } else { 0 };
            let b_curr = x.get(idx_b).copied().unwrap_or(0.0);
            interpolate_burnup_reactivity_pcm(
                &self.params.burnup_table_days,
                &self.params.burnup_table_reactivity_pcm,
                b_curr,
            )
        } else {
            0.0
        };

        let rho_fb_pcm = rho_thermal_fb_pcm + rho_xe_delta_pcm + rho_burnup_pcm;
        let rho_total_pcm = rho_ext_pcm + rho_fb_pcm;

        if !y.is_empty() {
            y[0] = p_thermal; // Potencia térmica total [kW]
        }
        if y.len() > 1 {
            y[1] = p_n; // Potencia neutrónica pronta [kW]
        }
        if y.len() > 2 {
            y[2] = rho_total_pcm; // Reactividad total [pcm]
        }
        if y.len() > 3 {
            y[3] = rho_fb_pcm; // Reactividad de realimentaciones [pcm]
        }
        if self.params.xenon_enabled {
            if y.len() > 4 {
                y[4] = rho_xe_pcm; // Reactividad por Xenón actual [pcm]
            }
            if y.len() > 5 {
                y[5] = x_rel; // Concentración relativa de Xenón [-]
            }
            if y.len() > 6 {
                y[6] = i_rel; // Concentración relativa de Iodo [-]
            }
            if self.params.burnup_enabled {
                let idx_b = 1 + m + g + 2;
                let b_curr = x.get(idx_b).copied().unwrap_or(0.0);
                if y.len() > 7 {
                    y[7] = rho_burnup_pcm; // Reactividad por quemado [pcm]
                }
                if y.len() > 8 {
                    y[8] = b_curr; // Días de quemado acumulados DPP [días]
                }
                if y.len() > 9 {
                    y[9] = decay_heat; // Potencia de decaimiento gamma [kW]
                }
            } else if y.len() > 7 {
                y[7] = decay_heat; // Potencia de decaimiento gamma [kW]
            }
        } else if self.params.burnup_enabled {
            let idx_b = 1 + m + g;
            let b_curr = x.get(idx_b).copied().unwrap_or(0.0);
            if y.len() > 4 {
                y[4] = rho_burnup_pcm; // Reactividad por quemado [pcm]
            }
            if y.len() > 5 {
                y[5] = b_curr; // Días de quemado acumulados DPP [días]
            }
            if y.len() > 6 {
                y[6] = decay_heat; // Potencia de decaimiento gamma [kW]
            }
        } else if y.len() > 4 {
            y[4] = decay_heat; // Potencia de decaimiento gamma [kW]
        }
    }
}
