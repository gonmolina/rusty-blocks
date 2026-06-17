use crate::blocks::{Block, BlockRegistry};
use crate::thermo::{ThermoLib, LinearWater};
use serde_json::Value as JsonValue;
use serde::Deserialize;

// --- DISCRETE HEADER BLOCK ---

pub struct DiscreteHeader {
    volume: f64,
    thermo: ThermoLib,
    temp_indices: Vec<usize>,
    press_indices: Vec<usize>,
    ts: f64,
    pressure_filter_alpha: f64,
}

impl DiscreteHeader {
    pub fn new(volume: f64, thermo: ThermoLib, temp_indices: Vec<usize>, press_indices: Vec<usize>, ts: f64, pressure_filter_alpha: f64) -> Self {
        Self { volume, thermo, temp_indices, press_indices, ts, pressure_filter_alpha }
    }

    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params {
            volume: f64,
            ts: f64,
            #[serde(default)] temp_indices: Vec<usize>,
            #[serde(default)] press_indices: Vec<usize>,
            #[serde(default)] pressure_filter_alpha: Option<f64>,
        }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        let alpha = p.pressure_filter_alpha.unwrap_or(0.9);
        Ok(Box::new(Self::new(p.volume, std::sync::Arc::new(LinearWater), p.temp_indices, p.press_indices, p.ts, alpha)))
    }
}

impl Block for DiscreteHeader {
    fn num_states(&self) -> usize { 3 } // [mass, internal_energy, p_out]
    fn num_inputs(&self) -> usize { 3 } // [w_net, wh_net, q_ext]
    fn num_outputs(&self) -> usize { 4 } 
    
    fn input_width(&self, _port: usize) -> usize { 1 }
    fn output_width(&self, port: usize) -> usize {
        match port {
            0 => self.temp_indices.len(),
            1 => self.press_indices.len(),
            2 => 1, // Enthalpy (Boundary)
            3 => 1, // Density (Boundary)
            _ => 0,
        }
    }

    fn get_initial_conditions(&self, x: &mut [f64]) {
        let state = self.thermo.from_p_t(1e5, 293.15);
        x[0] = state.rho * self.volume;
        x[1] = state.u * x[0];
        x[2] = state.p;
    }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], dx: &mut [f64]) {
        dx.fill(0.0);
    }

    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) {
        let m = x[0].max(1e-6); 
        let u_spec = x[1] / m;
        let rho = m / self.volume;
        let state = self.thermo.from_rho_u(rho, u_spec);

        let mut offset = 0;
        for &idx in &self.temp_indices { if idx == 0 { y[offset] = state.t; offset += 1; } }
        // NOTA: Se expone la presión FILTRADA x[2], no la presión termodinámica state.p.
        // Esto es intencional: el filtro IIR amortigua oscilaciones acústicas de alta frecuencia.
        // En estado estacionario, x[2] → state.p (ganancia DC = 1 del filtro IIR).
        for &idx in &self.press_indices { if idx == 0 { y[offset] = x[2]; offset += 1; } }
        y[offset] = state.h; offset += 1;
        y[offset] = state.rho;
    }

    fn has_direct_feedthrough(&self) -> bool { false }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let w_net = u[0];
        let wh_net = u[1];
        let q_ext = u[2];

        let mut nx = vec![0.0; 3];
        nx[0] = (x[0] + w_net * self.ts).max(1e-6);
        nx[1] = x[1] + (wh_net + q_ext) * self.ts;

        let rho = nx[0] / self.volume;
        let u_spec = nx[1] / nx[0];
        let state = self.thermo.from_rho_u(rho, u_spec);

        // Filter pressure: p_out_new = (1.0 - alpha) * p_real + alpha * p_out_old
        nx[2] = (1.0 - self.pressure_filter_alpha) * state.p + self.pressure_filter_alpha * x[2];

        nx
    }
}

// --- DISCRETE PIPE 1D BLOCK ---

pub struct DiscretePipe1D {
    n_cells: usize,
    vol_cell: f64,
    geom_inertia: f64,
    fric_factor: f64,
    elevation_drop: f64,
    thermo: ThermoLib,
    temp_indices: Vec<usize>,
    press_indices: Vec<usize>,
    ts: f64,
}

impl DiscretePipe1D {
    pub fn new(n_cells: usize, length: f64, diameter: f64, elevation_drop: f64, 
               thermo: ThermoLib, temp_indices: Vec<usize>, press_indices: Vec<usize>, ts: f64) -> Self {
        let area = std::f64::consts::PI * (diameter / 2.0).powi(2);
        let dz = length / n_cells as f64;
        let vol_cell = area * dz;
        let geom_inertia = area / dz;
        let fric_factor = (0.02 * dz) / (2.0 * diameter * area.powi(2));
        Self { n_cells, vol_cell, geom_inertia, fric_factor, elevation_drop, thermo, temp_indices, press_indices, ts }
    }

    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params {
            n_cells: usize, length: f64, diameter: f64, ts: f64,
            #[serde(default)] elevation_drop: f64,
            #[serde(default)] temp_indices: Vec<usize>,
            #[serde(default)] press_indices: Vec<usize>,
        }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.n_cells, p.length, p.diameter, p.elevation_drop, 
                              std::sync::Arc::new(LinearWater), p.temp_indices, p.press_indices, p.ts)))
    }

    fn calc_h_face_smooth(&self, w: f64, h_up: f64, h_down: f64) -> f64 {
        let eps = 1e-4;
        if w > eps { h_up } else if w < -eps { h_down } 
        else { ((w+eps)/(2.0*eps))*h_up + (1.0-(w+eps)/(2.0*eps))*h_down }
    }
}

impl Block for DiscretePipe1D {
    fn num_states(&self) -> usize { (self.n_cells + 1) + 2 * self.n_cells }
    fn num_inputs(&self) -> usize { 6 } 
    fn num_outputs(&self) -> usize { 8 }

    fn input_width(&self, _port: usize) -> usize { 1 }
    fn output_width(&self, port: usize) -> usize {
        match port {
            0 => self.temp_indices.len(),
            1 => self.press_indices.len(),
            2 => self.n_cells + 1, // W_all
            3 => self.n_cells + 1, // WH_all
            4 => 1, // W_in
            5 => 1, // W_out
            6 => 1, // WH_in
            7 => 1, // WH_out
            _ => 0,
        }
    }

    fn get_initial_conditions(&self, x: &mut [f64]) {
        let state = self.thermo.from_p_t(1e5, 293.15);
        let m = state.rho * self.vol_cell;
        let u = state.u * m;
        for i in 0..self.n_cells {
            x[self.n_cells + 1 + i] = m;
            x[2 * self.n_cells + 1 + i] = u;
        }
    }

    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], dx: &mut [f64]) {
        dx.fill(0.0);
    }

    fn outputs(&self, _t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        let h_in = u[1]; let h_out = u[3];
        let m_off = self.n_cells + 1; let u_off = 2 * self.n_cells + 1;
        let mut t_c = vec![0.0; self.n_cells]; let mut p_c = vec![0.0; self.n_cells]; let mut h_c = vec![0.0; self.n_cells];
        for i in 0..self.n_cells {
            let st = self.thermo.from_rho_u(x[m_off+i]/self.vol_cell, x[u_off+i]/x[m_off+i].max(1e-6));
            t_c[i] = st.t; p_c[i] = st.p; h_c[i] = st.h;
        }
        let mut off = 0;
        for &idx in &self.temp_indices { if idx < self.n_cells { y[off] = t_c[idx]; off += 1; } }
        for &idx in &self.press_indices { if idx < self.n_cells { y[off] = p_c[idx]; off += 1; } }
        for i in 0..=self.n_cells { y[off] = x[i]; off += 1; } // W_all
        let mut wh_all = vec![0.0; self.n_cells+1];
        for i in 0..=self.n_cells {
            let w = x[i]; let h_u = if i==0 { h_in } else { h_c[i-1] }; let h_d = if i==self.n_cells { h_out } else { h_c[i] };
            wh_all[i] = w * self.calc_h_face_smooth(w, h_u, h_d);
            y[off] = wh_all[i]; off += 1;
        }
        y[off] = x[0]; off += 1; y[off] = x[self.n_cells]; off += 1;
        y[off] = wh_all[0]; off += 1; y[off] = wh_all[self.n_cells];
    }

    fn has_direct_feedthrough(&self) -> bool { true }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let p_in = u[0]; let h_in = u[1]; let p_out = u[2]; let h_out = u[3];
        let valve = u[4].clamp(1e-4, 1.0); let q_ext = u[5];
        let m_off = self.n_cells + 1; let u_off = 2 * self.n_cells + 1;

        let mut p_c = vec![0.0; self.n_cells];
        let mut h_c = vec![0.0; self.n_cells];
        let mut rho_c = vec![0.0; self.n_cells];
        for i in 0..self.n_cells {
            let rho = x[m_off+i] / self.vol_cell;
            let st = self.thermo.from_rho_u(rho, x[u_off+i] / x[m_off+i].max(1e-6));
            p_c[i] = st.p; h_c[i] = st.h; rho_c[i] = st.rho;
        }

        let k_v = if valve < 1e-3 { 0.0 } else { (1.0/valve.powi(2)-1.0)*100.0 };
        let g = 9.80665; let dz = self.elevation_drop / self.n_cells as f64;
        let mut w_next = vec![0.0; self.n_cells + 1];
        for i in 0..=self.n_cells {
            let p_u = if i==0 { p_in } else { p_c[i-1] };
            let p_d = if i==self.n_cells { p_out } else { p_c[i] };
            let rho_f = if i==0 { rho_c[0] } else if i==self.n_cells { rho_c[self.n_cells-1] } else { (rho_c[i-1]+rho_c[i])*0.5 };
            
            // Caras frontera (i=0, i=n) corresponden a media celda:
            // mayor inercia geométrica (A/Δz_mid con Δz_mid = Δz/2)
            // menor fricción (tramo más corto)
            let half_cell = i == 0 || i == self.n_cells;
            let inertia_f = if half_cell { 2.0 * self.geom_inertia } else { self.geom_inertia };
            let fric_f    = if half_cell { 0.5 * self.fric_factor  } else { self.fric_factor  };
            let dz_f      = if half_cell { 0.5 * dz                } else { dz                };
            
            let w_prev = x[i];
            // Esquema semi-implícito en momento: fricción en denominador evita inestabilidad a caudal bajo.
            // Término de presión sigue siendo explícito (ver estabilidad CFL en documento de diseño).
            let numerator = w_prev + self.ts * inertia_f * (p_u - p_d + rho_f * g * dz_f);
            let denominator = 1.0 + self.ts * inertia_f * ((fric_f + k_v) / rho_f) * w_prev.abs();
            w_next[i] = numerator / denominator;
        }

        let mut m_next = vec![0.0; self.n_cells];
        let mut u_next = vec![0.0; self.n_cells];
        let q_c = q_ext / self.n_cells as f64;
        for i in 0..self.n_cells {
            let w_u = w_next[i]; let w_d = w_next[i+1];
            let wh_u = w_u * self.calc_h_face_smooth(w_u, if i==0 { h_in } else { h_c[i-1] }, h_c[i]);
            let wh_d = w_d * self.calc_h_face_smooth(w_d, h_c[i], if i==self.n_cells-1 { h_out } else { h_c[i+1] });
            m_next[i] = (x[m_off+i] + (w_u - w_d) * self.ts).max(1e-6);
            u_next[i] = x[u_off+i] + (wh_u - wh_d + q_c) * self.ts;
        }

        let mut nx = vec![0.0; self.num_states()];
        nx[0..=self.n_cells].copy_from_slice(&w_next);
        nx[m_off..u_off].copy_from_slice(&m_next);
        nx[u_off..].copy_from_slice(&u_next);
        nx
    }
}

// --- DISCRETE CENTRIFUGAL PUMP BLOCK ---

pub struct DiscreteCentrifugalPump {
    coeffs: [f64; 3],
    geom_inertia: f64,
    fric_pasivo: f64,
    ts: f64,
}

impl DiscreteCentrifugalPump {
    pub fn new(coeffs: [f64; 3], geom_inertia: f64, fric_pasivo: f64, ts: f64) -> Self {
        Self { coeffs, geom_inertia, fric_pasivo, ts }
    }
    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params {
            coeffs: [f64; 3],
            geom_inertia: f64,
            fric_pasivo: f64,
            ts: f64,
        }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.coeffs, p.geom_inertia, p.fric_pasivo, p.ts)))
    }
}

impl Block for DiscreteCentrifugalPump {
    fn num_states(&self) -> usize { 1 }
    fn num_inputs(&self) -> usize { 6 } 
    fn num_outputs(&self) -> usize { 8 }
    fn input_width(&self, _port: usize) -> usize { 1 }
    fn output_width(&self, port: usize) -> usize {
        match port {
            0 | 1 => 0,
            2 | 3 | 4 | 5 | 6 | 7 => 1,
            _ => 0,
        }
    }
    fn get_initial_conditions(&self, x: &mut [f64]) { x[0] = 0.0; }
    fn derivatives(&self, _t: f64, _x: &[f64], _u: &[f64], dx: &mut [f64]) { dx.fill(0.0); }
    fn outputs(&self, _t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        let w = x[0].clamp(-1000.0, 1000.0); 
        let h_f = if w > 1e-4 { u[1] } else if w < -1e-4 { u[3] } else { (u[1]+u[3])*0.5 };
        let wh = w * h_f;
        y[0] = w; y[1] = wh; y[2] = w; y[3] = w; y[4] = wh; y[5] = wh;
    }
    fn has_direct_feedthrough(&self) -> bool { true }
    fn sample_time(&self) -> Option<f64> { Some(self.ts) }

    fn update(&self, _t: f64, x: &[f64], u: &[f64]) -> Vec<f64> {
        let p_in = u[0]; let p_out = u[2]; let n = u[4]; let rho = if u[5]>0.0 {u[5]} else {1000.0}; let w_prev = x[0];
        let dp_p = self.coeffs[0]*n.powi(2) + self.coeffs[1]*n*w_prev + self.coeffs[2]*w_prev*w_prev.abs();
        
        let numerator = w_prev + self.ts * self.geom_inertia * (p_in - p_out + dp_p);
        let w_mod = f64::max(w_prev.abs(), 1e-4);
        let denominator = 1.0 + self.ts * self.geom_inertia * (self.fric_pasivo / rho) * w_mod;
        
        vec![numerator / denominator]
    }
}
