== Bomba Centrífuga Dinámica

Modelar una bomba centrífuga en tu esquema de tiempo discreto es sumamente elegante porque, desde el punto de vista topológico, la bomba se comporta exactamente igual que una cara (o junction) del caño: es una resistencia activa al flujo.

En lugar de que el motor del caudal sea únicamente la diferencia de presiones de los headers ($Delta P = P_("in") - P_("out")$), la bomba añade un término de altura manométrica o salto de presión activo ($Delta P_("bomba")$) que depende del caudal actual y de la velocidad de giro de la bomba ($N$), determinado por tu lookup table.

Aquí tienes el diseño físico y la implementación en Rust para acoplarla de forma estable y bidireccional.

=== El Modelo Matemático de la Bomba Dinámica

La ecuación de momento discreta para el bloque bomba se deriva modificando la ecuación del caño. El salto de presión total que ve el fluido es el gradiente de los headers más la presión que inyecta el rodete, menos las pérdidas internas de la bomba:

$ W^(n+1) = W^n + (Delta t dot A) / L ( P_("in") - P_("out") + Delta P_("bomba")(W^n, N) - "Fricción"(W^n) ) $

==== Las Leyes de Afinidad para la Lookup Table

Por lo general, el fabricante entrega la curva de la bomba como una tabla de datos de Altura ($H$ en metros) versus Caudal Volumétrico ($Q$ en $m^3/h$) a una velocidad de giro nominal ($N_("nom")$).

Si la simulación permite que la bomba varíe su velocidad (por ejemplo, controlada por un variador de frecuencia o por la inercia del motor en un transitorio de parada), usamos las Leyes de Afinidad de las bombas centrífugas para escalar la curva a cualquier velocidad $N$ actual sin necesitar una tabla bidimensional gigante:

$ Q = Q_("tab") dot (N / N_("nom")) => W = W_("tab") dot (N / N_("nom")) dot rho / rho_("nom") $

$ Delta P_("bomba") = Delta P_("tab") dot (N / N_("nom"))^2 dot rho / rho_("nom") $

Esto significa que es posible usar una única lookup table estática 1D ($W_("tab") arrow.r.double Delta P_("tab")$). Para cualquier caudal real $W$, se "normaliza" a la velocidad nominal, se busca en la tabla la presión nominal, y luego se escala el resultado.

=== Implementación de la Lookup Table 1D Lineal en Rust

Para mantener las miles de consultas por segundo, la lookup table debe hacer una búsqueda binaria rápida e interpolación lineal, evitando cualquier asignación de memoria dinámica (alloc) en el loop.

```rust
pub struct LookupTable1D {
    x: Vec<f64>, // Caudales másicos de prueba (deben estar ordenados)
    y: Vec<f64>, // Saltos de presión correspondientes (Pa)
}

impl LookupTable1D {
    pub fn new(x: Vec<f64>, y: Vec<f64>) -> Self {
        assert_eq!(x.len(), y.len());
        Self { x, y }
    }

    /// Interpolación lineal con extrapolación segura en los extremos
    pub fn interpolate(&self, target_x: f64) -> f64 {
        let n = self.x.len();
        if target_x <= self.x[0] { return self.y[0]; }
        if target_x >= self.x[n - 1] { return self.y[n - 1]; }

        // Búsqueda binaria optimizada (O(log N))
        let idx = match self.x.binary_search_by(|val| val.partial_cmp(&target_x).unwrap()) {
            Ok(exact_idx) => exact_idx,
            Err(insert_idx) => insert_idx - 1,
        };

        // Interpolación lineal estándar
        let x0 = self.x[idx];
        let x1 = self.x[idx + 1];
        let y0 = self.y[idx];
        let y1 = self.y[idx + 1];

        y0 + (target_x - x0) * (y1 - y0) / (x1 - x0)
    }
}
```

=== Estructura e Implementación del Bloque Bomba

```rust
pub struct CentrifugalPumpBlock {
    // Geometría interna (Independiente del dt)
    geom_inertia: f64, // A / L

    // Curva característica a velocidad nominal
    curve_table: LookupTable1D,
    n_nominal: f64,
    rho_nominal: f64,

    fric_factor_pasivo: f64,
    w_last: f64,
}

impl CentrifugalPumpBlock {
    pub fn tick(&mut self, p_in: f64, p_out: f64, rho: f64, speed: f64, dt: f64) -> f64 {
        let delta_p_headers = p_in - p_out;
        let inertia_dt = self.geom_inertia * dt;

        let speed_ratio = speed / self.n_nominal;
        let rho_ratio = rho / self.rho_nominal;

        let dp_bomba = if speed_ratio.abs() > 1e-3 && self.w_last >= 0.0 {
            let w_nominal_look = self.w_last / (speed_ratio * rho_ratio);
            let dp_nominal_look = self.curve_table.interpolate(w_nominal_look);
            dp_nominal_look * speed_ratio.powi(2) * rho_ratio
        } else {
            0.0
        };

        // --- ECUACIÓN DE MOMENTO SEMI-IMPLÍCITA ---
        let numerator = self.w_last + inertia_dt * (delta_p_headers + dp_bomba);

        let w_mod = f64::max(self.w_last.abs(), 1e-4);
        let k_fric = if self.w_last >= 0.0 { self.fric_factor_pasivo } else { self.fric_factor_pasivo * 5.0 };
        let denominator = 1.0 + inertia_dt * (k_fric / rho) * w_mod;

        let w_current = numerator / denominator;
        self.w_last = w_current;
        w_current
    }
}
```



=== Acoplamiento de Energía (Transporte Térmico)

La bomba genera un trabajo sobre el fluido que incrementa su energía. Al igual que con el caño, el transporte de entalpía hacia los headers se calcula afuera de la ecuación de momento usando el Upwind Suavizado:

- Flujo Directo ($W > 0$): La bomba succiona entalpía del Header_In y se la inyecta al Header_Out. Físicamente, parte de la potencia consumida por la bomba que no se convierte en presión (ineficiencia térmica o fricción interna) se disipa directamente como calor en el fluido. Se puede sumar esa ineficiencia como un término $Q_("disipado") = "Potencia" dot (1 - eta)$ directo al flujo de energía que recibe el Header_Out.

- Flujo Inverso ($W < 0$): Si las presiones externas vencen a la bomba y el fluido retrocede, la entalpía viaja desde el Header_Out hacia el Header_In.

