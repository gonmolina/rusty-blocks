use serde::Deserialize;
use crate::thnet::network::{
    Network, Node, Pipe, OpenTank, ClosedTank, StratifiedTank, HeatExchanger, ValveChar
};

#[derive(Deserialize, Debug)]
pub struct JsonNode {
    pub temperature: f64,
    pub pressure: f64,
    pub volume: f64,
    #[serde(default)]
    pub fixed_pressure: bool,
    pub fixed_temperature: Option<f64>,
    #[serde(default)]
    pub external_heat: f64,
}

#[derive(Deserialize, Debug)]
pub struct JsonPipe {
    pub node_up: usize,
    pub node_dn: usize,
    pub diameter: f64,
    pub length: f64,
    pub roughness: f64,
    #[serde(default)]
    pub elevation_dz: f64,
    #[serde(default = "default_n_cells")]
    pub n_cells: usize,
    #[serde(default = "default_t_init")]
    pub t_init: f64,
    pub component: Option<JsonComponent>,
    #[serde(default)]
    pub heat_total: f64,
    pub wall: Option<JsonWall>,
    pub hx_shell: Option<JsonHxShell>,
}

fn default_n_cells() -> usize {
    1
}

fn default_t_init() -> f64 {
    293.15
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum JsonComponent {
    #[serde(rename = "pipe")]
    Pipe,
    #[serde(rename = "pump")]
    Pump {
        dp_max: Option<f64>,
        w_max: Option<f64>,
        coefs: Option<[f64; 3]>,
        #[serde(default = "default_speed_ratio")]
        speed_ratio: f64,
    },
    #[serde(rename = "valve")]
    Valve {
        cv: Option<f64>,
        kv: Option<f64>,
        opening: f64,
        #[serde(default)]
        characteristic: JsonValveChar,
    },
    #[serde(rename = "check_valve")]
    CheckValve,
    #[serde(rename = "prv")]
    Prv {
        set_pressure: f64,
        blowdown: f64,
        cv_full: f64,
    },
}

fn default_speed_ratio() -> f64 {
    1.0
}

#[derive(Deserialize, Debug, Clone, Copy, Default)]
pub enum JsonValveChar {
    #[default]
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "equal_pct")]
    EqualPct,
    #[serde(rename = "quick_opening")]
    QuickOpening,
}

impl From<JsonValveChar> for ValveChar {
    fn from(c: JsonValveChar) -> Self {
        match c {
            JsonValveChar::Linear => ValveChar::Linear,
            JsonValveChar::EqualPct => ValveChar::EqualPercentage,
            JsonValveChar::QuickOpening => ValveChar::QuickOpening,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct JsonWall {
    pub mass_kg: f64,
    pub cp_j_kg_k: f64,
    pub ua_w_k: f64,
}

#[derive(Deserialize, Debug)]
pub struct JsonHxShell {
    pub ua_w_k: f64,
    pub t_coolant_k: f64,
}

#[derive(Deserialize, Debug)]
pub struct JsonOpenTank {
    pub node_id: usize,
    pub area: f64,
    pub level: f64,
    pub level_min: f64,
    pub level_max: f64,
    #[serde(default)]
    pub z_bottom: f64,
    #[serde(default = "default_p_atm")]
    pub p_atm: f64,
}

fn default_p_atm() -> f64 {
    1e5
}

#[derive(Deserialize, Debug)]
pub struct JsonClosedTank {
    pub node_id: usize,
    pub area: f64,
    pub level: f64,
    pub level_min: f64,
    pub level_max: f64,
    #[serde(default)]
    pub z_bottom: f64,
    pub v_total: f64,
    pub p_gas_init: f64,
    #[serde(default = "default_gamma")]
    pub gamma: f64,
    pub compressibility_beta: Option<f64>,
}

fn default_gamma() -> f64 {
    1.4
}

#[derive(Deserialize, Debug)]
pub struct JsonStratifiedTank {
    pub node_id: usize,
    pub area: f64,
    pub level: f64,
    #[serde(default)]
    pub level_min: f64,
    pub level_max: f64,
    #[serde(default)]
    pub z_bottom: f64,
    #[serde(default = "default_p_atm")]
    pub p_atm: f64,
    pub n_layers: usize,
    pub t_init: f64,
    #[serde(default)]
    pub heater_height: f64,
    #[serde(default)]
    pub heater_power: f64,
    #[serde(default)]
    pub inlet_height: f64,
    #[serde(default)]
    pub inlet_flow: f64,
    pub inlet_temp: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct JsonHeatExchanger {
    pub pipe_hot: usize,
    pub pipe_cold: usize,
    pub ua: f64,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct SimulationConfig {
    pub dt: f64,
    pub t_final: f64,
    #[serde(default = "default_max_newton_iter")]
    pub max_newton_iter: usize,
    #[serde(default = "default_tol_flow")]
    pub tol_flow: f64,
}

fn default_max_newton_iter() -> usize {
    50
}

fn default_tol_flow() -> f64 {
    1e-9
}

use crate::thnet::output::RecordSelector;

#[derive(Deserialize, Debug, Clone)]
pub struct JsonRecorder {
    pub file: String,
    pub signals: Vec<String>,
    pub interval: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct JsonNetworkDefinition {
    #[serde(default)]
    pub nodes: Vec<JsonNode>,
    #[serde(default)]
    pub pipes: Vec<JsonPipe>,
    #[serde(default)]
    pub open_tanks: Vec<JsonOpenTank>,
    #[serde(default)]
    pub closed_tanks: Vec<JsonClosedTank>,
    #[serde(default)]
    pub stratified_tanks: Vec<JsonStratifiedTank>,
    #[serde(default)]
    pub heat_exchangers: Vec<JsonHeatExchanger>,
    pub simulation: Option<SimulationConfig>,
    pub recorder: Option<JsonRecorder>,
}

/// Obtiene la configuración de simulación desde el JSON si existe.
pub fn load_simulation_config(json: &str) -> Result<SimulationConfig, String> {
    let def: JsonNetworkDefinition = serde_json::from_str(json)
        .map_err(|e| format!("Error parseando JSON de simulación: {}", e))?;
    def.simulation.ok_or_else(|| "No se encontró la sección 'simulation' en el JSON".to_string())
}

/// Parsea un string descriptor de señal y retorna su RecordSelector correspondiente.
pub fn parse_signal(s: &str) -> Result<RecordSelector, String> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 3 {
        return Err(format!("Formato de señal inválido: '{}'. Debe tener al menos 3 partes (ej: node.0.pressure)", s));
    }
    let category = parts[0];
    let id: usize = parts[1].parse().map_err(|_| format!("ID de componente inválido en señal '{}'", s))?;
    let var = parts[2];

    match category {
        "node" => match var {
            "pressure" => Ok(RecordSelector::NodePressure(id)),
            "temperature" => Ok(RecordSelector::NodeTemperature(id)),
            "enthalpy" => Ok(RecordSelector::NodeEnthalpy(id)),
            "density" => Ok(RecordSelector::NodeDensity(id)),
            _ => Err(format!("Variable de nodo desconocida '{}' en señal '{}'", var, s)),
        },
        "pipe" => match var {
            "flow" => Ok(RecordSelector::PipeFlow(id)),
            "cell_temp" => {
                if parts.len() < 4 {
                    return Err(format!("Señal de temperatura de celda de caño requiere índice de celda: '{}' (ej: pipe.0.cell_temp.2)", s));
                }
                let cell_idx: usize = parts[3].parse().map_err(|_| format!("Índice de celda inválido en señal '{}'", s))?;
                Ok(RecordSelector::PipeCellTemperature(id, cell_idx))
            }
            "wall_temp" => {
                if parts.len() < 4 {
                    return Err(format!("Señal de temperatura de pared de caño requiere índice de celda: '{}' (ej: pipe.0.wall_temp.2)", s));
                }
                let cell_idx: usize = parts[3].parse().map_err(|_| format!("Índice de pared inválido en señal '{}'", s))?;
                Ok(RecordSelector::PipeWallTemperature(id, cell_idx))
            }
            _ => Err(format!("Variable de caño desconocida '{}' en señal '{}'", var, s)),
        },
        "open_tank" => match var {
            "level" => Ok(RecordSelector::OpenTankLevel(id)),
            _ => Err(format!("Variable de tanque abierto desconocida '{}' en señal '{}'", var, s)),
        },
        "closed_tank" => match var {
            "level" => Ok(RecordSelector::ClosedTankLevel(id)),
            "pressure" => Ok(RecordSelector::ClosedTankPressure(id)),
            _ => Err(format!("Variable de tanque cerrado desconocida '{}' en señal '{}'", var, s)),
        },
        "stratified_tank" => match var {
            "level" => Ok(RecordSelector::StratifiedTankLevel(id)),
            "layer_temp" => {
                if parts.len() < 4 {
                    return Err(format!("Señal de temperatura de capa de tanque estratificado requiere índice: '{}' (ej: stratified_tank.0.layer_temp.5)", s));
                }
                let layer_idx: usize = parts[3].parse().map_err(|_| format!("Índice de capa inválido en señal '{}'", s))?;
                Ok(RecordSelector::StratifiedTankLayerTemp(id, layer_idx))
            }
            _ => Err(format!("Variable de tanque estratificado desconocida '{}' en señal '{}'", var, s)),
        },
        "prv" => match var {
            "is_open" => Ok(RecordSelector::PrvIsOpen(id)),
            _ => Err(format!("Variable de PRV desconocida '{}' en señal '{}'", var, s)),
        },
        _ => Err(format!("Categoría de componente desconocida '{}' en señal '{}'", category, s)),
    }
}

/// Obtiene la configuración del registrador desde el JSON si existe.
pub fn load_recorder_config(json: &str) -> Result<Option<JsonRecorder>, String> {
    let def: JsonNetworkDefinition = serde_json::from_str(json)
        .map_err(|e| format!("Error parseando JSON de registrador: {}", e))?;
    Ok(def.recorder)
}



/// Construye una red completa (`Network`) a partir de un archivo JSON.
pub fn load_network(json: &str) -> Result<Network, String> {
    let def: JsonNetworkDefinition = serde_json::from_str(json)
        .map_err(|e| format!("Error parseando JSON de red: {}", e))?;

    let n_nodes = def.nodes.len();
    let n_pipes = def.pipes.len();

    // Validar referencias
    for (idx, pipe) in def.pipes.iter().enumerate() {
        if pipe.node_up >= n_nodes {
            return Err(format!(
                "Tubería {} (índice {}): node_up ({}) fuera de rango (total nodos: {})",
                idx, idx, pipe.node_up, n_nodes
            ));
        }
        if pipe.node_dn >= n_nodes {
            return Err(format!(
                "Tubería {} (índice {}): node_dn ({}) fuera de rango (total nodos: {})",
                idx, idx, pipe.node_dn, n_nodes
            ));
        }
    }

    for (idx, tank) in def.open_tanks.iter().enumerate() {
        if tank.node_id >= n_nodes {
            return Err(format!(
                "Tanque abierto {} (índice {}): node_id ({}) fuera de rango (total nodos: {})",
                idx, idx, tank.node_id, n_nodes
            ));
        }
    }

    for (idx, tank) in def.closed_tanks.iter().enumerate() {
        if tank.node_id >= n_nodes {
            return Err(format!(
                "Tanque cerrado {} (índice {}): node_id ({}) fuera de rango (total nodos: {})",
                idx, idx, tank.node_id, n_nodes
            ));
        }
    }

    for (idx, tank) in def.stratified_tanks.iter().enumerate() {
        if tank.node_id >= n_nodes {
            return Err(format!(
                "Tanque estratificado {} (índice {}): node_id ({}) fuera de rango (total nodos: {})",
                idx, idx, tank.node_id, n_nodes
            ));
        }
    }

    for (idx, hx) in def.heat_exchangers.iter().enumerate() {
        if hx.pipe_hot >= n_pipes {
            return Err(format!(
                "Intercambiador {} (índice {}): pipe_hot ({}) fuera de rango (total tuberías: {})",
                idx, idx, hx.pipe_hot, n_pipes
            ));
        }
        if hx.pipe_cold >= n_pipes {
            return Err(format!(
                "Intercambiador {} (índice {}): pipe_cold ({}) fuera de rango (total tuberías: {})",
                idx, idx, hx.pipe_cold, n_pipes
            ));
        }
    }

    let mut net = Network::new();

    // 1. Agregar nodos
    for json_node in def.nodes {
        let mut node = Node::new(
            json_node.temperature,
            json_node.pressure,
            json_node.volume,
        );
        if json_node.fixed_pressure {
            node = node.with_fixed_pressure();
        }
        if let Some(t_fixed) = json_node.fixed_temperature {
            node = node.with_fixed_temperature(t_fixed);
        }
        if json_node.external_heat != 0.0 {
            node = node.with_external_heat(json_node.external_heat);
        }
        net.add_node(node);
    }

    // 2. Agregar tuberías
    for json_pipe in def.pipes {
        let mut pipe = Pipe::new(
            json_pipe.node_up,
            json_pipe.node_dn,
            json_pipe.diameter,
            json_pipe.length,
            json_pipe.roughness,
            json_pipe.elevation_dz,
            json_pipe.n_cells,
            json_pipe.t_init,
        );

        if let Some(comp) = &json_pipe.component {
            match comp {
                JsonComponent::Pipe => {}
                JsonComponent::Pump { dp_max, w_max, coefs, speed_ratio } => {
                    if let Some(c) = coefs {
                        pipe = pipe.with_pump_curve(c[0], c[1], c[2]);
                    } else {
                        let dp = dp_max.unwrap_or(0.0);
                        let w = w_max.unwrap_or(0.0);
                        pipe = pipe.with_pump(dp, w);
                    }
                    pipe = pipe.with_pump_speed(*speed_ratio);
                }
                JsonComponent::Valve { cv, kv, opening, characteristic } => {
                    if let Some(cv_val) = cv {
                        pipe = pipe.with_valve(*cv_val, *opening);
                    } else if let Some(kv_val) = kv {
                        pipe = pipe.with_valve_kv(*kv_val, *opening);
                    } else {
                        pipe = pipe.with_valve(0.0, *opening);
                    }
                    let char_enum: ValveChar = (*characteristic).into();
                    pipe = pipe.with_valve_char(char_enum);
                }
                JsonComponent::CheckValve => {
                    pipe = pipe.with_check_valve();
                }
                JsonComponent::Prv { set_pressure, blowdown, cv_full } => {
                    pipe = pipe.with_prv(*set_pressure, *blowdown, *cv_full);
                }
            }
        }

        if json_pipe.heat_total != 0.0 {
            pipe = pipe.with_heat(json_pipe.heat_total);
        }

        if let Some(wall) = &json_pipe.wall {
            pipe = pipe.with_wall(wall.mass_kg, wall.cp_j_kg_k, wall.ua_w_k);
        }

        if let Some(hx) = &json_pipe.hx_shell {
            pipe = pipe.with_heat_exchanger(hx.ua_w_k, hx.t_coolant_k);
        }

        net.add_pipe(pipe);
    }

    // 3. Agregar tanques abiertos
    for json_open_tank in def.open_tanks {
        let tank = OpenTank::new(
            json_open_tank.node_id,
            json_open_tank.area,
            json_open_tank.level,
            json_open_tank.level_min,
            json_open_tank.level_max,
            json_open_tank.z_bottom,
            json_open_tank.p_atm,
        );
        net.add_open_tank(tank);
    }

    // 4. Agregar tanques cerrados
    for json_closed_tank in def.closed_tanks {
        let mut tank = ClosedTank::new(
            json_closed_tank.node_id,
            json_closed_tank.area,
            json_closed_tank.level,
            json_closed_tank.level_min,
            json_closed_tank.level_max,
            json_closed_tank.z_bottom,
            json_closed_tank.v_total,
            json_closed_tank.p_gas_init,
            json_closed_tank.gamma,
        );
        if let Some(beta) = json_closed_tank.compressibility_beta {
            tank = tank.with_liquid_compressibility(beta);
        }
        net.add_closed_tank(tank);
    }

    // 5. Agregar tanques estratificados
    for json_stratified_tank in def.stratified_tanks {
        let mut tank = StratifiedTank::new(
            json_stratified_tank.node_id,
            json_stratified_tank.area,
            json_stratified_tank.level,
            json_stratified_tank.level_min,
            json_stratified_tank.level_max,
            json_stratified_tank.z_bottom,
            json_stratified_tank.p_atm,
            json_stratified_tank.n_layers,
            json_stratified_tank.t_init,
            json_stratified_tank.heater_height,
            json_stratified_tank.heater_power,
            json_stratified_tank.inlet_height,
        );
        if json_stratified_tank.inlet_flow != 0.0 {
            tank.set_inlet_flow(json_stratified_tank.inlet_flow);
        }
        if let Some(t_in) = json_stratified_tank.inlet_temp {
            tank.set_inlet_temp(t_in);
        } else {
            tank.set_inlet_temp(json_stratified_tank.t_init);
        }
        net.add_stratified_tank(tank);
    }

    // 6. Agregar intercambiadores de calor
    for json_hx in def.heat_exchangers {
        let hx = HeatExchanger::new(
            json_hx.pipe_hot,
            json_hx.pipe_cold,
            json_hx.ua,
        );
        net.add_heat_exchanger(hx);
    }

    Ok(net)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_valid_network() {
        let json = r#"{
            "nodes": [
                { "temperature": 293.15, "pressure": 1e5, "volume": 0.001, "fixed_pressure": true },
                { "temperature": 293.15, "pressure": 1e5, "volume": 0.001 },
                { "temperature": 293.15, "pressure": 1e5, "volume": 0.001, "fixed_pressure": true }
            ],
            "pipes": [
                {
                    "node_up": 1, "node_dn": 0,
                    "diameter": 0.25, "length": 5.0, "roughness": 1.5e-5,
                    "n_cells": 2, "t_init": 293.15,
                    "heat_total": 1000.0,
                    "wall": { "mass_kg": 50.0, "cp_j_kg_k": 450.0, "ua_w_k": 200.0 }
                },
                {
                    "node_up": 1, "node_dn": 2,
                    "diameter": 0.25, "length": 5.0, "roughness": 1.5e-5,
                    "component": { "type": "valve", "cv": 30.55, "opening": 0.5, "characteristic": "equal_pct" }
                }
            ],
            "open_tanks": [
                { "node_id": 0, "area": 2.5, "level": 1.2, "level_min": 0.0, "level_max": 3.0 }
            ],
            "closed_tanks": [
                { "node_id": 2, "area": 1.5, "level": 0.8, "level_min": 0.0, "level_max": 2.0, "v_total": 5.0, "p_gas_init": 2e5 }
            ],
            "stratified_tanks": [
                {
                    "node_id": 1,
                    "area": 4.9, "level": 19.0, "level_max": 20.0,
                    "n_layers": 20, "t_init": 293.15,
                    "heater_height": 2.0, "heater_power": 10000.0,
                    "inlet_height": 18.0, "inlet_flow": 10.0
                }
            ],
            "heat_exchangers": [
                { "pipe_hot": 0, "pipe_cold": 1, "ua": 4180.0 }
            ]
        }"#;

        let net = load_network(json).unwrap();
        assert_eq!(net.nodes.len(), 3);
        assert_eq!(net.pipes.len(), 2);
        assert_eq!(net.open_tanks.len(), 1);
        assert_eq!(net.closed_tanks.len(), 1);
        assert_eq!(net.stratified_tanks.len(), 1);
        assert_eq!(net.heat_exchangers.len(), 1);

        // Check node
        assert!(net.nodes[0].fixed_pressure.is_some());
        
        // Check pipe configuration
        let pipe0 = &net.pipes[0];
        assert_eq!(pipe0.n_cells(), 2);
        assert_eq!(pipe0.wall_mass(), 50.0);
        
        // Check pipe 1 valve component
        let pipe1 = &net.pipes[1];
        assert_eq!(pipe1.valve_opening, 0.5);
    }

    #[test]
    fn test_load_invalid_reference() {
        let json = r#"{
            "nodes": [
                { "temperature": 293.15, "pressure": 1e5, "volume": 0.001 }
            ],
            "pipes": [
                {
                    "node_up": 0, "node_dn": 5,
                    "diameter": 0.25, "length": 5.0, "roughness": 1.5e-5
                }
            ]
        }"#;

        let res = load_network(json);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("node_dn (5) fuera de rango"));
    }

    #[test]
    fn test_load_example_files() {
        let files = [
            "examples/thnet/dos_tanques.json",
            "examples/thnet/tanque_estratificado.json",
            "examples/thnet/bomba_valvula_cano.json",
            "examples/thnet/conv_natural.json",
        ];
        for file_path in &files {
            let content = std::fs::read_to_string(file_path)
                .unwrap_or_else(|_| panic!("Failed to read {}", file_path));
            let net = load_network(&content)
                .unwrap_or_else(|e| panic!("Failed to load network from {}: {}", file_path, e));
            assert!(net.nodes.len() > 0);
        }
    }

    #[test]
    fn test_prv_behavior() {
        let json = r#"{
            "nodes": [
                { "temperature": 293.15, "pressure": 1e5, "volume": 0.001 },
                { "temperature": 293.15, "pressure": 1e5, "volume": 0.001, "fixed_pressure": true }
            ],
            "pipes": [
                {
                    "node_up": 0, "node_dn": 1,
                    "diameter": 0.05, "length": 1.0, "roughness": 1.5e-5,
                    "component": {
                        "type": "prv",
                        "set_pressure": 2.5e5,
                        "blowdown": 5e4,
                        "cv_full": 10.0
                    }
                }
            ],
            "simulation": {
                "dt": 0.1,
                "t_final": 2.0
            }
        }"#;

        let mut net = load_network(json).unwrap();
        assert_eq!(net.nodes.len(), 2);
        assert_eq!(net.pipes.len(), 1);

        let prv_idx = 0;
        
        // Inicialmente la PRV debe estar cerrada (is_open = false)
        if let crate::thnet::network::BranchComponent::Prv(ref prv) = net.pipes[prv_idx].component {
            assert!(!prv.is_open);
        } else {
            panic!("Debe ser un componente PRV");
        }

        let mut solver = crate::thnet::solver::Solver::new();

        // Paso 1: Presión en nodo_up = 1e5 Pa (menor a set_pressure = 2.5e5 Pa)
        // La PRV debe seguir cerrada
        solver.step(&mut net, 0.1);
        if let crate::thnet::network::BranchComponent::Prv(ref prv) = net.pipes[prv_idx].component {
            assert!(!prv.is_open);
        }

        // Paso 2: Forzar la presión en el nodo 0 a 3e5 Pa (mayor a set_pressure)
        net.nodes[0].pressure = 3e5;
        solver.step(&mut net, 0.1);
        
        // Ahora la PRV debe haberse abierto (is_open = true)
        if let crate::thnet::network::BranchComponent::Prv(ref prv) = net.pipes[prv_idx].component {
            assert!(prv.is_open);
        }

        // Paso 3: Bajar la presión en el nodo 0 a 2.2e5 Pa (menor a set_pressure pero mayor a set_pressure - blowdown = 2e5 Pa)
        // Debe seguir abierta por histéresis
        net.nodes[0].pressure = 2.2e5;
        solver.step(&mut net, 0.1);
        if let crate::thnet::network::BranchComponent::Prv(ref prv) = net.pipes[prv_idx].component {
            assert!(prv.is_open);
        }

        // Paso 4: Bajar la presión en el nodo 0 a 1.9e5 Pa (menor a set_pressure - blowdown)
        // La PRV debe haberse cerrado
        net.nodes[0].pressure = 1.9e5;
        solver.step(&mut net, 0.1);
        if let crate::thnet::network::BranchComponent::Prv(ref prv) = net.pipes[prv_idx].component {
            assert!(!prv.is_open);
        }
    }
}
