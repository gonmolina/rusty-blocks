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

use super::thermo;

/// Identificador de nodo (índice en el vector Network::nodes)
pub type NodeId = usize;

/// Identificador de tubería (índice en el vector Network::pipes)
pub type PipeId = usize;

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
pub struct Pipe {
    /// Nodo de donde parte el flujo en la dirección positiva
    pub node_up: NodeId,
    /// Nodo al que llega el flujo en la dirección positiva
    pub node_dn: NodeId,
    /// Diámetro interior [m]
    pub diameter: f64,
    /// Longitud del tramo [m]
    pub length: f64,
    /// Rugosidad absoluta [m] (ε para Colebrook-White / Churchill)
    /// Acero inoxidable 304: ε ≈ 1.5e-5 m
    /// Acero comercial: ε ≈ 4.6e-5 m
    pub roughness: f64,
    /// Diferencia de cota: z_dn - z_up [m]
    /// Positivo = el nodo dn está más arriba (tubería sube hacia dn)
    pub elevation_dz: f64,
    /// Número de celdas 1D para el transporte térmico a lo largo de la tubería
    pub n_cells: usize,
    /// Calor total inyectado uniformemente a lo largo de la tubería [W]
    /// Positivo = calor entra al fluido (calentamiento).
    /// Solo se usa cuando `ua_hx == 0.0` (modo fuente de calor constante).
    pub heat_total: f64,
    /// Coeficiente global de transferencia de calor del intercambiador [W/K]
    /// Cuando > 0 activa el modo HX: Q_cell = (UA/N)·(T_coolant − T_cell)
    /// Se usa en vez de `heat_total`.
    pub ua_hx: f64,
    /// Temperatura del refrigerante del intercambiador [K]
    /// Solo relevante cuando `ua_hx > 0`.
    pub t_coolant: f64,
    /// Caudal másico actual [kg/s]
    /// Positivo = flujo de node_up hacia node_dn
    pub flow: f64,
    /// Perfil de temperatura en cada celda [K]
    pub cell_temp: Vec<f64>,
    /// Masa de la pared de la tubería [kg]
    pub wall_mass: f64,
    /// Calor específico de la pared [J/(kg·K)]
    pub wall_cp: f64,
    /// Coeficiente de transferencia global pared-fluido [W/K]
    pub wall_ua: f64,
    /// Perfil de temperatura de la pared de la tubería [K]
    pub wall_temp: Vec<f64>,
    /// Presión máxima de la bomba a caudal cero [Pa]
    pub pump_dp_max: f64,
    /// Caudal de referencia/máximo de la bomba [kg/s]
    pub pump_w_max: f64,
    /// Coeficiente Cv de la válvula de control
    pub valve_cv: f64,
    /// Apertura de la válvula (0.0 a 1.0)
    pub valve_opening: f64,
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
            diameter,
            length,
            roughness,
            elevation_dz,
            n_cells: n,
            heat_total: 0.0,
            ua_hx: 0.0,
            t_coolant: 293.15, // 20 °C por defecto
            flow: 0.0,
            cell_temp: vec![t_init_k; n],
            wall_mass: 0.0,
            wall_cp: 0.0,
            wall_ua: 0.0,
            wall_temp: Vec::new(),
            pump_dp_max: 0.0,
            pump_w_max: 0.0,
            valve_cv: 0.0,
            valve_opening: 1.0,
        }
    }

    /// Fuente de calor constante distribuida a lo largo de la tubería [W].
    /// Positivo = calienta el fluido (p. ej.: resistencia eléctrica).
    pub fn with_heat(mut self, q_total_watts: f64) -> Self {
        self.heat_total = q_total_watts;
        self
    }

    /// Intercambiador de calor: extrae (o aporta) calor en función de la
    /// diferencia entre la temperatura del fluido y la del refrigerante.
    ///
    /// # Parámetros
    /// - `ua_total_w_per_k`: Coeficiente global UA [W/K] del intercambiador.
    ///   Elegir UA ≈ Q_objetivo / ΔT_log_esperado (ver MATH_SOLVER.md §4.8).
    /// - `t_coolant_k`: Temperatura del refrigerante [K].
    ///
    /// # Física implícita (estabilidad incondicional)
    ///
    /// El calor por celda se calcula de forma semi-implícita:
    /// ```text
    /// h_new = (h_old + CFL·h_in + NTU·h_cool) / (1 + CFL + NTU)
    /// NTU   = (UA/N)·Δt / (M_cell·cp)
    /// ```
    /// Esto garantiza estabilidad para cualquier NTU y cualquier Δt.
    pub fn with_heat_exchanger(mut self, ua_total_w_per_k: f64, t_coolant_k: f64) -> Self {
        self.ua_hx = ua_total_w_per_k;
        self.t_coolant = t_coolant_k;
        self
    }

    /// Configura la inercia térmica de la pared de la tubería y su acoplamiento con el fluido.
    ///
    /// # Parámetros
    /// - `wall_mass_kg`: Masa de la pared metálica de la tubería [kg].
    /// - `wall_cp_j_kg_k`: Calor específico del metal de la pared [J/(kg·K)].
    /// - `wall_ua_w_k`: Coeficiente global de transferencia de calor pared-fluido [W/K].
    pub fn with_wall(mut self, wall_mass_kg: f64, wall_cp_j_kg_k: f64, wall_ua_w_k: f64) -> Self {
        self.wall_mass = wall_mass_kg;
        self.wall_cp = wall_cp_j_kg_k;
        self.wall_ua = wall_ua_w_k;
        self.wall_temp = vec![self.cell_temp[0]; self.n_cells];
        self
    }

    /// Configura una bomba centrífuga en esta línea de tubería (curva cuadrática).
    pub fn with_pump(mut self, dp_max_pa: f64, w_max_kg_s: f64) -> Self {
        self.pump_dp_max = dp_max_pa;
        self.pump_w_max = w_max_kg_s;
        self
    }

    /// Configura una válvula de control en esta línea de tubería.
    pub fn with_valve(mut self, cv: f64, opening: f64) -> Self {
        self.valve_cv = cv;
        self.valve_opening = opening;
        self
    }

    // ── Geometría ──────────────────────────────────────────────────────────

    /// Sección transversal interior [m²]
    pub fn area(&self) -> f64 {
        std::f64::consts::PI * (self.diameter * 0.5).powi(2)
    }

    /// Volumen interior de la tubería [m³]
    pub fn volume(&self) -> f64 {
        self.area() * self.length
    }

    // ── Propiedades medias ─────────────────────────────────────────────────

    /// Temperatura media de las celdas [K]
    pub fn mean_temperature(&self) -> f64 {
        if self.cell_temp.is_empty() {
            return thermo::T_REF_K + 20.0;
        }
        self.cell_temp.iter().copied().sum::<f64>() / self.n_cells as f64
    }

    /// Densidad media del fluido en la tubería [kg/m³]
    pub fn mean_density(&self) -> f64 {
        thermo::density(self.mean_temperature())
    }

    // ── Hidráulica ─────────────────────────────────────────────────────────

    /// Inercia hidráulica de la tubería [1/m]
    ///
    /// I = L/A. Representa la resistencia del fluido a cambiar su caudal másico.
    pub fn hydraulic_inertia(&self, _rho: f64) -> f64 {
        self.length / self.area()
    }

    /// Factor de fricción de Darcy-Weisbach mediante la fórmula de Churchill (1977).
    ///
    /// Válido para todo Re (laminar Re<2300 y turbulento Re>4000).
    /// No requiere iteración a diferencia de Colebrook-White.
    pub fn friction_factor(&self, re: f64) -> f64 {
        let re = re.max(1.0);

        // Régimen laminar (Hagen-Poiseuille)
        if re < 2300.0 {
            return (64.0 / re).max(0.008);
        }

        // Fórmula universal de Churchill (1977)
        let eps_d = self.roughness / self.diameter;
        let a = {
            let arg = (7.0 / re).powf(0.9) + 0.27 * eps_d;
            // arg debe ser positivo para ln()
            let arg = arg.max(1e-15);
            (-2.457 * arg.ln()).powi(16)
        };
        let b = (37530.0_f64 / re).powi(16);

        8.0 * ((8.0 / re).powi(12) + (a + b).powf(-1.5)).powf(1.0 / 12.0)
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
            self.flow.abs() * self.diameter / (a * mu)
        } else {
            0.0
        };
        let f = self.friction_factor(re);
        // Coeficiente cuadrático K [Pa·s²/kg²]: ΔP_fric = K·W·|W|/ρ
        let k = f * self.length / (self.diameter * 2.0 * a * a);
        // Regularización: usa caudal mínimo para que G no sea infinita en W=0
        // El caudal mínimo equivale a Re ≈ 100 (flujo muy lento, laminar)
        let w_reg = self.flow.abs().max(mu * a * 100.0 / self.diameter);
        let mut r_total = k * w_reg / rho;

        // Resistencia cuadrática de la curva H-Q de la bomba:
        // H(Q) = H0 - A*Q^2 => dP = dP_max - R_pump * W^2
        // R_pump = dP_max / W_max^2
        if self.pump_dp_max > 0.0 && self.pump_w_max > 0.0 {
            let r_pump = self.pump_dp_max / (self.pump_w_max * self.pump_w_max);
            let w_reg_pump = self.flow.abs().max(1e-4);
            r_total += r_pump * w_reg_pump;
        }

        // Resistencia cuadrática de la válvula de control basada en Cv:
        // W = Cv_eff * 2.4026e-5 * sqrt(rho * dP) => dP = W^2 / (rho * Cv_eff^2)
        if self.valve_cv > 0.0 {
            let opening = self.valve_opening.clamp(1e-6, 1.0);
            let cv_eff_si = self.valve_cv * 2.4026e-5 * opening;
            let r_valve = 1.0 / (rho * cv_eff_si * cv_eff_si);
            let w_reg_valve = self.flow.abs().max(1e-4);
            r_total += r_valve * w_reg_valve;
        }

        r_total
    }

    /// Caída de presión gravitacional [Pa]: ΔP_grav = -ρ·g·(z_dn - z_up)
    ///
    /// Positivo = la gravedad ayuda al flujo (caída cuesta abajo).
    /// Negativo = la gravedad se opone al flujo (subida cuesta arriba).
    pub fn gravity_pressure_drop(&self) -> f64 {
        const G: f64 = 9.806_65;
        -self.mean_density() * G * self.elevation_dz
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
}

impl Network {
    pub fn new() -> Self {
        Network {
            nodes: Vec::new(),
            pipes: Vec::new(),
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
