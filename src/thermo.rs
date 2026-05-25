use std::sync::Arc;

pub struct ThermoState {
    pub p: f64,         // Presión [Pa]
    pub t: f64,         // Temperatura [K]
    pub rho: f64,       // Densidad [kg/m³]
    pub h: f64,         // Entalpía específica [J/kg]
    pub u: f64,         // Energía interna específica [J/kg]
    
    // Propiedades de transporte (opcionales)
    pub cp: f64,        // Calor específico a P constante [J/kg·K]
    pub cv: f64,        // Calor específico a V constante [J/kg·K]
    
    // Derivadas parciales para estabilidad numérica (Jacobianos)
    pub drho_dp_h: f64, // (∂ρ/∂p)_h
    pub drho_dh_p: f64, // (∂ρ/∂h)_p
}

pub trait ThermoLibrary: Send + Sync {
    fn name(&self) -> &str;
    fn from_p_h(&self, p: f64, h: f64) -> ThermoState;
    fn from_rho_u(&self, rho: f64, u: f64) -> ThermoState;
    fn from_p_t(&self, p: f64, t: f64) -> ThermoState;
}

pub type ThermoLib = Arc<dyn ThermoLibrary>;

/// Implementación simple lineal para pruebas (Agua líquida aproximada)
pub struct LinearWater;

impl ThermoLibrary for LinearWater {
    fn name(&self) -> &str { "LinearWater" }

    fn from_p_h(&self, p: f64, h: f64) -> ThermoState {
        let cp = 4184.0;
        let rho0 = 1000.0;
        let t = 293.15 + h / cp;
        ThermoState {
            p,
            t,
            rho: rho0,
            h,
            u: h - p / rho0,
            cp,
            cv: cp,
            drho_dp_h: 4.5e-10 * rho0,
            drho_dh_p: -0.0002 * rho0,
        }
    }

    fn from_rho_u(&self, rho: f64, u: f64) -> ThermoState {
        let cp = 4184.0;
        let rho0 = 1000.0;
        let beta = 1e-4; // Fluido muy suave para estabilidad acústica con dt=0.01
        let alpha = 2e-4; // Coeficiente de expansión térmica (aprox agua)
        let t0 = 293.15;
        
        let t = t0 + u / cp;
        
        // rho_target(T) = rho0 * (1 - alpha * (T - T0))
        let rho_t = rho0 * (1.0 - alpha * (t - t0));
        
        // p = p0 + (rho - rho_target(T)) / (rho0 * beta)
        let p = 1e5 + (rho - rho_t) / (rho0 * beta);
        let h = u + p / rho;
        
        ThermoState {
            p,
            t,
            rho,
            h,
            u,
            cp,
            cv: cp,
            drho_dp_h: beta * rho0,
            drho_dh_p: -alpha * rho0 / cp,
        }
    }

    fn from_p_t(&self, p: f64, t: f64) -> ThermoState {
        let cp = 4184.0;
        let rho0 = 1000.0;
        let h = cp * (t - 293.15);
        ThermoState {
            p,
            t,
            rho: rho0,
            h,
            u: h - p / rho0,
            cp,
            cv: cp,
            drho_dp_h: 4.5e-10 * rho0,
            drho_dh_p: -0.0002 * rho0,
        }
    }
}
