use crate::blocks::{Block, BlockRegistry};
use crate::thermo::{ThermoLib, LinearWater};
use serde_json::Value as JsonValue;
use serde::Deserialize;

// --- HEADER BLOCK ---

pub struct Header {
    volume: f64,
    thermo: ThermoLib,
    temp_indices: Vec<usize>,
    press_indices: Vec<usize>,
}

impl Header {
    pub fn new(volume: f64, thermo: ThermoLib, temp_indices: Vec<usize>, press_indices: Vec<usize>) -> Self {
        Self { volume, thermo, temp_indices, press_indices }
    }

    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params {
            volume: f64,
            #[serde(default)] temp_indices: Vec<usize>,
            #[serde(default)] press_indices: Vec<usize>,
        }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.volume, std::sync::Arc::new(LinearWater), p.temp_indices, p.press_indices)))
    }
}

impl Block for Header {
    fn num_states(&self) -> usize { 2 } // [mass, internal_energy]
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
    }

    fn derivatives(&self, _t: f64, _x: &[f64], u: &[f64], dx: &mut [f64]) {
        let w_net = u[0];
        let wh_net = u[1];
        let q_ext = u[2];

        // Mass must be conserved and positive
        dx[0] = w_net;
        dx[1] = wh_net + q_ext;
    }

    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) {
        let m = x[0].max(1e-6); // Prevent division by zero
        let u_spec = x[1] / m;
        let rho = m / self.volume;
        let state = self.thermo.from_rho_u(rho, u_spec);

        let mut offset = 0;
        for &idx in &self.temp_indices { if idx == 0 { y[offset] = state.t; offset += 1; } }
        for &idx in &self.press_indices { if idx == 0 { y[offset] = state.p; offset += 1; } }
        y[offset] = state.h; offset += 1;
        y[offset] = state.rho;
    }

    fn has_direct_feedthrough(&self) -> bool { false }
}

// --- PIPE 1D BLOCK ---

pub struct Pipe1D {
    n_cells: usize,
    vol_cell: f64,
    geom_inertia: f64,
    fric_factor: f64,
    elevation_drop: f64,
    thermo: ThermoLib,
    temp_indices: Vec<usize>,
    press_indices: Vec<usize>,
}

impl Pipe1D {
    pub fn new(n_cells: usize, length: f64, diameter: f64, elevation_drop: f64, 
               thermo: ThermoLib, temp_indices: Vec<usize>, press_indices: Vec<usize>) -> Self {
        let area = std::f64::consts::PI * (diameter / 2.0).powi(2);
        let dz = length / n_cells as f64;
        let vol_cell = area * dz;
        let geom_inertia = area / dz;
        let fric_factor = (0.02 * dz) / (2.0 * diameter * area.powi(2));
        Self { n_cells, vol_cell, geom_inertia, fric_factor, elevation_drop, thermo, temp_indices, press_indices }
    }

    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)]
        struct Params {
            n_cells: usize, length: f64, diameter: f64, 
            #[serde(default)] elevation_drop: f64,
            #[serde(default)] temp_indices: Vec<usize>,
            #[serde(default)] press_indices: Vec<usize>,
        }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.n_cells, p.length, p.diameter, p.elevation_drop, 
                              std::sync::Arc::new(LinearWater), p.temp_indices, p.press_indices)))
    }

    fn calc_h_face_smooth(&self, w: f64, h_up: f64, h_down: f64) -> f64 {
        let eps = 1e-4;
        if w > eps { h_up } else if w < -eps { h_down } 
        else { ((w+eps)/(2.0*eps))*h_up + (1.0-(w+eps)/(2.0*eps))*h_down }
    }
}

impl Block for Pipe1D {
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

    fn derivatives(&self, _t: f64, x: &[f64], u: &[f64], dx: &mut [f64]) {
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
        for i in 0..=self.n_cells {
            let p_u = if i==0 { p_in } else { p_c[i-1] };
            let p_d = if i==self.n_cells { p_out } else { p_c[i] };
            let rho_f = if i==0 { rho_c[0] } else if i==self.n_cells { rho_c[self.n_cells-1] } else { (rho_c[i-1]+rho_c[i])*0.5 };
            let w = x[i];
            if valve < 1e-3 { dx[i] = -w * 100.0; }
            else {
                // Caras frontera (i=0, i=n) corresponden a media celda:
                // menor inercia (tramo más corto), menor fricción (tramo más corto)
                let half_cell = i == 0 || i == self.n_cells;
                let inertia_f = if half_cell { 2.0 * self.geom_inertia } else { self.geom_inertia };
                let fric_f    = if half_cell { 0.5 * self.fric_factor  } else { self.fric_factor  };
                let dz_f      = if half_cell { 0.5 * dz                } else { dz                };
                dx[i] = inertia_f * (p_u - p_d + rho_f*g*dz_f - (fric_f+k_v)*w*w.abs()/rho_f);
            }
        }

        let q_c = q_ext / self.n_cells as f64;
        for i in 0..self.n_cells {
            let w_u = x[i]; let w_d = x[i+1];
            let wh_u = w_u * self.calc_h_face_smooth(w_u, if i==0 { h_in } else { h_c[i-1] }, h_c[i]);
            let wh_d = w_d * self.calc_h_face_smooth(w_d, h_c[i], if i==self.n_cells-1 { h_out } else { h_c[i+1] });
            dx[m_off+i] = w_u - w_d;
            dx[u_off+i] = wh_u - wh_d + q_c;
        }
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
}

// --- CENTRIFUGAL PUMP BLOCK ---

pub struct CentrifugalPump {
    coeffs: [f64; 3], geom_inertia: f64, fric_pasivo: f64,
}

impl CentrifugalPump {
    pub fn new(coeffs: [f64; 3], geom_inertia: f64, fric_pasivo: f64) -> Self {
        Self { coeffs, geom_inertia, fric_pasivo }
    }
    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct Params { coeffs: [f64; 3], geom_inertia: f64, fric_pasivo: f64 }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.coeffs, p.geom_inertia, p.fric_pasivo)))
    }
}

impl Block for CentrifugalPump {
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
    fn derivatives(&self, _t: f64, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let p_in = u[0]; let p_out = u[2]; let n = u[4]; let rho = if u[5]>0.0 {u[5]} else {1000.0}; let w = x[0];
        let dp_p = self.coeffs[0]*n.powi(2) + self.coeffs[1]*n*w + self.coeffs[2]*w*w.abs();
        // Limit derivative to prevent explosion
        let dw_dt = self.geom_inertia * (p_in - p_out + dp_p - self.fric_pasivo*w*w.abs()/rho);
        dx[0] = dw_dt.clamp(-1e5, 1e5);
    }
    fn outputs(&self, _t: f64, x: &[f64], u: &[f64], y: &mut [f64]) {
        let w = x[0].clamp(-1000.0, 1000.0); 
        let h_f = if w > 1e-4 { u[1] } else if w < -1e-4 { u[3] } else { (u[1]+u[3])*0.5 };
        let wh = w * h_f;
        y[0] = w; y[1] = wh; y[2] = w; y[3] = w; y[4] = wh; y[5] = wh;
    }
    fn has_direct_feedthrough(&self) -> bool { true }
}

// --- STRATIFIED TANK BLOCK ---

pub struct StratifiedTank {
    n_layers: usize, vol_layer: f64, area: f64, dx: f64, thermo: ThermoLib, temp_indices: Vec<usize>, press_indices: Vec<usize>, k_water: f64,
}

impl StratifiedTank {
    pub fn new(n_layers: usize, total_vol: f64, area: f64, thermo: ThermoLib, t_idx: Vec<usize>, p_idx: Vec<usize>) -> Self {
        let vol_l = total_vol / n_layers as f64;
        Self { n_layers, vol_layer: vol_l, area, dx: vol_l/area, thermo, temp_indices: t_idx, press_indices: p_idx, k_water: 0.6 }
    }
    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct Params { n_layers: usize, total_volume: f64, area: f64, #[serde(default)] temp_indices: Vec<usize>, #[serde(default)] press_indices: Vec<usize> }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.n_layers, p.total_volume, p.area, std::sync::Arc::new(LinearWater), p.temp_indices, p.press_indices)))
    }
}

impl Block for StratifiedTank {
    fn num_states(&self) -> usize { self.n_layers * 2 }
    fn num_inputs(&self) -> usize { 5 }
    fn num_outputs(&self) -> usize { 4 }
    fn input_width(&self, _port: usize) -> usize { 1 }
    fn output_width(&self, port: usize) -> usize { match port { 0=>self.temp_indices.len(), 1=>self.press_indices.len(), 2|3=>1, _=>0 } }
    fn get_initial_conditions(&self, x: &mut [f64]) {
        let st = self.thermo.from_p_t(1e5, 293.15); let m = st.rho*self.vol_layer;
        for i in 0..self.n_layers { x[i]=m; x[self.n_layers+i]=st.u*m; }
    }
    fn derivatives(&self, _t: f64, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let w_t = u[0]; let wh_t = u[1]; let w_b = u[2]; let wh_b = u[3]; let q_e = u[4];
        let mut t_l = vec![0.0; self.n_layers]; let mut h_l = vec![0.0; self.n_layers];
        for i in 0..self.n_layers {
            let st = self.thermo.from_rho_u(x[i]/self.vol_layer, x[self.n_layers+i]/(x[i].max(1e-6)));
            t_l[i] = st.t; h_l[i] = st.h;
        }
        for i in 0..self.n_layers {
            let mut dw = 0.0; let mut dwh = 0.0;
            if i==0 { dw += w_t; dwh += wh_t; }
            if i==self.n_layers-1 { dw += w_b; dwh += wh_b; }
            if i>0 { let wh = if w_t>0.0 { w_t*h_l[i-1] } else { w_t*h_l[i] }; dw += w_t; dwh += wh; }
            if i<self.n_layers-1 { let wh = if w_t>0.0 { w_t*h_l[i] } else { w_t*h_l[i+1] }; dw -= w_t; dwh -= wh; }
            if i>0 { let q = (self.k_water*self.area/self.dx)*(t_l[i-1]-t_l[i]); dwh += q; if t_l[i-1]<t_l[i]-0.1 { dwh -= 1000.0*self.area*(t_l[i]-t_l[i-1]); } }
            if i<self.n_layers-1 { let q = (self.k_water*self.area/self.dx)*(t_l[i+1]-t_l[i]); dwh += q; if t_l[i]<t_l[i+1]-0.1 { dwh += 1000.0*self.area*(t_l[i+1]-t_l[i]); } }
            dx[i] = dw; dx[self.n_layers+i] = dwh + q_e/self.n_layers as f64;
        }
    }
    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) {
        let mut t_l = vec![0.0; self.n_layers]; let mut p_l = vec![0.0; self.n_layers]; let mut h_l = vec![0.0; self.n_layers];
        for i in 0..self.n_layers {
            let st = self.thermo.from_rho_u(x[i]/self.vol_layer, x[self.n_layers+i]/(x[i].max(1e-6)));
            t_l[i]=st.t; p_l[i]=st.p; h_l[i]=st.h;
        }
        let mut off = 0;
        for &idx in &self.temp_indices { if idx<self.n_layers { y[off]=t_l[idx]; off+=1; } }
        for &idx in &self.press_indices { if idx<self.n_layers { y[off]=p_l[idx]; off+=1; } }
        y[off]=h_l[0]; off+=1; y[off]=h_l[self.n_layers-1];
    }
    fn has_direct_feedthrough(&self) -> bool { false }
}

// --- CLOSED TANK BLOCK ---

pub struct ClosedTank {
    v_tot: f64, area: f64, p_nom: f64, v_gas_nom: f64, gamma: f64, thermo: ThermoLib, t_idx: Vec<usize>, p_idx: Vec<usize>,
}

impl ClosedTank {
    pub fn new(v_tot: f64, area: f64, p_nom: f64, v_gas_nom: f64, thermo: ThermoLib, t_idx: Vec<usize>, p_idx: Vec<usize>) -> Self {
        Self { v_tot, area, p_nom, v_gas_nom, gamma: 1.4, thermo, t_idx, p_idx }
    }
    pub fn build(params: JsonValue, _reg: &BlockRegistry) -> Result<Box<dyn Block>, String> {
        #[derive(Deserialize)] struct Params { v_total: f64, area: f64, p_gas_nominal: f64, v_gas_nominal: f64, #[serde(default)] temp_indices: Vec<usize>, #[serde(default)] press_indices: Vec<usize> }
        let p: Params = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(Box::new(Self::new(p.v_total, p.area, p.p_gas_nominal, p.v_gas_nominal, std::sync::Arc::new(LinearWater), p.temp_indices, p.press_indices)))
    }
}

impl Block for ClosedTank {
    fn num_states(&self) -> usize { 2 }
    fn num_inputs(&self) -> usize { 5 }
    fn num_outputs(&self) -> usize { 6 }
    fn input_width(&self, _port: usize) -> usize { 1 }
    fn output_width(&self, port: usize) -> usize { match port { 0=>self.t_idx.len(), 1=>self.p_idx.len(), 2|3|4|5=>1, _=>0 } }
    fn get_initial_conditions(&self, x: &mut [f64]) {
        let st = self.thermo.from_p_t(self.p_nom, 293.15); let v_l = self.v_tot - self.v_gas_nom;
        x[0]=st.rho*v_l; x[1]=st.u*x[0];
    }
    fn derivatives(&self, _t: f64, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let w_i=u[0]; let wh_i=u[1]; let w_o=u[2]; let wh_o=u[3]; let q_e=u[4];
        let m=x[0].max(1e-6); let u_spec=x[1]/m; 
        
        // rho_target check to avoid sqrt(neg) or other EOS issues
        let st = self.thermo.from_rho_u(1000.0, u_spec);
        let v_l = (m/st.rho).min(self.v_tot * 0.99); 
        let v_g = (self.v_tot-v_l).max(1e-4);
        let p_g = self.p_nom * (self.v_gas_nom/v_g).powf(self.gamma);
        
        // Hard mass limits in derivative
        let m_net = w_i - w_o;
        dx[0] = if x[0] <= 1e-6 && m_net < 0.0 { 0.0 } else { m_net };
        dx[1] = wh_i - wh_o + q_e - p_g*(m_net)/st.rho;
    }
    fn outputs(&self, _t: f64, x: &[f64], _u: &[f64], y: &mut [f64]) {
        let m=x[0].max(1e-6); let st = self.thermo.from_rho_u(1000.0, x[1]/m);
        let v_l = m/st.rho; let v_g = (self.v_tot-v_l).max(1e-4);
        let p_g = self.p_nom * (self.v_gas_nom/v_g).powf(self.gamma);
        let level = v_l/self.area; let p_b = p_g + st.rho*9.81*level;
        let mut off=0;
        for &idx in &self.t_idx { if idx==0 { y[off]=st.t; off+=1; } }
        for &idx in &self.p_idx { if idx==0 { y[off]=p_b; off+=1; } }
        y[off]=level; off+=1; y[off]=st.h; off+=1; y[off]=st.rho; off+=1; y[off]=p_b;
    }
    fn has_direct_feedthrough(&self) -> bool { false }
}
