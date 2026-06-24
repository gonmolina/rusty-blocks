use crate::thnet::network::Network;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Selectores de variables físicas de la red para registrar en el CSV.
#[derive(Debug, Clone)]
pub enum RecordSelector {
    /// Presión del nodo [Pa]
    NodePressure(usize),
    /// Temperatura del nodo [K]
    NodeTemperature(usize),
    /// Entalpía específica del nodo [J/kg]
    NodeEnthalpy(usize),
    /// Densidad en el nodo [kg/m³]
    NodeDensity(usize),
    /// Caudal másico por la tubería [kg/s]
    PipeFlow(usize),
    /// Temperatura del fluido en la celda `cell_idx` de la tubería [K]
    PipeCellTemperature(usize, usize),
    /// Temperatura de la pared en la celda `cell_idx` de la tubería [K]
    PipeWallTemperature(usize, usize),
    /// Nivel de líquido del tanque abierto [m]
    OpenTankLevel(usize),
    /// Nivel de líquido del tanque cerrado [m]
    ClosedTankLevel(usize),
    /// Presión del tanque cerrado [Pa]
    ClosedTankPressure(usize),
    /// Nivel de líquido del tanque estratificado [m]
    StratifiedTankLevel(usize),
    /// Temperatura de la capa `layer_idx` del tanque estratificado [K]
    StratifiedTankLayerTemp(usize, usize),
    /// Estado de apertura de la PRV (1.0 = abierta, 0.0 = cerrada)
    PrvIsOpen(usize),
}

impl RecordSelector {
    /// Retorna una cadena de texto descriptiva para usar como encabezado de columna en el CSV.
    pub fn header(&self) -> String {
        match self {
            RecordSelector::NodePressure(id) => format!("node_{}_pressure_pa", id),
            RecordSelector::NodeTemperature(id) => format!("node_{}_temp_k", id),
            RecordSelector::NodeEnthalpy(id) => format!("node_{}_enthalpy_j_kg", id),
            RecordSelector::NodeDensity(id) => format!("node_{}_density_kg_m3", id),
            RecordSelector::PipeFlow(id) => format!("pipe_{}_flow_kg_s", id),
            RecordSelector::PipeCellTemperature(id, cell) => format!("pipe_{}_cell_{}_temp_k", id, cell),
            RecordSelector::PipeWallTemperature(id, cell) => format!("pipe_{}_cell_{}_wall_temp_k", id, cell),
            RecordSelector::OpenTankLevel(id) => format!("open_tank_{}_level_m", id),
            RecordSelector::ClosedTankLevel(id) => format!("closed_tank_{}_level_m", id),
            RecordSelector::ClosedTankPressure(id) => format!("closed_tank_{}_pressure_pa", id),
            RecordSelector::StratifiedTankLevel(id) => format!("stratified_tank_{}_level_m", id),
            RecordSelector::StratifiedTankLayerTemp(id, layer) => format!("strat_tank_{}_layer_{}_temp_k", id, layer),
            RecordSelector::PrvIsOpen(id) => format!("prv_{}_is_open", id),
        }
    }

    /// Evalúa el selector sobre la red en el instante actual, retornando su valor físico.
    pub fn evaluate(&self, net: &Network) -> f64 {
        match self {
            RecordSelector::NodePressure(id) => {
                if *id < net.nodes.len() {
                    net.nodes[*id].pressure
                } else {
                    0.0
                }
            }
            RecordSelector::NodeTemperature(id) => {
                if *id < net.nodes.len() {
                    net.nodes[*id].temperature
                } else {
                    0.0
                }
            }
            RecordSelector::NodeEnthalpy(id) => {
                if *id < net.nodes.len() {
                    net.nodes[*id].h
                } else {
                    0.0
                }
            }
            RecordSelector::NodeDensity(id) => {
                if *id < net.nodes.len() {
                    net.nodes[*id].density()
                } else {
                    0.0
                }
            }
            RecordSelector::PipeFlow(id) => {
                if *id < net.pipes.len() {
                    net.pipes[*id].flow
                } else {
                    0.0
                }
            }
            RecordSelector::PipeCellTemperature(id, cell) => {
                if *id < net.pipes.len() {
                    let pipe = &net.pipes[*id];
                    if *cell < pipe.cell_temp.len() {
                        pipe.cell_temp[*cell]
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
            RecordSelector::PipeWallTemperature(id, cell) => {
                if *id < net.pipes.len() {
                    let pipe = &net.pipes[*id];
                    if *cell < pipe.wall_temp.len() {
                        pipe.wall_temp[*cell]
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
            RecordSelector::OpenTankLevel(id) => {
                if *id < net.open_tanks.len() {
                    net.open_tanks[*id].level
                } else {
                    0.0
                }
            }
            RecordSelector::ClosedTankLevel(id) => {
                if *id < net.closed_tanks.len() {
                    net.closed_tanks[*id].level
                } else {
                    0.0
                }
            }
            RecordSelector::ClosedTankPressure(id) => {
                if *id < net.closed_tanks.len() {
                    let tank = &net.closed_tanks[*id];
                    if tank.node_id < net.nodes.len() {
                        let rho = net.nodes[tank.node_id].density();
                        tank.calculate_pressure(rho)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
            RecordSelector::StratifiedTankLevel(id) => {
                if *id < net.stratified_tanks.len() {
                    net.stratified_tanks[*id].level()
                } else {
                    0.0
                }
            }
            RecordSelector::StratifiedTankLayerTemp(id, layer) => {
                if *id < net.stratified_tanks.len() {
                    let temps = net.stratified_tanks[*id].layer_temp();
                    if *layer < temps.len() {
                        temps[*layer]
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
            RecordSelector::PrvIsOpen(id) => {
                if *id < net.pipes.len() {
                    if let crate::thnet::network::BranchComponent::Prv(ref prv) = net.pipes[*id].component {
                        if prv.is_open { 1.0 } else { 0.0 }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            }
        }
    }
}

/// Registrador para escribir variables seleccionadas de la simulación a un archivo CSV.
pub struct CsvRecorder {
    writer: csv::Writer<BufWriter<File>>,
    selectors: Vec<RecordSelector>,
}

impl CsvRecorder {
    /// Crea un nuevo CsvRecorder. Crea el archivo y escribe automáticamente la fila de encabezados.
    pub fn new<P: AsRef<Path>>(path: P, selectors: Vec<RecordSelector>) -> Result<Self, csv::Error> {
        // Asegurarse de que el directorio padre exista
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path)?;
        let buf_writer = BufWriter::new(file);
        let mut writer = csv::Writer::from_writer(buf_writer);

        // Escribir encabezados: time_s + selectores
        let mut headers = vec!["time_s".to_string()];
        for sel in &selectors {
            headers.push(sel.header());
        }
        writer.write_record(&headers)?;

        Ok(CsvRecorder { writer, selectors })
    }

    /// Evalúa la red en el instante actual y escribe una fila de resultados en el CSV.
    pub fn record(&mut self, time: f64, net: &Network) -> Result<(), csv::Error> {
        let mut row = vec![format!("{:.6}", time)];
        for sel in &self.selectors {
            row.push(format!("{:.6}", sel.evaluate(net)));
        }
        self.writer.write_record(&row)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), csv::Error> {
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thnet::network::{Node, Pipe};

    #[test]
    fn test_csv_recorder_flow() {
        let t_init = 293.15;
        let mut net = Network::new();
        let id_0 = net.add_node(Node::new(t_init, 2e5, 0.1).with_fixed_pressure());
        let id_1 = net.add_node(Node::new(t_init, 1e5, 0.1));
        let pipe_id = net.add_pipe(Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init));
        net.pipes[pipe_id].flow = 1.234;

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_record.csv");

        let selectors = vec![
            RecordSelector::NodePressure(id_0),
            RecordSelector::NodePressure(id_1),
            RecordSelector::PipeFlow(pipe_id),
            RecordSelector::PipeCellTemperature(pipe_id, 2),
        ];

        let mut recorder = CsvRecorder::new(&path, selectors).unwrap();
        recorder.record(0.0, &net).unwrap();
        recorder.flush().unwrap();

        // Verificar el contenido
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("time_s,node_0_pressure_pa,node_1_pressure_pa,pipe_0_flow_kg_s,pipe_0_cell_2_temp_k"));
        assert!(lines[1].contains("0.000000,200000.000000,100000.000000,1.234000,293.150000"));

        // Limpiar
        let _ = std::fs::remove_file(&path);
    }
}

