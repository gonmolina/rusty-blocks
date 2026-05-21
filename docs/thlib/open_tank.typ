#import "@preview/gentle-clues:1.3.1": *

== Open Tank (Tanque de Presión Hidrostática)

Un tanque abierto (o pileta de nivel variable) es la simplificación más noble del bloque volumétrico. Al estar en contacto directo con la atmósfera, la cámara de gas desaparece de las ecuaciones porque su presión ya no depende de la compresión del volumen: la presión en la superficie es una constante fija ($P_("atm")$).

Esto significa que el colchón de aire ya no actúa como un resorte neumático. Toda la variación de presión en el fondo del del tanque se debe única y exclusivamente al peso de la columna de líquido (la altura del nivel).

=== El Modelado Matemático

El modelo se reduce a la conservación de masa y al cálculo de la presión hidrostática pura.

==== Balance de Masa y Nivel

La masa acumulada ($M_L$) sigue gobernada por la suma algebraica de los caudales de los caños conectados ($W_("net")$):

$M_L^(n+1) = M_L^n + W_("net") dot Delta t$

A partir de la masa y la densidad actual del líquido ($rho_L$), determinamos el volumen y la altura del nivel ($z$):

$V_L = M_L / rho_L$$ z = V_L / A $

==== Ecuación de Presión en la Base

La presión absoluta en el fondo del tanque que "ven" los caños acoplados es simplemente:

$P_("base") = P_("atm") + rho_L dot g dot z$

#info[
  En simulaciones numéricas, un tanque abierto es el estabilizador definitivo. Al fijar $P_("surface") = P_("atm")$, el sistema tiene un nodo de presión de referencia constante. Esto evita que las presiones del sistema floten o diverjan sin control numérico.
]

=== Estructura de Datos en Rust

```rust
pub struct OpenTankBlock {
    // Geometría fija
    area: f64,    // Área transversal (m²)
    g: f64,       // Gravedad (m/s²)
    p_atm: f64,   // Presión atmosférica local (Pa)

    // VARIABLES DE ESTADO INTEGRADAS
    pub m_liq: f64, // Masa de líquido actual (kg)
    pub h_liq: f64, // Entalpía específica del líquido (J/kg)

    // SALIDAS DEDUCIDAS
    pub p_base: f64, // Presión en el fondo (Pa)
    pub level: f64,  // Nivel de líquido (m)
    pub rho_l: f64,  // Densidad del líquido (kg/m³)
}
```

=== Implementación del Bloque (tick)

```rust
impl OpenTankBlock {
    pub fn new(area: f64, initial_level: f64, p_atm: f64, initial_h: f64) -> Self {
        let rho_l = 1000.0;
        let v_liq = initial_level * area;
        let m_liq = v_liq * rho_l;

        let mut tank = Self {
            area,
            g: 9.80665,
            p_atm,
            m_liq,
            h_liq: initial_h,
            p_base: p_atm + rho_l * 9.80665 * initial_level,
            level: initial_level,
            rho_l,
        };
        tank.refresh_geometry();
        tank
    }

    pub fn tick(&mut self, w_net: f64, wh_net: f64, q_externo: f64, dt: f64) {
        // 1. INTEGRACIÓN DE MASA
        self.m_liq += w_net * dt;
        if self.m_liq < 0.0 { self.m_liq = 0.0; }

        // 2. INTEGRACIÓN DE ENERGÍA
        let dh_dt = (wh_net + q_externo - (self.h_liq * w_net)) / (self.m_liq + 1e-3);
        self.h_liq += dh_dt * dt;

        // 3. ACTUALIZACIÓN DE SALIDAS
        self.refresh_geometry();
    }

    fn refresh_geometry(&mut self) {
        self.rho_l = 1000.0; // Enlace simplificado
        let v_liq = self.m_liq / self.rho_l;
        self.level = v_liq / self.area;
        self.p_base = self.p_atm + self.rho_l * self.g * self.level;
    }
}
```


=== Particularidades en la Red

Desborde Teórico: Si la masa inyectada supera la capacidad física, el modelo aumentará el level indefinidamente. Si se desea modelar el derrame, se debe limitar el level a la altura constructiva máxima en refresh_geometry() y reportar la masa perdida fuera del balance.

Evaporación y Pérdidas Térmicas ($Q_("externo")$): Los tanques abiertos tienen gran superficie de intercambio. El término $Q_("externo")$ permite simular la pérdida de calor por convección con el aire o radiación solar.
