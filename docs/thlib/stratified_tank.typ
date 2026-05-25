
== Tanque Térmico Estratificado

Un tanque estratificado (típico en sistemas termosolares, acumulación térmica o agua caliente sanitaria) se basa en el principio de que el agua caliente es menos densa que el agua fría. Esto genera capas térmicas estables que no se mezclan de forma natural debido a la flotabilidad.

En un esquema de diagramas de bloques distribuidos, se modela como una serie de sub-volúmenes (nodos) acoplados verticalmente. Cada capa actúa matemáticamente como un Header o un pequeño tanque interconectado, pero aplicando dos leyes físicas adicionales: convección forzada (el agua se desplaza entre capas al entrar/salir fluido) y mezclado/conducción (intercambio destructivo de la estratificación).

=== Topología de $N$ Capas ($N = 2, 3, 4 ...$)

Dividir el tanque en 3 o 4 capas suele ser el punto óptimo para simulación en tiempo real.

*2 capas (Termoclina simple)*: Divide el tanque en "Capa Superior Hot" y "Capa Inferior Cold". Es rápido pero no captura bien los frentes térmicos móviles.

*3 a 4 capas*: Permite modelar una zona de transición (termoclina) que sube y baja según la carga y descarga del tanque.

```
       +-----------------------+
       |   Capa 1 (Top / Hot)  |  <- Entrada/Salida Alta (Caldera/Consumo)
       +-----------------------+  ^
       |        Capa 2         |  |  Flujos de acoplamiento interno (W_ij)
       +-----------------------+  |  Conducción térmica (Q_cond)
       |        Capa 3         |  v  Mezclado flotante (Q_turb)
       +-----------------------+
       | Capa 4 (Bottom/Cold)  |  <- Entrada/Salida Baja (Retorno)
       +-----------------------+
```



Cada capa $i$ tiene su propia masa $M_i$, entalpía $h_i$ y temperatura $T_i$. El volumen geométrico de cada capa es fijo $display((V_i = V_("tot")/N))$.

=== Modelado de los Fenómenos de Mezclado

Hay tres mecanismos que transfieren energía entre la capa $i$ y la capa $i+1$:

==== Convección Forzada (Desplazamiento Neto de Masa)

Si se inyecta agua en la Capa 4 (abajo) y se extrae agua de la Capa 1 (arriba), se genera un caudal neto vertical ascendente ($W_("net")$) que atraviesa todas las capas intermedias. Cada capa le transfiere masa a la siguiente usando el esquema Upwind: si el flujo sube, la capa $i$ recibe la entalpía de la capa $i+1$.

==== Conducción Térmica Pasiva

Existe un flujo conductivo directo entre capas adyacentes debido a la diferencia de temperatura, que tiende a homogeneizar el tanque lentamente:

$ Q_("cond", i arrow i+1) = (k dot A) / (Delta x) (T_i - T_(i+1)) $

Donde $k$ es la conductividad térmica del agua, $A$ es el área transversal del tanque y $Delta x$ es la distancia entre los centros de las capas.

==== Mezclado por Inversión de Densidad (Penacho Convectivo)

Si $rho_(i) > rho_(i-1)$ (la capa de arriba es más densa que la de abajo), se produce una inestabilidad de Rayleigh-Taylor. En lugar de una conducción simple, modelamos un *caudal de intercambio turbulento* ($W_("turb")$) que mezcla ambas capas rápidamente:

$ W_("turb") = C_("mix") dot A dot sqrt(g dot L_("char") dot (Delta rho / rho)) $

Este caudal extrae masa y energía de ambas capas y las promedia, simulando la caída de penachos fríos.

=== Implementación en Rust (Modelo de 3 Capas Conservativo)

```rust
pub struct StratifiedTank {
    volume_layer: f64, 
    area: f64,
    dx: f64,
    k_water: f64,
    mix_factor: f64,

    // Variables de estado [0: Top, 1: Mid, 2: Bottom]
    // Integración de masa y energía por capa
    pub m: [f64; 3],
    pub u: [f64; 3],

    // Salidas calculadas
    pub p: f64,
    pub h: [f64; 3],
    pub t: [f64; 3],
    pub rho: [f64; 3],
}

impl StratifiedTank {
    pub fn tick(&mut self, w_top: f64, wh_top: f64, w_bot: f64, wh_bot: f64, dt: f64) {
        // 1. BALANCES DE MASA Y ENERGÍA
        let w_net = w_top; 

        for i in 0..3 {
            // Flujos convectivos internos (entre capas, Upwind)
            // (Simplificado para ilustración)
            let _wh_01 = if w_net > 0.0 { w_net * self.h[1] } else { w_net * self.h[0] };
            
            // Intercambio por conducción y mezcla turbulenta
            let _q_mix_01 = self.calc_layer_exchange(0, 1);

            // Integración de m[i] y u[i]...
        }

        self.refresh_properties();
    }

    fn calc_layer_exchange(&self, i: usize, j: usize) -> f64 {
        let q_cond = (self.k_water * self.area / self.dx) * (self.t[i] - self.t[j]);

        // Si la densidad de la capa superior (i) es mayor que la inferior (j)
        if self.rho[i] > self.rho[j] {
            let w_turb = self.mix_factor * self.area * (self.rho[i] - self.rho[j]).sqrt();
            let q_turb = w_turb * (self.h[i] - self.h[j]);
            return q_cond + q_turb;
        }
        q_cond
    }

    fn refresh_properties(&mut self) {
        for i in 0..3 {
            self.rho[i] = self.m[i] / self.volume_layer;
            let u_spec = self.u[i] / self.m[i];
            
            let (p, h, t) = thermo::get_state_from_rho_u(self.rho[i], u_spec);
            self.h[i] = h;
            self.t[i] = t;
        }
        // p se actualiza según la red de presión
    }
}
```
