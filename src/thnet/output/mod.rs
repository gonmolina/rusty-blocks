/// Módulo para registrar variables de simulación en archivos de salida (CSV).

pub mod csv;

pub use self::csv::{CsvRecorder, RecordSelector};
