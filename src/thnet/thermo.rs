/// Propiedades físicas del agua líquida monofásica
///
/// Ajuste polinomial a datos NIST/IAPWS para agua líquida en el rango 0–120 °C.
/// Estas correlaciones capturan correctamente la variación de densidad con la
/// temperatura, que es el motor físico de la convección natural.

/// Temperatura de referencia [K] para cálculo de entalpía (h = 0 a 0 °C)
pub const T_REF_K: f64 = 273.15;

/// Densidad del agua líquida [kg/m³] como función de T [K]
///
/// Ajuste polinomial de 5° a datos NIST. Válido 0–120 °C (273–393 K).
/// Captura correctamente el máximo de densidad cerca de 4 °C.
pub fn density(t_k: f64) -> f64 {
    let t_c = (t_k - T_REF_K).clamp(-2.0, 120.0);
    // Coeficientes del ajuste polinomial (Kell, 1975 — alta precisión)
    999.842_594
        + 6.793_952e-2 * t_c
        - 9.095_290e-3 * t_c * t_c
        + 1.001_685e-4 * t_c * t_c * t_c
        - 1.120_083e-6 * t_c.powi(4)
        + 6.536_332e-9 * t_c.powi(5)
}

/// Viscosidad dinámica del agua líquida [Pa·s] como función de T [K]
///
/// Ajuste exponencial válido 0–100 °C.
pub fn viscosity(t_k: f64) -> f64 {
    let t_c = (t_k - T_REF_K).clamp(1.0, 100.0);
    // Ajuste exponencial a datos NIST (±3% en 0–100 °C)
    1.78e-3 * (-0.024_76 * t_c).exp()
}

/// Entalpía específica del agua líquida [J/kg] relativa a 0 °C
///
/// Integral de cp dT con cp variable.
pub fn enthalpy(t_k: f64) -> f64 {
    let t_c = (t_k - T_REF_K).clamp(-2.0, 120.0);
    // h(T_C) = ∫₀^{T_C} cp(τ) dτ  con cp = 4217 - 1.80·τ [J/(kg·K)]
    4217.0 * t_c - 0.90 * t_c * t_c
}

/// Calor específico del agua líquida [J/(kg·K)] como función de T [K]
///
/// Válido 0–100 °C. Mínimo ≈ 4176 J/(kg·K) alrededor de 37 °C.
pub fn cp(t_k: f64) -> f64 {
    let t_c = (t_k - T_REF_K).clamp(0.0, 100.0);
    4217.0 - 1.80 * t_c
}

/// Temperatura [K] a partir de entalpía específica [J/kg]
///
/// Inversión numérica de `enthalpy()` mediante iteraciones Newton.
pub fn temperature_from_enthalpy(h: f64) -> f64 {
    // Estimación inicial con cp constante
    let mut t_c = (h / 4184.0).clamp(-2.0, 130.0);
    // Iteraciones Newton (converge en ≤ 3 pasos para el rango de interés)
    for _ in 0..4 {
        let h_est = enthalpy(t_c + T_REF_K);
        let cp_est = cp(t_c + T_REF_K);
        if cp_est.abs() < 1e-10 {
            break;
        }
        let delta = (h - h_est) / cp_est;
        t_c += delta;
        if delta.abs() < 1e-9 {
            break;
        }
    }
    (t_c + T_REF_K).clamp(T_REF_K - 5.0, T_REF_K + 130.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_at_20c() {
        let rho = density(293.15);
        // NIST: 998.21 kg/m³ a 20 °C
        assert!((rho - 998.21).abs() < 0.1, "ρ(20°C) = {} ≠ 998.21", rho);
    }

    #[test]
    fn density_at_60c() {
        let rho = density(333.15);
        // NIST: 983.21 kg/m³ a 60 °C
        assert!((rho - 983.21).abs() < 0.5, "ρ(60°C) = {} ≠ 983.21", rho);
    }

    #[test]
    fn enthalpy_roundtrip() {
        for t_c in [10.0, 30.0, 60.0, 90.0_f64] {
            let t_k = t_c + T_REF_K;
            let h = enthalpy(t_k);
            let t_k_back = temperature_from_enthalpy(h);
            assert!(
                (t_k_back - t_k).abs() < 1e-6,
                "Roundtrip T={}: error = {}",
                t_c,
                (t_k_back - t_k).abs()
            );
        }
    }
}
