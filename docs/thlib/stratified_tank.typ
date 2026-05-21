
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

==== Mezclado por Inestabilidad del Flujo (Inversión de Densidad)

Si ingresa agua fría en la capa superior o agua caliente en la inferior, el sistema se vuelve hidrodinámicamente inestable. El agua pesada cae y la liviana sube, generando mezclado turbulento macroscópico.

Modelado: Si $T_(i+1) > T_i$ (la capa de abajo está más caliente que la de arriba), calculamos un coeficiente de mezclado masivo ($k_("mix")$) que transfiere energía de forma casi instantánea hasta que las densidades se ordenen.

=== Implementación en Rust (Modelo de 3 Capas)

```rust

pub struct StratifiedTank {
    volume_layer: f64, // Volumen de cada capa (m³)
    area: f64,         // Área transversal (m²)
    dx: f64,           // Distancia entre centros de capas (m)

    // Variables de estado [0: Top, 1: Mid, 2: Bottom]
    pub p: f64,
    pub h: [f64; 3],
    pub t: [f64; 3],
    pub rho: [f64; 3],

    // Parámetros de mezcla
    k_water: f64,      // Conductividad térmica del agua (~0.6 W/mK)
    mix_factor: f64,   // Multiplicador de mezcla turbulenta por inversión
}

impl StratifiedTank {
    pub fn new(total_volume: f64, area: f64, initial_p: f64, initial_t: f64) -> Self {
        let vol_layer = total_volume / 3.0;
        let dx = vol_layer / area;

        Self {
            volume_layer: vol_layer,
            area,
            dx,
            p: initial_p,
            h: [4184.0 * (initial_t - 273.15); 3],
            t: [initial_t; 3],
            rho: [1000.0; 3],
            k_water: 0.6,
            mix_factor: 50.0,
        }
    }

    pub fn tick(&mut self, w_top: f64, wh_top: f64, w_bot: f64, wh_bot: f64, dt: f64) {
        let w_internal = w_top;

        let q_inter_01 = self.calc_layer_exchange(0, 1);
        let q_inter_12 = self.calc_layer_exchange(1, 2);

        // --- CAPA 0 (TOP) ---
        let mass_0 = self.volume_layer * self.rho[0];
        let conv_01 = if w_internal > 0.0 { -w_internal * self.h[0] } else { -w_internal * self.h[1] };
        let dh0_dt = (wh_top + conv_01 - q_inter_01) / mass_0;

        // --- CAPA 1 (MID) ---
        let mass_1 = self.volume_layer * self.rho[1];
        let conv_10 = if w_internal > 0.0 { w_internal * self.h[0] } else { w_internal * self.h[1] };
        let conv_12 = if w_internal > 0.0 { -w_internal * self.h[1] } else { -w_internal * self.h[2] };
        let dh1_dt = (conv_10 + conv_12 + q_inter_01 - q_inter_12) / mass_1;

        // --- CAPA 2 (BOTTOM) ---
        let mass_2 = self.volume_layer * self.rho[2];
        let conv_21 = if w_internal > 0.0 { w_internal * self.h[1] } else { w_internal * self.h[2] };
        let dh2_dt = (wh_bot + conv_21 + q_inter_12) / mass_2;

        self.h[0] += dh0_dt * dt;
        self.h[1] += dh1_dt * dt;
        self.h[2] += dh2_dt * dt;

        self.refresh_properties();
    }

    fn calc_layer_exchange(&self, i: usize, j: usize) -> f64 {
        let q_cond = (self.k_water * self.area / self.dx) * (self.t[i] - self.t[j]);

        let q_mix = if self.t[j] > self.t[i] {
            (self.t[j] - self.t[i]) * (self.k_water * self.area / self.dx) * self.mix_factor
        } else {
            0.0
        };

        q_cond - q_mix
    }

    fn refresh_properties(&mut self) {
        for i in 0..3 {
            self.t[i] = 273.15 + (self.h[i] / 4184.0);
            self.rho[i] = 1000.0 * (1.0 - 0.0002 * (self.t[i] - 293.15));
        }
    }
}
```
