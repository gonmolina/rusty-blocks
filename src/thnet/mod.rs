/// THNet — Simulador de Redes Termohidráulicas
///
/// Módulo principal. Exporta los submódulos del simulador.
pub mod thermo;
pub mod network;
pub mod solver;
pub mod output;
pub mod loader;

pub use self::loader::{
    load_network, load_simulation_config, load_recorder_config, parse_signal,
    SimulationConfig, JsonRecorder,
};



