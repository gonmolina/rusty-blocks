use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::blocks::{Block, BlockRegistry};

/// Calidad de la señal del sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorQuality {
    Good,
    Bad,
    Uncertain,
    Faulted,
}

impl Default for SensorQuality {
    fn default() -> Self {
        SensorQuality::Good
    }
}

/// Modos de falla inyectables en un sensor desde la consola de instructor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorFault {
    /// Operación normal sin fallas.
    None,
    /// Clavado en un valor numérico fijo (en unidades de ingeniería).
    Stuck(f64),
    /// Congelado en el último valor medido antes de la falla.
    StuckAtCurrent,
    /// Lazo abierto / rotura de cable (corriente < 3.6 mA).
    OpenCircuit,
    /// Cortocircuito / saturación a tope (corriente > 21.5 mA).
    ShortCircuit,
    /// Ráfaga de ruido de alta varianza.
    NoiseBurst(f64),
    /// Deriva temporal continua (rampa).
    Drift {
        rate_per_sec: f64,
        max_drift: f64,
        start_time: f64,
    },
    /// Error de ganancia y sesgo/bias.
    GainBias {
        gain: f64,
        bias: f64,
    },
}

impl Default for SensorFault {
    fn default() -> Self {
        SensorFault::None
    }
}

/// Bloque sensor en tiempo discreto para instrumentación de planta.
///
/// Modela:
/// - Filtro pasabajos de 1° orden (retardo dinámico)
/// - Generador de ruido gaussiano
/// - Mapeo a unidades de ingeniería (EU) y señal eléctrica 4-20 mA
/// - Inyección de fallas en caliente
/// - Exportación de metadata y calidad de señal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorBlock {
    pub tag: String,
    pub description: String,
    pub source_signal: String,
    pub eu_unit: String,
    pub min_eu: f64,
    pub max_eu: f64,
    pub min_ma: f64,
    pub max_ma: f64,
    pub time_constant_tau: f64,
    pub sample_time_dt: f64,
    pub noise_std_dev: f64,
    pub noise_enabled: bool,
    pub fault: SensorFault,
    
    // Estado interno para PRNG (Xorshift32)
    #[serde(skip)]
    rng_state: std::cell::Cell<u32>,
    #[serde(skip)]
    frozen_val: std::cell::Cell<Option<f64>>,
}

impl SensorBlock {
    pub fn new(
        tag: impl Into<String>,
        description: impl Into<String>,
        source_signal: impl Into<String>,
        eu_unit: impl Into<String>,
        min_eu: f64,
        max_eu: f64,
        time_constant_tau: f64,
        sample_time_dt: f64,
    ) -> Self {
        Self {
            tag: tag.into(),
            description: description.into(),
            source_signal: source_signal.into(),
            eu_unit: eu_unit.into(),
            min_eu,
            max_eu,
            min_ma: 4.0,
            max_ma: 20.0,
            time_constant_tau: time_constant_tau.max(1e-6),
            sample_time_dt: sample_time_dt.max(1e-6),
            noise_std_dev: 0.0,
            noise_enabled: false,
            fault: SensorFault::None,
            rng_state: std::cell::Cell::new(123456789),
            frozen_val: std::cell::Cell::new(None),
        }
    }

    pub fn with_noise(mut self, std_dev: f64, enabled: bool) -> Self {
        self.noise_std_dev = std_dev;
        self.noise_enabled = enabled;
        self
    }

    pub fn set_fault(&mut self, fault: SensorFault) {
        self.fault = fault;
    }

    /// Generador simple de números pseudoaleatorios con distribución normal estándar (Box-Muller).
    fn next_gaussian(&self) -> f64 {
        let mut x = self.rng_state.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state.set(x);
        let u1 = ((x & 0x00FFFFFF) as f64 + 1.0) / 16777216.0;

        let mut y = x.wrapping_add(0x9E3779B9);
        y ^= y << 13;
        y ^= y >> 17;
        y ^= y << 5;
        self.rng_state.set(y);
        let u2 = ((y & 0x00FFFFFF) as f64 + 1.0) / 16777216.0;

        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Calcula la calidad actual de la señal en función del estado de fallas y límites.
    pub fn current_quality(&self, val_eu: f64) -> SensorQuality {
        match &self.fault {
            SensorFault::None => {
                if val_eu < self.min_eu || val_eu > self.max_eu {
                    SensorQuality::Uncertain
                } else {
                    SensorQuality::Good
                }
            }
            SensorFault::OpenCircuit | SensorFault::ShortCircuit => SensorQuality::Bad,
            SensorFault::NoiseBurst(_) => SensorQuality::Uncertain,
            SensorFault::Stuck(_) | SensorFault::StuckAtCurrent | SensorFault::Drift { .. } | SensorFault::GainBias { .. } => {
                SensorQuality::Faulted
            }
        }
    }

    /// Convierte valor en unidades de ingeniería (EU) a corriente eléctrica (mA).
    pub fn eu_to_ma(&self, val_eu: f64) -> f64 {
        let span_eu = self.max_eu - self.min_eu;
        if span_eu.abs() < 1e-12 {
            return self.min_ma;
        }
        let frac = (val_eu - self.min_eu) / span_eu;
        self.min_ma + frac * (self.max_ma - self.min_ma)
    }

    pub fn build(params: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        let tag = params.get("tag")
            .and_then(|v| v.as_str())
            .unwrap_or("SENSOR_001")
            .to_string();
        let description = params.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Sensor de Proceso")
            .to_string();
        let source_signal = params.get("source_signal")
            .and_then(|v| v.as_str())
            .unwrap_or("process_var")
            .to_string();
        let eu_unit = params.get("eu_unit")
            .and_then(|v| v.as_str())
            .unwrap_or("EU")
            .to_string();
        let min_eu = params.get("min_eu").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let max_eu = params.get("max_eu").and_then(|v| v.as_f64()).unwrap_or(100.0);
        let tau = params.get("time_constant_tau").and_then(|v| v.as_f64()).unwrap_or(0.1);
        let dt = params.get("sample_time_dt").and_then(|v| v.as_f64()).unwrap_or(0.01);
        let noise_std = params.get("noise_std_dev").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let noise_en = params.get("noise_enabled").and_then(|v| v.as_bool()).unwrap_or(false);

        let s = Self::new(tag, description, source_signal, eu_unit, min_eu, max_eu, tau, dt)
            .with_noise(noise_std, noise_en);
        Ok(Box::new(s))
    }
}

impl Block for SensorBlock {
    fn num_states(&self) -> usize {
        1 // Estado x[0]: valor filtrado y_filt
    }

    fn num_inputs(&self) -> usize {
        1 // Puerto 0: entrada física del proceso u(t)
    }

    fn num_outputs(&self) -> usize {
        2 // Puerto 0: Valor medido EU, Puerto 1: Corriente 4-20 mA
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
        Some(self.sample_time_dt)
    }

    fn get_initial_conditions(&self, x: &mut [f64]) {
        x[0] = self.min_eu;
    }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {
        // Bloque puramente discreto
    }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let u_raw = if u.is_empty() { 0.0 } else { u[0] };
        let dt = self.sample_time_dt;
        let tau = self.time_constant_tau;
        let alpha = dt / (tau + dt);
        
        let x_prev = x[0];
        let x_next = (1.0 - alpha) * x_prev + alpha * u_raw;
        vec![x_next]
    }

    fn outputs(&self, t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) {
        let val_filt = x[0];

        // Guardar para StuckAtCurrent
        if self.frozen_val.get().is_none() {
            self.frozen_val.set(Some(val_filt));
        }

        // 1. Aplicar Ruido
        let mut noise = 0.0;
        if self.noise_enabled && self.noise_std_dev > 0.0 {
            noise = self.next_gaussian() * self.noise_std_dev;
        }

        // 2. Evaluar Falla
        let (val_eu, val_ma) = match &self.fault {
            SensorFault::None => {
                let v = val_filt + noise;
                (v, self.eu_to_ma(v))
            }
            SensorFault::Stuck(fixed_val) => {
                (*fixed_val, self.eu_to_ma(*fixed_val))
            }
            SensorFault::StuckAtCurrent => {
                let v = self.frozen_val.get().unwrap_or(val_filt);
                (v, self.eu_to_ma(v))
            }
            SensorFault::OpenCircuit => {
                (self.min_eu - (self.max_eu - self.min_eu) * 0.1, 0.0) // 0 mA (< 3.6 mA)
            }
            SensorFault::ShortCircuit => {
                (self.max_eu + (self.max_eu - self.min_eu) * 0.1, 24.0) // 24 mA (> 21 mA)
            }
            SensorFault::NoiseBurst(mult) => {
                let v = val_filt + self.next_gaussian() * self.noise_std_dev.max(1.0) * mult;
                (v, self.eu_to_ma(v))
            }
            SensorFault::Drift { rate_per_sec, max_drift, start_time } => {
                let dt_fault = (t - start_time).max(0.0);
                let drift = (rate_per_sec * dt_fault).clamp(-max_drift.abs(), max_drift.abs());
                let v = val_filt + noise + drift;
                (v, self.eu_to_ma(v))
            }
            SensorFault::GainBias { gain, bias } => {
                let v = val_filt * gain + bias + noise;
                (v, self.eu_to_ma(v))
            }
        };

        y[0] = val_eu;
        if y.len() > 1 {
            y[1] = val_ma;
        }
    }
}
