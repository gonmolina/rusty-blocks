use super::{Block, BlockRegistry};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Write, BufWriter};

/// A sink block that records signals into a file.
pub struct FileSink {
    filename: String,
    interval: f64,
    width: usize,
    writer: RefCell<Option<BufWriter<File>>>,
    // Store as Option to handle the first point (t=0) correctly
    last_save_t: RefCell<Option<f64>>,
}

impl FileSink {
    pub fn new(filename: &str, interval: f64, width: usize) -> Self {
        Self {
            filename: filename.to_string(),
            interval,
            width,
            writer: RefCell::new(None),
            last_save_t: RefCell::new(None),
        }
    }

    pub fn build(v: Value, _registry: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params { filename: String, interval: f64, width: usize }
        let p: Params = serde_json::from_value(v).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(&p.filename, p.interval, p.width)))
    }

    fn ensure_file_is_open(&self) {
        if self.writer.borrow().is_none() {
            let file = File::create(&self.filename).expect("Could not create sink file");
            let mut writer = BufWriter::new(file);
            write!(writer, "t").unwrap();
            for i in 0..self.width {
                write!(writer, ",val_{}", i).unwrap();
            }
            writeln!(writer).unwrap();
            *self.writer.borrow_mut() = Some(writer);
        }
    }
}

impl Block for FileSink {
    fn num_states(&self) -> usize { 0 }
    fn num_inputs(&self) -> usize { 1 }
    fn num_outputs(&self) -> usize { 0 }
    fn input_width(&self, _port: usize) -> usize { self.width }
    fn output_width(&self, _port: usize) -> usize { 0 }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _dx: &mut [f64]) {}
    fn outputs(&self, _t: f64, _x: &[f64], _u: &[&[f64]], _y: &mut [&mut [f64]]) {}

    fn next_event(&self, t: f64) -> Option<f64> {
        if self.interval <= 0.0 { return None; }
        
        match *self.last_save_t.borrow() {
            None => Some(0.0), // First event is always at t=0
            Some(_) => {
                // Next event is at the next grid point: ceil( (t + eps) / interval ) * interval
                let next = ((t / self.interval + 1e-9).floor() + 1.0) * self.interval;
                Some(next)
            }
        }
    }

    fn on_step_end(&self, t: f64, _x: &[f64], u: &[&[f64]]) {
        let mut last_t_opt = self.last_save_t.borrow_mut();
        
        let should_save = match *last_t_opt {
            None => true, // Always save the first point (usually t=0)
            Some(last_t) => t >= last_t + self.interval - 1e-9,
        };

        if should_save {
            self.ensure_file_is_open();
            if let Some(ref mut writer) = *self.writer.borrow_mut() {
                write!(writer, "{:.6}", t).unwrap();
                for val in u[0] {
                    write!(writer, ",{:.6}", val).unwrap();
                }
                writeln!(writer).unwrap();
            }
            *last_t_opt = Some(t);
        }
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn get_initial_conditions(&self, _x: &mut [f64]) {}
}
