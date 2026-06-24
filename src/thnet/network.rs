/// Estructuras de datos de la red termohidráulica.
///
/// El grafo de la red se representa como:
/// - **Nodos**: almacenan presión, temperatura, entalpía y masa.
/// - **Tuberías** (Pipe): ramas que conectan dos nodos, transportan caudal másico.
///
/// Convención de signos:
/// - W > 0 → flujo en dirección node_up → node_dn
/// - elevation_dz = z_dn - z_up  (positivo = el nodo dn está más arriba)
/// - ΔP_grav = -ρ·g·elevation_dz  (se opone al flujo si dz > 0)

use super::thermo::{self, ThermoLib};

pub mod builder;
pub mod graph;

pub use builder::NetworkBuilder;
pub use graph::IncidenceMatrix;

/// Identificador de nodo (índice en el vector Network::nodes)
pub type NodeId = usize;

/// Identificador de tubería (índice en el vector Network::pipes)
pub type PipeId = usize;

/// Nodo de la red: almacena estado termodinámico
pub trait NetworkNode: Send + Sync {
    /// Presión actual [Pa]
    fn pressure(&self) -> f64;
    /// Temperatura actual [K]
    fn temperature(&self) -> f64;
    /// Entalpía específica actual [J/kg]
    fn enthalpy(&self) -> f64;
    /// Densidad actual del agua [kg/m³]
    fn density(&self) -> f64;
    /// ¿Es este nodo un nodo de presión fija (condición de contorno Dirichlet)?
    fn is_pressure_fixed(&self) -> bool;
    /// Valor de la presión fija si es de tipo Dirichlet [Pa]
    fn fixed_pressure(&self) -> f64;
    /// Actualizar estado térmico tras un paso de transporte
    fn update_thermal(
        &mut self,
        h_net: f64,
        w_net: f64,
        q_ext: f64,
        dt: f64,
        thermo: &dyn ThermoLib,
    );
}

/// Rama de la red: conecta dos nodos
pub trait NetworkBranch: Send + Sync {
    /// Retorna los identificadores de los nodos extremos (node_up, node_dn)
    fn nodes(&self) -> (NodeId, NodeId);
    /// Caudal másico actual de la rama [kg/s]
    fn flow_rate(&self) -> f64;
    /// Establece el caudal másico actual [kg/s]
    fn set_flow_rate(&mut self, w: f64);
    /// Conductancia hidráulica linealizada G = Δt / (I + Δt·R)
    fn conductance(&self, rho: f64, dt: f64) -> f64;
    /// Término fuente S^k (contribución independiente de P)
    fn source_term(&self, rho: f64, dt: f64) -> f64;
    /// Entalpía que transporta la rama (upwind) [W] (flujo de energía)
    fn enthalpy_flux(&self, h_up: f64, h_dn: f64) -> f64;
}

// ─────────────────────────────────────────────────────────────────────────────
// NODO
// ─────────────────────────────────────────────────────────────────────────────

/// Estado de un nodo de la red.
///
/// Un nodo puede ser:
/// - **Libre**: presión calculada por el solvedor (caso general).
/// - **Dirichlet**: presión fija (`fixed_pressure = Some(p)`). Se usa como
///   referencia de presión (punto de expansión, pileta abierta, etc.).
///
/// **Nota sobre temperatura en nodos Dirichlet**: incluso si la presión está
/// fijada, la temperatura sigue evolucionando con el balance térmico.
#[derive(Debug, Clone)]
pub struct Node {
    /// Temperatura actual [K]
    pub temperature: f64,
    /// Presión actual [Pa]
    pub pressure: f64,
    /// Volumen del nodo (header, pileta, etc.) [m³]
    /// Determina la masa térmica: M = ρ(T)·V
    pub volume: f64,
    /// Calor externo inyectado directamente al nodo [W] (positivo = calor entra)
    pub external_heat: f64,
    /// Presión fija (condición de Dirichlet). `None` = nodo libre.
    pub fixed_pressure: Option<f64>,
    /// Temperatura fija (condición de Dirichlet térmica). `None` = temperatura libre.
    pub fixed_temperature: Option<f64>,
    /// Entalpía específica actual [J/kg]
    pub h: f64,
}

impl Node {
    /// Crea un nodo con temperatura y presión iniciales y un volumen dado.
    pub fn new(temperature_k: f64, pressure_pa: f64, volume_m3: f64) -> Self {
        Node {
            temperature: temperature_k,
            pressure: pressure_pa,
            volume: volume_m3,
            external_heat: 0.0,
            fixed_pressure: None,
            fixed_temperature: None,
            h: thermo::enthalpy(temperature_k),
        }
    }

    /// Convierte el nodo en nodo de presión fija (Dirichlet) a la presión actual.
    pub fn with_fixed_pressure(mut self) -> Self {
        self.fixed_pressure = Some(self.pressure);
        self
    }

    /// Convierte el nodo en nodo de temperatura fija (Dirichlet térmica) a la temperatura dada (en K).
    pub fn with_fixed_temperature(mut self, temp_k: f64) -> Self {
        self.fixed_temperature = Some(temp_k);
        self.temperature = temp_k;
        self.h = thermo::enthalpy(temp_k);
        self
    }

    /// Agrega calor externo al nodo [W].
    pub fn with_external_heat(mut self, q_watts: f64) -> Self {
        self.external_heat = q_watts;
        self
    }

    /// Densidad actual del agua en este nodo [kg/m³]
    pub fn density(&self) -> f64 {
        thermo::density(self.temperature)
    }

    /// Masa de agua en el volumen del nodo [kg]
    pub fn mass(&self) -> f64 {
        self.density() * self.volume
    }

    /// ¿Es este nodo un nodo de presión fija?
    pub fn is_fixed(&self) -> bool {
        self.fixed_pressure.is_some()
    }
}

impl NetworkNode for Node {
    fn pressure(&self) -> f64 {
        self.pressure
    }

    fn temperature(&self) -> f64 {
        self.temperature
    }

    fn enthalpy(&self) -> f64 {
        self.h
    }

    fn density(&self) -> f64 {
        self.density()
    }

    fn is_pressure_fixed(&self) -> bool {
        self.is_fixed()
    }

    fn fixed_pressure(&self) -> f64 {
        self.fixed_pressure.unwrap_or(0.0)
    }

    fn update_thermal(
        &mut self,
        h_net: f64,
        w_net: f64,
        q_ext: f64,
        dt: f64,
        thermo: &dyn ThermoLib,
    ) {
        if let Some(t_fixed) = self.fixed_temperature {
            self.temperature = t_fixed;
            self.h = thermo.enthalpy(t_fixed);
            return;
        }

        let m = self.mass().max(1e-10);
        let w_abs = w_net.abs();
        let denom = m + dt * w_abs;

        let h_new = if w_net >= 0.0 {
            (m * self.h + dt * (h_net + q_ext)) / denom
        } else {
            let env_heat = w_abs * thermo.enthalpy(self.temperature);
            (m * self.h + dt * (h_net + env_heat + q_ext)) / denom
        };

        self.h = h_new.clamp(
            thermo.enthalpy(super::thermo::T_REF_K + 1.0),
            thermo.enthalpy(super::thermo::T_REF_K + 110.0),
        );
        self.temperature = thermo.temperature_from_enthalpy(self.h);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TUBERÍA
// ─────────────────────────────────────────────────────────────────────────────

/// Tubería circular que conecta dos nodos de la red.
///
/// Física implementada:
/// - Inercia hidráulica: I = ρ·L/A
/// - Fricción de Darcy-Weisbach con factor f de Churchill (1977) — no requiere
///   iteración y es válido para todo Re (laminar y turbulento)
/// - Efecto gravitacional: ΔP_grav = -ρ·g·(z_dn - z_up)
/// - Transporte de entalpía: esquema upwind implícito en N celdas 1D
#[derive(Debug, Clone)]
pub enum BranchComponent {
    Pipe(PipeComponent),
    Pump(PumpComponent),
    Valve(ValveComponent),
    CheckValve(PipeComponent),
    Prv(PrvComponent),
}

#[derive(Debug, Clone)]
pub struct PipeComponent {
    pub diameter: f64,
    pub length: f64,
    pub roughness: f64,
    pub elevation_dz: f64,
    pub n_cells: usize,
    pub wall_mass: f64,
    pub wall_cp: f64,
    pub wall_ua: f64,
    pub ua_hx: f64,
    pub t_coolant: f64,
    pub heat_total: f64,
}

#[derive(Debug, Clone)]
pub struct PumpComponent {
    pub pump_dp_max: f64,
    pub pump_w_max: f64,
    pub pump_coefs: Option<[f64; 3]>,
}

#[derive(Debug, Clone)]
pub struct ValveComponent {
    pub valve_cv: f64,
    pub valve_char: ValveChar,
}

#[derive(Debug, Clone)]
pub struct PrvComponent {
    pub set_pressure: f64,
    pub blowdown: f64,
    pub cv_full: f64,
    pub is_open: bool,
}

#[derive(Debug, Clone)]
pub struct Pipe {
    /// Nodo de donde parte el flujo en la dirección positiva
    pub node_up: NodeId,
    /// Nodo al que llega el flujo en la dirección positiva
    pub node_dn: NodeId,
    /// Caudal másico actual [kg/s]
    /// Positivo = flujo de node_up hacia node_dn
    pub flow: f64,
    /// Perfil de temperatura en cada celda [K]
    pub cell_temp: Vec<f64>,
    /// Perfil de temperatura de la pared de la tubería [K]
    pub wall_temp: Vec<f64>,
    /// Apertura de la válvula (0.0 a 1.0)
    pub valve_opening: f64,
    /// Relación de velocidad de la bomba (omega / omega_nom)
    pub pump_speed_ratio: f64,
    /// Indica si esta tubería actúa como válvula de retención (check valve)
    pub is_check_valve: bool,
    /// Estado actual de la válvula de retención: true si está cerrada, false si está abierta
    pub check_valve_closed: bool,
    /// Componente específico de esta rama
    pub component: BranchComponent,
    /// Calor neto recibido desde un intercambiador de calor en este paso [W]
    pub q_hx_external: f64,
}

/// Características de comportamiento de la apertura de una válvula de control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValveChar {
    /// Característica lineal: f(x) = x
    #[default]
    Linear,
    /// Característica de porcentaje igual (Equal Percentage): f(x) = r^(x-1) con r = 50
    EqualPercentage,
    /// Característica de apertura rápida (Quick Opening): f(x) = sqrt(x)
    QuickOpening,
}

impl ValveChar {
    /// Evalúa la función de apertura de la válvula f(x) para un valor x en [0, 1].
    pub fn evaluate(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        match self {
            ValveChar::Linear => x,
            ValveChar::EqualPercentage => {
                if x <= 0.0 {
                    0.0
                } else {
                    const R: f64 = 50.0;
                    R.powf(x - 1.0)
                }
            }
            ValveChar::QuickOpening => x.sqrt(),
        }
    }
}

impl Pipe {
    /// Constructor principal.
    pub fn new(
        node_up: NodeId,
        node_dn: NodeId,
        diameter: f64,
        length: f64,
        roughness: f64,
        elevation_dz: f64,
        n_cells: usize,
        t_init_k: f64,
    ) -> Self {
        let n = n_cells.max(1);
        Pipe {
            node_up,
            node_dn,
            flow: 0.0,
            cell_temp: vec![t_init_k; n],
            wall_temp: Vec::new(),
            valve_opening: 1.0,
            pump_speed_ratio: 1.0,
            is_check_valve: false,
            check_valve_closed: false,
            q_hx_external: 0.0,
            component: BranchComponent::Pipe(PipeComponent {
                diameter,
                length,
                roughness,
                elevation_dz,
                n_cells: n,
                wall_mass: 0.0,
                wall_cp: 0.0,
                wall_ua: 0.0,
                ua_hx: 0.0,
                t_coolant: 293.15, // 20 °C por defecto
                heat_total: 0.0,
            }),
        }
    }

    /// Fuente de calor constante distribuida a lo largo de la tubería [W].
    /// Positivo = calienta el fluido (p. ej.: resistencia eléctrica).
    pub fn with_heat(mut self, q_total_watts: f64) -> Self {
        if let BranchComponent::Pipe(ref mut p) | BranchComponent::CheckValve(ref mut p) = self.component {
            p.heat_total = q_total_watts;
        }
        self
    }

    /// Intercambiador de calor: extrae (o aporta) calor en función de la
    /// diferencia entre la temperatura del fluido y la del refrigerante.
    pub fn with_heat_exchanger(mut self, ua_total_w_per_k: f64, t_coolant_k: f64) -> Self {
        if let BranchComponent::Pipe(ref mut p) | BranchComponent::CheckValve(ref mut p) = self.component {
            p.ua_hx = ua_total_w_per_k;
            p.t_coolant = t_coolant_k;
        }
        self
    }

    /// Configura la inercia térmica de la pared de la tubería y su acoplamiento con el fluido.
    pub fn with_wall(mut self, wall_mass_kg: f64, wall_cp_j_kg_k: f64, wall_ua_w_k: f64) -> Self {
        self.wall_temp = vec![self.cell_temp[0]; self.cell_temp.len()];
        if let BranchComponent::Pipe(ref mut p) | BranchComponent::CheckValve(ref mut p) = self.component {
            p.wall_mass = wall_mass_kg;
            p.wall_cp = wall_cp_j_kg_k;
            p.wall_ua = wall_ua_w_k;
        }
        self
    }

    /// Configura una bomba centrífuga en esta línea de tubería (curva cuadrática).
    pub fn with_pump(mut self, dp_max_pa: f64, w_max_kg_s: f64) -> Self {
        self.component = BranchComponent::Pump(PumpComponent {
            pump_dp_max: dp_max_pa,
            pump_w_max: w_max_kg_s,
            pump_coefs: None,
        });
        self
    }

    /// Configura una bomba centrífuga con coeficientes polinomiales específicos [a0, a1, a2]
    /// para dP_nom = a0 + a1*W + a2*W^2 [Pa].
    pub fn with_pump_curve(mut self, a0: f64, a1: f64, a2: f64) -> Self {
        self.component = BranchComponent::Pump(PumpComponent {
            pump_dp_max: 0.0,
            pump_w_max: 0.0,
            pump_coefs: Some([a0, a1, a2]),
        });
        self
    }

    /// Configura la relación de velocidad de la bomba (omega / omega_nom).
    pub fn with_pump_speed(mut self, ratio: f64) -> Self {
        self.pump_speed_ratio = ratio;
        self
    }

    /// Configura una válvula de control en esta línea de tubería utilizando el coeficiente Cv.
    pub fn with_valve(mut self, cv: f64, opening: f64) -> Self {
        self.valve_opening = opening;
        self.component = BranchComponent::Valve(ValveComponent {
            valve_cv: cv,
            valve_char: ValveChar::Linear,
        });
        self
    }

    /// Configura una válvula de control en esta línea de tubería utilizando el coeficiente europeo Kv.
    /// Convierte internamente el coeficiente a Cv (Cv = 1.156099 * Kv).
    pub fn with_valve_kv(mut self, kv: f64, opening: f64) -> Self {
        self.valve_opening = opening;
        self.component = BranchComponent::Valve(ValveComponent {
            valve_cv: kv * 1.156_099,
            valve_char: ValveChar::Linear,
        });
        self
    }

    /// Configura la curva característica de la válvula de control.
    pub fn with_valve_char(mut self, char: ValveChar) -> Self {
        if let BranchComponent::Valve(ref mut v) = self.component {
            v.valve_char = char;
        }
        self
    }

    /// Configura la tubería como una válvula de retención (check valve).
    pub fn with_check_valve(mut self) -> Self {
        self.is_check_valve = true;
        self.check_valve_closed = false;
        if let BranchComponent::Pipe(p) = self.component {
            self.component = BranchComponent::CheckValve(p);
        } else {
            self.component = BranchComponent::CheckValve(PipeComponent {
                diameter: 0.05,
                length: 1.0,
                roughness: 1.5e-5,
                elevation_dz: 0.0,
                n_cells: 1,
                wall_mass: 0.0,
                wall_cp: 0.0,
                wall_ua: 0.0,
                ua_hx: 0.0,
                t_coolant: 293.15,
                heat_total: 0.0,
            });
        }
        self
    }

    /// Configura la tubería como una válvula de alivio de presión (PRV).
    pub fn with_prv(mut self, set_pressure: f64, blowdown: f64, cv_full: f64) -> Self {
        self.component = BranchComponent::Prv(PrvComponent {
            set_pressure,
            blowdown,
            cv_full,
            is_open: false,
        });
        self
    }

    // ── Getters de Configuración (para Solver) ────────────────────────────────

    pub fn diameter(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.diameter,
            _ => 0.0,
        }
    }

    pub fn length(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.length,
            _ => 0.0,
        }
    }

    pub fn elevation_dz(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.elevation_dz,
            _ => 0.0,
        }
    }

    pub fn n_cells(&self) -> usize {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.n_cells,
            _ => 0,
        }
    }

    pub fn wall_mass(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.wall_mass,
            _ => 0.0,
        }
    }

    pub fn wall_cp(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.wall_cp,
            _ => 0.0,
        }
    }

    pub fn wall_ua(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.wall_ua,
            _ => 0.0,
        }
    }

    pub fn ua_hx(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.ua_hx,
            _ => 0.0,
        }
    }

    pub fn t_coolant(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.t_coolant,
            _ => 0.0,
        }
    }

    pub fn heat_total(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.heat_total,
            _ => 0.0,
        }
    }

    // ── Geometría ──────────────────────────────────────────────────────────

    /// Sección transversal interior [m²]
    pub fn area(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => std::f64::consts::PI * (p.diameter * 0.5).powi(2),
            _ => 0.0,
        }
    }

    /// Volumen interior de la tubería [m³]
    pub fn volume(&self) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => self.area() * p.length,
            _ => 0.0,
        }
    }

    // ── Propiedades medias ─────────────────────────────────────────────────

    /// Temperatura media de las celdas [K]
    pub fn mean_temperature(&self) -> f64 {
        if self.cell_temp.is_empty() {
            return thermo::T_REF_K + 20.0;
        }
        self.cell_temp.iter().copied().sum::<f64>() / self.cell_temp.len() as f64
    }

    /// Densidad media del fluido en la tubería [kg/m³]
    pub fn mean_density(&self) -> f64 {
        thermo::density(self.mean_temperature())
    }

    // ── Hidráulica ─────────────────────────────────────────────────────────

    /// Inercia hidráulica de la tubería [1/m]
    pub fn hydraulic_inertia(&self, _rho: f64) -> f64 {
        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => p.length / self.area(),
            _ => 0.0,
        }
    }

    /// Factor de fricción de Darcy-Weisbach mediante la fórmula de Churchill (1977).
    pub fn friction_factor(&self, re: f64) -> f64 {
        let re = re.max(1.0);

        // Régimen laminar (Hagen-Poiseuille)
        if re < 2300.0 {
            return (64.0 / re).max(0.008);
        }

        match &self.component {
            BranchComponent::Pipe(p) | BranchComponent::CheckValve(p) => {
                // Fórmula universal de Churchill (1977)
                let eps_d = p.roughness / p.diameter;
                let a = {
                    let arg = (7.0 / re).powf(0.9) + 0.27 * eps_d;
                    let arg = arg.max(1e-15);
                    (-2.457 * arg.ln()).powi(16)
                };
                let b = (37530.0_f64 / re).powi(16);
                8.0 * ((8.0 / re).powi(12) + (a + b).powf(-1.5)).powf(1.0 / 12.0)
            }
            _ => 0.0,
        }
    }

    // ── Bomba Centrífuga Curve & Speed ─────────────────────────────────────

    /// Retorna la ganancia de presión de la bomba [Pa] evaluada a la velocidad y caudal actuales.
    pub fn pump_pressure_gain(&self) -> f64 {
        let w = self.flow;
        let s = self.pump_speed_ratio;
        
        match &self.component {
            BranchComponent::Pump(p) => {
                if let Some(coefs) = p.pump_coefs {
                    coefs[0] * s * s + coefs[1] * s * w + coefs[2] * w.abs() * w
                } else if p.pump_dp_max > 0.0 && p.pump_w_max > 0.0 {
                    let r_pump = p.pump_dp_max / (p.pump_w_max * p.pump_w_max);
                    p.pump_dp_max * s * s - r_pump * w.abs() * w
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Retorna la derivada de la ganancia de presión de la bomba respecto al caudal [Pa·s/kg].
    pub fn pump_pressure_gain_derivative(&self) -> f64 {
        let w = self.flow;
        let s = self.pump_speed_ratio;
        
        match &self.component {
            BranchComponent::Pump(p) => {
                if let Some(coefs) = p.pump_coefs {
                    coefs[1] * s + 2.0 * coefs[2] * w.abs()
                } else if p.pump_dp_max > 0.0 && p.pump_w_max > 0.0 {
                    let r_pump = p.pump_dp_max / (p.pump_w_max * p.pump_w_max);
                    -2.0 * r_pump * w.abs()
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Retorna la fuente de presión efectiva de la bomba para el método MNA.
    pub fn pump_pressure_src(&self) -> f64 {
        let r_pump_eff = -self.pump_pressure_gain_derivative();
        let r_pump_clamped = r_pump_eff.max(0.0);
        self.pump_pressure_gain() + r_pump_clamped * self.flow
    }

    /// Resistencia hidráulica linealizada [Pa·s/kg].
    ///
    /// R_lin = K · |W| / ρ
    /// donde K = f·L/(D·2·A²) es el coeficiente de fricción cuadrático.
    ///
    /// Se usa para construir la matriz de conductancia en cada iteración Newton.
    /// Se regulariza con un caudal mínimo para evitar R_lin = 0 cuando W ≈ 0.
    pub fn resistance_linearized(&self) -> f64 {
        let rho = self.mean_density().max(1.0);
        let mu = thermo::viscosity(self.mean_temperature());
        let a = self.area();
        // Número de Reynolds (evita división por cero)
        let re = if mu > 1e-12 && a > 1e-12 {
            self.flow.abs() * self.diameter() / (a * mu)
        } else {
            0.0
        };
        let f = self.friction_factor(re);
        // Coeficiente cuadrático K [Pa·s²/kg²]: ΔP_fric = K·W·|W|/ρ
        let k = if self.diameter() > 1e-12 {
            f * self.length() / (self.diameter() * 2.0 * a * a)
        } else {
            0.0
        };
        // Regularización: usa caudal mínimo para que G no sea infinita en W=0
        // El caudal mínimo equivale a Re ≈ 100 (flujo muy lento, laminar)
        let w_reg = if self.diameter() > 1e-12 {
            self.flow.abs().max(mu * a * 100.0 / self.diameter())
        } else {
            self.flow.abs()
        };
        let mut r_total = k * w_reg / rho;

        // Resistencia de la bomba (derivada de la curva H-Q para el jacobiano MNA)
        let r_pump_eff = -self.pump_pressure_gain_derivative();
        let r_pump_clamped = r_pump_eff.max(0.0);
        r_total += r_pump_clamped;

        // Resistencia cuadrática de la válvula de control basada en Cv:
        // W = Cv_eff * 2.4026e-5 * sqrt(rho * dP) => dP = W^2 / (rho * Cv_eff^2)
        if let BranchComponent::Valve(ref v) = self.component {
            if v.valve_cv > 0.0 {
                let f_val = v.valve_char.evaluate(self.valve_opening).clamp(1e-6, 1.0);
                let cv_eff_si = v.valve_cv * 2.4026e-5 * f_val;
                let r_valve = 1.0 / (rho * cv_eff_si * cv_eff_si);
                let w_reg_valve = self.flow.abs().max(1e-4);
                r_total += r_valve * w_reg_valve;
            }
        }

        if let BranchComponent::Prv(ref prv) = self.component {
            if prv.is_open {
                if prv.cv_full > 0.0 {
                    let cv_eff_si = prv.cv_full * 2.4026e-5;
                    let r_valve = 1.0 / (rho * cv_eff_si * cv_eff_si);
                    let w_reg_valve = self.flow.abs().max(1e-4);
                    r_total += r_valve * w_reg_valve;
                }
            } else {
                r_total += 1e12;
            }
        }

        if self.is_check_valve && self.check_valve_closed {
            r_total += 1e12;
        }

        r_total
    }

    /// Caída de presión gravitacional [Pa]: ΔP_grav = -ρ·g·(z_dn - z_up)
    ///
    /// Positivo = la gravedad ayuda al flujo (caída cuesta abajo).
    /// Negativo = la gravedad se opone al flujo (subida cuesta arriba).
    pub fn gravity_pressure_drop(&self) -> f64 {
        const G: f64 = 9.806_65;
        -self.mean_density() * G * self.elevation_dz()
    }
}

impl NetworkBranch for Pipe {
    fn nodes(&self) -> (NodeId, NodeId) {
        (self.node_up, self.node_dn)
    }

    fn flow_rate(&self) -> f64 {
        self.flow
    }

    fn set_flow_rate(&mut self, w: f64) {
        self.flow = w;
    }

    fn conductance(&self, rho: f64, dt: f64) -> f64 {
        let inertia = self.hydraulic_inertia(rho);
        let r_lin = self.resistance_linearized();
        dt / (inertia + dt * r_lin).max(1e-30)
    }

    fn source_term(&self, rho: f64, dt: f64) -> f64 {
        let inertia = self.hydraulic_inertia(rho);
        let r_lin = self.resistance_linearized();
        let dp_grav = self.gravity_pressure_drop();
        let dp_pump = self.pump_pressure_src();
        let denom = (inertia + dt * r_lin).max(1e-30);
        (inertia * self.flow + dt * (dp_grav + dp_pump)) / denom
    }

    fn enthalpy_flux(&self, h_up: f64, h_dn: f64) -> f64 {
        let w = self.flow;
        if w >= 0.0 {
            w * h_up
        } else {
            w * h_dn
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TANQUE ABIERTO (OPENTANK)
// ─────────────────────────────────────────────────────────────────────────────

/// Tanque abierto a la atmósfera con nivel dinámico variable.
#[derive(Debug, Clone)]
pub struct OpenTank {
    /// Identificador del nodo de la red asociado al fondo del tanque
    pub node_id: NodeId,
    /// Área de la sección transversal del tanque [m²]
    pub area: f64,
    /// Nivel de líquido actual [m]
    pub level: f64,
    /// Nivel de líquido mínimo [m]
    pub level_min: f64,
    /// Nivel de líquido máximo [m]
    pub level_max: f64,
    /// Cota del fondo del tanque [m] (cota absoluta de referencia)
    pub z_bottom: f64,
    /// Presión en el espacio de gas sobre el líquido (típicamente atmosférica) [Pa]
    pub p_atm: f64,
}

impl OpenTank {
    /// Crea un nuevo tanque abierto asociado a un nodo.
    pub fn new(
        node_id: NodeId,
        area: f64,
        level: f64,
        level_min: f64,
        level_max: f64,
        z_bottom: f64,
        p_atm: f64,
    ) -> Self {
        OpenTank {
            node_id,
            area,
            level,
            level_min,
            level_max,
            z_bottom,
            p_atm,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TANQUE CERRADO (CLOSEDTANK)
// ─────────────────────────────────────────────────────────────────────────────

/// Tanque cerrado con colchón de gas (gas cushion) o compresibilidad directa de líquido.
#[derive(Debug, Clone)]
pub struct ClosedTank {
    /// Identificador del nodo de la red asociado al fondo del tanque
    pub node_id: NodeId,
    /// Área de la sección transversal del tanque [m²]
    pub area: f64,
    /// Nivel de líquido actual [m]
    pub level: f64,
    /// Nivel de líquido inicial [m]
    pub level_init: f64,
    /// Nivel de líquido mínimo [m]
    pub level_min: f64,
    /// Nivel de líquido máximo [m]
    pub level_max: f64,
    /// Cota del fondo del tanque [m]
    pub z_bottom: f64,
    /// Volumen total del tanque [m³]
    pub v_total: f64,
    /// Presión inicial de referencia del gas cushion [Pa]
    pub p_gas_init: f64,
    /// Volumen inicial de referencia del gas cushion [m³]
    pub v_gas_init: f64,
    /// Exponente adiabático/isotérmico del gas (gamma, e.g., 1.4 para aire, 1.0 para isotérmico)
    pub gamma: f64,
    /// Si es Some(beta), se usa compresibilidad directa del líquido en lugar de gas cushion [1/Pa]
    pub compressibility_beta: Option<f64>,
}

impl ClosedTank {
    /// Crea un nuevo tanque cerrado con colchón de gas.
    pub fn new(
        node_id: NodeId,
        area: f64,
        level: f64,
        level_min: f64,
        level_max: f64,
        z_bottom: f64,
        v_total: f64,
        p_gas_init: f64,
        gamma: f64,
    ) -> Self {
        let v_gas_init = (v_total - level * area).max(1e-6);
        ClosedTank {
            node_id,
            area,
            level,
            level_init: level,
            level_min,
            level_max,
            z_bottom,
            v_total,
            p_gas_init,
            v_gas_init,
            gamma,
            compressibility_beta: None,
        }
    }

    /// Configura el tanque para usar compresibilidad directa del líquido en lugar de colchón de gas.
    pub fn with_liquid_compressibility(mut self, beta: f64) -> Self {
        self.compressibility_beta = Some(beta);
        self
    }

    /// Calcula la presión en el nodo del fondo del tanque para una densidad del líquido dada.
    pub fn calculate_pressure(&self, rho: f64) -> f64 {
        const G: f64 = 9.806_65;
        let p_gas = if let Some(beta) = self.compressibility_beta {
            let delta_h = self.level - self.level_init;
            let delta_p = (delta_h * self.area) / (beta * self.v_total);
            self.p_gas_init + delta_p
        } else {
            let v_gas = (self.v_total - self.level * self.area).max(1e-6);
            self.p_gas_init * (self.v_gas_init / v_gas).powf(self.gamma)
        };
        p_gas + rho * G * self.level
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// INTERCAMBIADOR DE CALOR (HEATEXCHANGER)
// ─────────────────────────────────────────────────────────────────────────────

/// Intercambiador de calor de dos fluidos (modelo NTU-ε contracorriente).
#[derive(Debug, Clone)]
pub struct HeatExchanger {
    /// Identificador de la tubería del lado caliente
    pub pipe_hot: PipeId,
    /// Identificador de la tubería del lado frío
    pub pipe_cold: PipeId,
    /// Coeficiente de transferencia global de calor [W/K]
    pub ua: f64,
}

impl HeatExchanger {
    /// Crea un nuevo intercambiador de calor NTU-ε.
    pub fn new(pipe_hot: PipeId, pipe_cold: PipeId, ua: f64) -> Self {
        HeatExchanger {
            pipe_hot,
            pipe_cold,
            ua,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TANQUE ESTRATIFICADO (STRATIFIEDTANK)
// ─────────────────────────────────────────────────────────────────────────────

/// Tanque vertical con estratificación térmica 1D en múltiples capas.
#[derive(Debug, Clone)]
pub struct StratifiedTank {
    /// Identificador del nodo de la red asociado al fondo del tanque (salida de la base)
    node_id: NodeId,
    /// Área de la sección transversal del tanque [m²]
    area: f64,
    /// Nivel de líquido actual [m]
    level: f64,
    /// Nivel de líquido mínimo [m]
    level_min: f64,
    /// Nivel de líquido máximo [m]
    level_max: f64,
    /// Cota del fondo del tanque [m]
    z_bottom: f64,
    /// Presión en el espacio de gas sobre el líquido [Pa]
    p_atm: f64,
    /// Número de capas en la discretización térmica
    n_layers: usize,
    /// Perfil de temperaturas de las capas [K]
    layer_temp: Vec<f64>,
    /// Altura del calefactor localizado [m]
    heater_height: f64,
    /// Potencia del calefactor localizado [W]
    heater_power: f64,
    /// Altura del nozzle de entrada (inyección de fluido) [m]
    inlet_height: f64,
    /// Caudal dinámico de inyección externo [kg/s]
    inlet_flow: f64,
    /// Temperatura del fluido inyectado dinámico [K]
    inlet_temp: f64,
}

impl StratifiedTank {
    /// Crea un nuevo tanque estratificado vertical con temperaturas iniciales uniformes.
    pub fn new(
        node_id: NodeId,
        area: f64,
        level: f64,
        level_min: f64,
        level_max: f64,
        z_bottom: f64,
        p_atm: f64,
        n_layers: usize,
        t_init: f64,
        heater_height: f64,
        heater_power: f64,
        inlet_height: f64,
    ) -> Self {
        StratifiedTank {
            node_id,
            area,
            level,
            level_min,
            level_max,
            z_bottom,
            p_atm,
            n_layers,
            layer_temp: vec![t_init; n_layers],
            heater_height,
            heater_power,
            inlet_height,
            inlet_flow: 0.0,
            inlet_temp: t_init,
        }
    }

    /// Calcula la presión en el nodo del fondo del tanque para el estado actual.
    pub fn calculate_pressure(&self) -> f64 {
        const G: f64 = 9.806_65;
        let rho_bottom = thermo::density(self.layer_temp[0]);
        self.p_atm + rho_bottom * G * self.level
    }

    /// Actualiza el nivel y la estratificación térmica en el tanque tras el paso temporal.
    pub fn update_levels_and_temp(&mut self, w_net: f64, dt: f64) {
        let rho_bottom = thermo::density(self.layer_temp[0]).max(1.0);
        let w_out_total = -w_net; // Flujo neto saliente desde el fondo (si es positivo, sale)

        // 1. Integrar el nivel del tanque
        let level_prev = self.level;
        let dm = (self.inlet_flow - w_out_total) * dt;
        let current_mass = self.level * self.area * rho_bottom;
        let new_mass = (current_mass + dm).max(0.0);
        self.level = new_mass / (self.area * rho_bottom);
        self.level = self.level.clamp(self.level_min, self.level_max);

        // 2. Actualizar la estratificación mediante balance 1D advectivo
        let dz = self.level_max / self.n_layers as f64;
        let k_top = ((self.level / dz).floor() as usize).min(self.n_layers - 1);
        let k_heater = ((self.heater_height / dz).ceil() as usize).saturating_sub(1).min(self.n_layers - 1);
        let k_inlet = ((self.inlet_height / dz).ceil() as usize).saturating_sub(1).min(self.n_layers - 1);

        // Flujo vertical descendente
        let mut w_down = vec![0.0; self.n_layers];
        for j in 0..self.n_layers {
            if j > k_inlet {
                w_down[j] = (w_out_total - self.inlet_flow).max(0.0);
            } else {
                w_down[j] = w_out_total;
            }
        }

        // Masas en paso k y k+1
        let mut m_old = vec![0.0; self.n_layers];
        let mut m_new = vec![0.0; self.n_layers];
        for j in 0..self.n_layers {
            let z_j = j as f64 * dz;
            if level_prev > z_j {
                let h_wet = (level_prev - z_j).min(dz);
                m_old[j] = h_wet * self.area * thermo::density(self.layer_temp[j]);
            }
            if self.level > z_j {
                let h_wet = (self.level - z_j).min(dz);
                m_new[j] = h_wet * self.area * thermo::density(self.layer_temp[j]);
            }
        }

        // Balance entálpico implícito
        let tank_h: Vec<f64> = self.layer_temp.iter().map(|&temp| thermo::enthalpy(temp)).collect();
        let mut tank_h_new = tank_h.clone();

        for j in (0..=k_top).rev() {
            let m_o = m_old[j];
            let w_in_from_top = if j == k_top { 0.0 } else { w_down[j + 1] };
            let h_in_from_top = if j == k_top { 0.0 } else { tank_h_new[j + 1] };

            // Calor del calefactor
            let mut q_src = 0.0;
            if j == k_heater && self.level > self.heater_height {
                q_src += self.heater_power;
            } else if j == k_top && self.level <= self.heater_height && self.level > 0.001 {
                q_src += self.heater_power;
            }

            // Inyección
            let mut w_src = 0.0;
            let mut h_src = 0.0;
            if self.level >= self.inlet_height && j == k_inlet {
                w_src = self.inlet_flow;
                h_src = thermo::enthalpy(self.inlet_temp);
            } else if self.level < self.inlet_height && j == k_top {
                w_src = self.inlet_flow;
                h_src = thermo::enthalpy(self.inlet_temp);
            }

            let denom = m_o + dt * (w_in_from_top + w_src);
            if denom > 1e-10 {
                let num = m_o * tank_h[j] + dt * (w_in_from_top * h_in_from_top + q_src + w_src * h_src);
                tank_h_new[j] = num / denom;
            } else {
                tank_h_new[j] = tank_h[j];
            }
        }

        for j in 0..=k_top {
            self.layer_temp[j] = thermo::temperature_from_enthalpy(tank_h_new[j]);
        }
        // Capas secas vuelven a la temperatura inicial/ambiente
        for j in (k_top + 1)..self.n_layers {
            self.layer_temp[j] = self.inlet_temp;
        }
    }

    // --- GETTERS ---

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn area(&self) -> f64 {
        self.area
    }

    pub fn level(&self) -> f64 {
        self.level
    }

    pub fn level_min(&self) -> f64 {
        self.level_min
    }

    pub fn level_max(&self) -> f64 {
        self.level_max
    }

    pub fn z_bottom(&self) -> f64 {
        self.z_bottom
    }

    pub fn p_atm(&self) -> f64 {
        self.p_atm
    }

    pub fn n_layers(&self) -> usize {
        self.n_layers
    }

    pub fn layer_temp(&self) -> &[f64] {
        &self.layer_temp
    }

    pub fn heater_height(&self) -> f64 {
        self.heater_height
    }

    pub fn heater_power(&self) -> f64 {
        self.heater_power
    }

    pub fn inlet_height(&self) -> f64 {
        self.inlet_height
    }

    pub fn inlet_flow(&self) -> f64 {
        self.inlet_flow
    }

    pub fn inlet_temp(&self) -> f64 {
        self.inlet_temp
    }

    // --- SETTERS / MODIFIERS ---

    pub fn set_inlet_flow(&mut self, flow: f64) {
        self.inlet_flow = flow;
    }

    pub fn set_inlet_temp(&mut self, temp_k: f64) {
        self.inlet_temp = temp_k;
    }

    pub fn set_heater_power(&mut self, power: f64) {
        self.heater_power = power;
    }

    pub fn set_level(&mut self, level: f64) {
        self.level = level.clamp(self.level_min, self.level_max);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RED
// ─────────────────────────────────────────────────────────────────────────────

/// Red termohidráulica: colección de nodos y tuberías.
#[derive(Debug)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub pipes: Vec<Pipe>,
    pub open_tanks: Vec<OpenTank>,
    pub closed_tanks: Vec<ClosedTank>,
    pub stratified_tanks: Vec<StratifiedTank>,
    pub heat_exchangers: Vec<HeatExchanger>,
}

impl Network {
    pub fn new() -> Self {
        Network {
            nodes: Vec::new(),
            pipes: Vec::new(),
            open_tanks: Vec::new(),
            closed_tanks: Vec::new(),
            stratified_tanks: Vec::new(),
            heat_exchangers: Vec::new(),
        }
    }

    /// Agrega un nodo y retorna su NodeId.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /// Agrega una tubería y retorna su PipeId.
    pub fn add_pipe(&mut self, pipe: Pipe) -> PipeId {
        let id = self.pipes.len();
        self.pipes.push(pipe);
        id
    }

    /// Agrega un tanque abierto a la red.
    pub fn add_open_tank(&mut self, tank: OpenTank) -> usize {
        let node = &mut self.nodes[tank.node_id];
        let rho = node.density();
        const G: f64 = 9.806_65;
        let p = tank.p_atm + rho * G * tank.level;
        node.pressure = p;
        node.fixed_pressure = Some(p);

        let id = self.open_tanks.len();
        self.open_tanks.push(tank);
        id
    }

    /// Agrega un tanque cerrado a la red.
    pub fn add_closed_tank(&mut self, tank: ClosedTank) -> usize {
        let node = &mut self.nodes[tank.node_id];
        let rho = node.density();
        let p = tank.calculate_pressure(rho);
        node.pressure = p;
        node.fixed_pressure = Some(p);

        let id = self.closed_tanks.len();
        self.closed_tanks.push(tank);
        id
    }

    /// Agrega un tanque estratificado a la red.
    pub fn add_stratified_tank(&mut self, tank: StratifiedTank) -> usize {
        let node = &mut self.nodes[tank.node_id()];
        let p = tank.calculate_pressure();
        node.pressure = p;
        node.fixed_pressure = Some(p);

        // Fijar la temperatura del nodo a la temperatura de la base (capa 0)
        let t_bottom = tank.layer_temp()[0];
        node.temperature = t_bottom;
        node.h = thermo::enthalpy(t_bottom);

        let id = self.stratified_tanks.len();
        self.stratified_tanks.push(tank);
        id
    }

    /// Agrega un intercambiador de calor a la red.
    pub fn add_heat_exchanger(&mut self, hx: HeatExchanger) -> usize {
        let id = self.heat_exchangers.len();
        self.heat_exchangers.push(hx);
        id
    }

    pub fn n_nodes(&self) -> usize {
        self.nodes.len()
    }
    pub fn n_pipes(&self) -> usize {
        self.pipes.len()
    }
}
impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_builder_and_incidence_matrix() {
        let t_init = 293.15;
        let net = NetworkBuilder::new()
            .add_node(Node::new(t_init, 1e5, 0.1))
            .add_node(Node::new(t_init, 1e5, 0.1))
            .add_pipe(Pipe::new(0, 1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init))
            .build();

        assert_eq!(net.n_nodes(), 2);
        assert_eq!(net.n_pipes(), 1);

        let mat = IncidenceMatrix::build(net.n_nodes(), &net.pipes);
        assert_eq!(mat.data.len(), 2);
        assert_eq!(mat.data[0].len(), 1);
        assert_eq!(mat.data[0][0], -1.0); // node 0 is upstream (leaves node 0)
        assert_eq!(mat.data[1][0], 1.0);  // node 1 is downstream (enters node 1)
    }
}


