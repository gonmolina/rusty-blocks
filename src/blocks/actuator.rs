use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::blocks::{Block, BlockRegistry};

/// Modo de fallo ante pérdida de alimentación motriz / aire comprimido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailMode {
    /// Falla cerrada (cierre automático por resorte).
    FailClose,
    /// Falla abierta (apertura automática por resorte).
    FailOpen,
    /// Falla en posición actual (se traba donde está).
    FailAsIs,
}

/// Modos de falla inyectables en un actuador desde la consola de instructor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActuatorFault {
    /// Operación normal.
    None,
    /// Atasco mecánico en una posición dada [0.0, 1.0].
    Jam(f64),
    /// Atasco mecánico en la posición actual.
    JamCurrent,
    /// Pérdida de energía / alimentación motriz.
    PowerLoss(FailMode),
    /// Movimiento lento por fricción o baja presión neumática (multiplicador de tiempo de carrera).
    SlowTravel(f64),
    /// Desvío o error de cero/calibración de posición.
    PositionOffset(f64),
}

impl Default for ActuatorFault {
    fn default() -> Self {
        ActuatorFault::None
    }
}

/// Bloque actuador en tiempo discreto (válvulas motorizadas/neumáticas, bombas y variadores).
///
/// Modela:
/// - Tiempo de carrera completo (*slew rate limit*)
/// - Zona muerta (*deadband*)
/// - Modos de falla en caliente (*Jam*, *PowerLoss*, *SlowTravel*)
/// - Posición real de salida lista para inyectar en THNet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorBlock {
    pub tag: String,
    pub description: String,
    pub target_signal: String,
    pub travel_time_s: f64,
    pub spring_return_time_s: f64,
    pub deadband: f64,
    pub sample_time_dt: f64,
    pub min_pos: f64,
    pub max_pos: f64,
    pub initial_position: f64,
    pub fault: ActuatorFault,

    #[serde(skip)]
    frozen_pos: std::cell::Cell<Option<f64>>,
}

impl ActuatorBlock {
    pub fn new(
        tag: impl Into<String>,
        description: impl Into<String>,
        target_signal: impl Into<String>,
        travel_time_s: f64,
        sample_time_dt: f64,
    ) -> Self {
        Self {
            tag: tag.into(),
            description: description.into(),
            target_signal: target_signal.into(),
            travel_time_s: travel_time_s.max(1e-4),
            spring_return_time_s: (travel_time_s * 0.5).max(1e-4),
            deadband: 0.005, // 0.5% default deadband
            sample_time_dt: sample_time_dt.max(1e-4),
            min_pos: 0.0,
            max_pos: 1.0,
            initial_position: 0.0,
            fault: ActuatorFault::None,
            frozen_pos: std::cell::Cell::new(None),
        }
    }

    pub fn with_initial_pos(mut self, pos: f64) -> Self {
        self.initial_position = pos.clamp(self.min_pos, self.max_pos);
        self
    }

    pub fn with_deadband(mut self, deadband: f64) -> Self {
        self.deadband = deadband.max(0.0);
        self
    }

    pub fn set_fault(&mut self, fault: ActuatorFault) {
        self.fault = fault;
    }

    pub fn build(params: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        let tag = params.get("tag")
            .and_then(|v| v.as_str())
            .unwrap_or("ACTUATOR_001")
            .to_string();
        let description = params.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Actuador de Control")
            .to_string();
        let target_signal = params.get("target_signal")
            .and_then(|v| v.as_str())
            .unwrap_or("pipe.0.valve_opening")
            .to_string();
        let travel_time = params.get("travel_time_s").and_then(|v| v.as_f64()).unwrap_or(10.0);
        let dt = params.get("sample_time_dt").and_then(|v| v.as_f64()).unwrap_or(0.01);
        let init_pos = params.get("initial_position").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let deadband = params.get("deadband").and_then(|v| v.as_f64()).unwrap_or(0.005);

        let a = Self::new(tag, description, target_signal, travel_time, dt)
            .with_initial_pos(init_pos)
            .with_deadband(deadband);
        Ok(Box::new(a))
    }
}

impl Block for ActuatorBlock {
    fn num_states(&self) -> usize {
        1 // Estado x[0]: posición física actual pos(t)
    }

    fn num_inputs(&self) -> usize {
        1 // Puerto 0: consigna demandada u_demand(t) [0.0, 1.0]
    }

    fn num_outputs(&self) -> usize {
        1 // Puerto 0: posición real física alcanzada y_pos(t) [0.0, 1.0]
    }

    fn input_width(&self, _port: usize) -> usize {
        1
    }

    fn output_width(&self, _port: usize) -> usize {
        1
    }

    fn has_direct_feedthrough(&self) -> bool {
        false // Estado dinámico con integración de carrera
    }

    fn sample_time(&self) -> Option<f64> {
        Some(self.sample_time_dt)
    }

    fn get_initial_conditions(&self, x: &mut [f64]) {
        x[0] = self.initial_position;
    }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {
        // Bloque puramente discreto
    }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let u_demand = if u.is_empty() { 0.0 } else { u[0].clamp(self.min_pos, self.max_pos) };
        let pos_curr = x[0];
        let dt = self.sample_time_dt;

        // Guardar para JamCurrent
        if self.frozen_pos.get().is_none() {
            self.frozen_pos.set(Some(pos_curr));
        }

        // Evaluar dinámica según fallas
        let pos_next = match &self.fault {
            ActuatorFault::Jam(fixed_pos) => *fixed_pos,
            ActuatorFault::JamCurrent => self.frozen_pos.get().unwrap_or(pos_curr),
            ActuatorFault::PowerLoss(mode) => match mode {
                FailMode::FailClose => {
                    let rate = dt / self.spring_return_time_s;
                    (pos_curr - rate).max(self.min_pos)
                }
                FailMode::FailOpen => {
                    let rate = dt / self.spring_return_time_s;
                    (pos_curr + rate).min(self.max_pos)
                }
                FailMode::FailAsIs => pos_curr,
            },
            ActuatorFault::SlowTravel(factor) => {
                let effective_travel = self.travel_time_s * factor.max(1.0);
                let max_delta = dt / effective_travel;
                let error = u_demand - pos_curr;
                if error.abs() < self.deadband {
                    pos_curr
                } else {
                    let delta = error.clamp(-max_delta, max_delta);
                    (pos_curr + delta).clamp(self.min_pos, self.max_pos)
                }
            }
            ActuatorFault::None | ActuatorFault::PositionOffset(_) => {
                let max_delta = dt / self.travel_time_s;
                let error = u_demand - pos_curr;
                if error.abs() < self.deadband {
                    pos_curr
                } else {
                    let delta = error.clamp(-max_delta, max_delta);
                    (pos_curr + delta).clamp(self.min_pos, self.max_pos)
                }
            }
        };

        vec![pos_next]
    }

    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) {
        let pos = x[0];
        let out_pos = match &self.fault {
            ActuatorFault::PositionOffset(offset) => (pos + offset).clamp(self.min_pos, self.max_pos),
            _ => pos,
        };
        y[0] = out_pos;
    }
}
