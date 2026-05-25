#import "@preview/gentle-clues:1.3.1": *

== Modelo Termohidráulico de Tubería 1D Discretizada

Este documento detalla la arquitectura, el modelado matemático y la implementación en Rust de un componente de tubería unidimensional (Pipe1D) basado en volúmenes finitos. El modelo está diseñado para simulaciones de tiempo discreto de fluidos monofásicos (como sistemas de refrigeración de reactores o circuitos de agua pesada), priorizando la estabilidad numérica, la velocidad de ejecución por ciclo y el manejo natural de flujos bidireccionales y de bajo caudal.

=== Topología del Modelo (Grilla Escalonada)

Para evitar la inestabilidad espacial que sufren los esquemas centrados, el Pipe1D utiliza una grilla escalonada (staggered grid). La tubería se divide en $N$ volúmenes de control, lo que genera dos dominios de cálculo paralelos:

Celdas ($N$ elementos): Representan los volúmenes finitos. Aquí residen las variables escalares: presión ($p$), entalpía ($h$) y densidad ($rho$).

Caras o Uniones ($N+1$ elementos): Representan las fronteras entre celdas (y los extremos conectados a los headers externos). Aquí reside la variable vectorial: el caudal másico ($W$).

Esta topología permite evaluar los gradientes de presión directamente sobre las caras, y los balances de masa directamente sobre las celdas.

=== Estructura de Datos y Caché (SoA)

```rust
pub struct Pipe1D {
    pub n_cells: usize,

    // Geometría (Parámetros fijos)
    vol_cell: f64,
    geom_inertia: f64, // A / dz
    fric_factor: f64,  // (f * dz) / (2 * D * A^2)
    elevation_drop: f64, // z_in - z_out [m]

    // ...
}
```

=== Estabilización Numérica y Límites

#warning(title: "Límite de Estabilidad CFL")[
  Para que la integración de energía (advección) sea estable, el paso de tiempo debe ser menor al tiempo de tránsito de una celda:
  $ Delta t <= frac(rho dot V_("cell"), |W|) $
  Si el flujo es muy rápido ($|W|$ grande) o las celdas muy pequeñas ($V_("cell")$ pequeño), el sistema oscilará. Se recomienda reducir el $Delta t$ global o aumentar el volumen de las celdas.
]

==== Momento: Fricción Semi-Implícita y Gravedad

Para habilitar la *convección natural*, la ecuación de momento se extiende con el término de cabezal hidrostático:

$
  W^(n+1) = frac(W^n + frac(Delta t dot A, Delta z) (Delta P + rho dot g dot Delta z_("cell")), 1 + frac(Delta t dot A, Delta z) K_("total") dot max(|W^n|, epsilon_w))
$

Donde $Delta z_("cell") = "elevation_drop" / N$. Si una rama del circuito está más caliente, su densidad $rho$ será menor, generando una diferencia de presión neta que impulsa el fluido sin necesidad de bombas.

=== Implementación del Motor de Simulación (Método tick)

```rust
impl Pipe1D {
    pub fn tick(&mut self, p_in: f64, h_in: f64, p_out: f64, h_out: f64, dt: f64) -> (f64, f64) {

        // 1. ECUACIÓN DE MOMENTO (N+1 Caras)
        let inertia_dt = self.geom_inertia * dt;

        for i in 0..=self.n_cells {
            let p_up = if i == 0 { p_in } else { self.p[i-1] };
            let p_down = if i == self.n_cells { p_out } else { self.p[i] };
            let rho_f = if i == 0 { self.rho[0] } 
                        else if i == self.n_cells { self.rho[self.n_cells-1] }
                        else { (self.rho[i-1] + self.rho[i]) * 0.5 };

            self.w[i] = self.calc_flow_semi_implicit(self.w_last[i], p_up, p_down, rho_f, inertia_dt);
        }

        // 2. BALANCES CONSERVATIVOS (N Celdas)
        for i in 0..self.n_cells {
            let phi_in = self.w[i] * self.calc_h_face_smooth(self.w[i], 
                         if i == 0 { h_in } else { self.h[i-1] }, self.h[i]);
            let phi_out = self.w[i+1] * self.calc_h_face_smooth(self.w[i+1], 
                          self.h[i], if i == self.n_cells - 1 { h_out } else { self.h[i+1] });

            self.m[i] += (self.w[i] - self.w[i+1]) * dt;
            self.u[i] += (phi_in - phi_out) * dt;

            let rho_real = self.m[i] / self.vol_cell;
            let u_spec = self.u[i] / self.m[i];
            
            let (p_new, h_new, _t_new) = thermo::get_state_from_rho_u(rho_real, u_spec);
            self.p[i] = p_new;
            self.h[i] = h_new;
            self.rho[i] = rho_real;
        }

        self.w_last.copy_from_slice(&self.w);
        (self.w[0], self.w[self.n_cells])
    }

    #[inline(always)]
    fn calc_flow_semi_implicit(&self, w_prev: f64, p_up: f64, p_down: f64, rho: f64, inertia_dt: f64) -> f64 {
        let delta_p = p_up - p_down;
        let numerator = w_prev + inertia_dt * delta_p;
        let w_mod = f64::max(w_prev.abs(), 1e-4);
        let denominator = 1.0 + inertia_dt * (self.fric_factor / rho) * w_mod;
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
}
```
