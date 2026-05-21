
== Modelo Termohidráulico de Tubería 1D Discretizada

Este documento detalla la arquitectura, el modelado matemático y la implementación en Rust de un componente de tubería unidimensional (Pipe1D) basado en volúmenes finitos. El modelo está diseñado para simulaciones de tiempo discreto de fluidos monofásicos (como sistemas de refrigeración de reactores o circuitos de agua pesada), priorizando la estabilidad numérica, la velocidad de ejecución por ciclo y el manejo natural de flujos bidireccionales y de bajo caudal.

=== Topología del Modelo (Grilla Escalonada)

Para evitar la inestabilidad espacial que sufren los esquemas centrados, el Pipe1D utiliza una grilla escalonada (staggered grid). La tubería se divide en $N$ volúmenes de control, lo que genera dos dominios de cálculo paralelos:

Celdas ($N$ elementos): Representan los volúmenes finitos. Aquí residen las variables escalares: presión ($p$), entalpía ($h$) y densidad ($rho$).

Caras o Uniones ($N+1$ elementos): Representan las fronteras entre celdas (y los extremos conectados a los headers externos). Aquí reside la variable vectorial: el caudal másico ($W$).

Esta topología permite evaluar los gradientes de presión directamente sobre las caras, y los balances de masa directamente sobre las celdas.

=== Estructura de Datos y Caché (SoA)

Para maximizar el rendimiento del procesador y habilitar la vectorización, la memoria se organiza bajo el patrón SoA (Structure of Arrays). En lugar de tener un vector de objetos "Celda", se utilizan vectores pre-asignados y contiguos en memoria para cada propiedad física.

```rust
pub struct Pipe1D {
pub n_cells: usize,

// Geometría
vol_cell: f64,
inertia_gain: f64, // (dt * A) / dz
fric_factor: f64,  // (f * dz) / (2 * D * A^2)

// Estado de las CARAS (Tamaño: N + 1)
w: Vec<f64>,       // Caudal actual (kg/s)
w_last: Vec<f64>,  // Memoria z^-1 del caudal (kg/s)

// Estado de las CELDAS (Tamaño: N)
p: Vec<f64>,       // Presión (Pa)
h: Vec<f64>,       // Entalpía (J/kg)
rho: Vec<f64>,     // Densidad (kg/m³)
}
```


=== Estabilización Numérica

La simulación en tiempo discreto puro requiere estrategias específicas para evitar oscilaciones espurias (chattering) y explosiones por el límite CFL (Courant).

==== Momento: Fricción Semi-Implícita

A caudales muy bajos, el término de fricción cuadrática tradicional pierde su capacidad de amortiguación. Para garantizar la estabilidad incondicional de la ecuación de momento ante cualquier paso de tiempo $Delta t$, se emplea una linealización semi-implícita de la fricción:

$
  W^(n+1) = (W^n + (Delta t dot A) / (Delta z) Delta P) / (1 + (Delta t dot A) / (Delta z) K_("fric") dot max(|W^n|, epsilon_w))
$

Donde $epsilon_w$ es un piso de regularización que impone un comportamiento de flujo laminar cerca del cero.

==== Energía: Esquema Upwind Suavizado

Para el transporte de energía, el modelo debe capturar la dirección del flujo. Si se utiliza un esquema Upwind rígido, el cruce por cero del caudal induce oscilaciones térmicas severas. Se implementa una banda de transición lineal alrededor del flujo nulo para promediar las entalpías de las celdas adyacentes.

==== Masa: Ablandamiento de Presión (Capacitancia Numérica)

Dado que los líquidos son débilmente compresibles, cambios microscópicos de masa generan saltos masivos de presión. La EDO de presión se evalúa de
forma explícita expandiendo la derivada total de la densidad, pero incrementando virtualmente la compresibilidad isotérmica $(partial rho) / (partial p)$ para "ablandar" el acoplamiento:

$ d p = ((W_("in") - W_("out")) dot Delta t) / (V_("cell") dot ((partial rho) / (partial p))_("virtual")) $

=== Implementación del Motor de Simulación (Método tick)

El lazo de resolución principal ejecuta el avance temporal en tres fases secuenciales. Este método debe ser llamado por el orquestador principal (el sistema de bloques) pasándole las condiciones de borde provenientes de los headers externos.

```rust
impl Pipe1D {
    /// Avanza el estado interno de la tubería un paso de tiempo dt.
    /// Retorna los caudales en los extremos (w_in, w_out) para acoplar con los headers.
    pub fn tick(&mut self, p_in: f64, h_in: f64, p_out: f64, h_out: f64, dt: f64) -> (f64, f64) {

        // =================================================================
        // FASE 1: ECUACIÓN DE MOMENTO (Resolución en N+1 Caras)
        // =================================================================

        // Cara 0 (Frontera de entrada)
        let rho_f0 = self.rho[0];
        self.w[0] = self.calc_flow_semi_implicit(self.w_last[0], p_in, self.p[0], rho_f0);

        // Caras internas (1 a N-1)
        for i in 1..self.n_cells {
            let rho_f = (self.rho[i-1] + self.rho[i]) * 0.5; // Densidad promediada en la cara
            self.w[i] = self.calc_flow_semi_implicit(self.w_last[i], self.p[i-1], self.p[i], rho_f);
        }

        // Cara N (Frontera de salida)
        let rho_fN = self.rho[self.n_cells - 1];
        self.w[self.n_cells] = self.calc_flow_semi_implicit(self.w_last[self.n_cells], self.p[self.n_cells-1], p_out, rho_fN);

        // =================================================================
        // FASE 2: ENERGÍA Y MASA (Resolución en N Celdas)
        // =================================================================

        // 2.a Calcular flujos de energía (Phi) en las caras con Upwind suavizado
        let mut phi = vec![0.0; self.n_cells + 1];

        // Phi en la entrada
        let h_face_0 = self.calc_h_face_smooth(self.w[0], h_in, self.h[0]);
        phi[0] = self.w[0] * h_face_0;

        // Phi en caras internas
        for i in 1..self.n_cells {
            let h_face = self.calc_h_face_smooth(self.w[i], self.h[i-1], self.h[i]);
            phi[i] = self.w[i] * h_face;
        }

        // Phi en la salida
        let h_face_N = self.calc_h_face_smooth(self.w[self.n_cells], self.h[self.n_cells - 1], h_out);
        phi[self.n_cells] = self.w[self.n_cells] * h_face_N;

        // 2.b Resolver EDOs en las celdas
        for i in 0..self.n_cells {
            let mass = self.vol_cell * self.rho[i];

            // Balance universal: entra por la izquierda, sale por la derecha
            let net_energy_flow = phi[i] - phi[i+1];
            let net_mass_flow = self.w[i] - self.w[i+1];

            // Integración de Energía
            let dh = net_energy_flow * dt / mass;
            self.h[i] += dh;

            // Integración de Presión (Ablandada)
            // drho_dp virtual: ajustar en función de la estabilidad requerida (CFL)
            let drho_dp_virtual = 4.5e-10 * self.rho[i] * (self.n_cells as f64);
            let dp = net_mass_flow * dt / (self.vol_cell * drho_dp_virtual);
            self.p[i] += dp;

            // Actualización Termodinámica (IAPWS-IF97 Región 1)
            self.rho[i] = self.get_density_if97(self.p[i], self.h[i]);
        }

        // =================================================================
        // FASE 3: ACTUALIZACIÓN DE ESTADO z^-1
        // =================================================================
        self.w_last.copy_from_slice(&self.w);

        (self.w[0], self.w[self.n_cells])
    }

    // Funciones auxiliares (Inlining forzado para máxima performance)

    #[inline(always)]
    fn calc_flow_semi_implicit(&self, w_prev: f64, p_up: f64, p_down: f64, rho: f64) -> f64 {
        let delta_p = p_up - p_down;
        let numerator = w_prev + self.inertia_gain * delta_p;
        let w_mod = f64::max(w_prev.abs(), 1e-4);
        let denominator = 1.0 + self.inertia_gain * (self.fric_factor / rho) * w_mod;
        numerator / denominator
    }

    #[inline(always)]
    fn calc_h_face_smooth(&self, w: f64, h_up: f64, h_down: f64) -> f64 {
        let epsilon_w = 1e-4;
        if w > epsilon_w {
            h_up
        } else if w < -epsilon_w {
            h_down
        } else {
            let factor = (w + epsilon_w) / (2.0 * epsilon_w);
            factor * h_up + (1.0 - factor) * h_down
        }
    }

    #[inline(always)]
    fn get_density_if97(&self, p: f64, h: f64) -> f64 {
        1000.0 // Valor simulado
    }
}
```
