
== Tank Closed (Tanque Presurizado de Nivel Variable)

Un tanque de nivel variable cerrado se diferencia del Header estructuralmente en un aspecto crítico: es un sistema de volumen de fluido variable con interfaz líquido-gas.

Mientras que el Header es un nodo totalmente lleno (monofásico cerrado, donde la presión sube instantáneamente al inyectar masa), el tanque cerrado absorbe masa cambiando su nivel de líquido, comprimiendo o expandiendo el colchón de gas (aire o nitrógeno) atrapado en la parte superior. Por lo tanto, la presión se rige por la ley de los gases ideales y procesos politrópicos.

=== El Modelado Matemático (0D de Dos Zonas)

Dividimos el tanque de volumen total fijo ($V_("tot")$) en dos zonas de volumen variable:

Zona Líquida ($V_L$): Fluido incompresible con densidad $rho_L$.

Zona de Gas ($V_G$): Gas ideal compresible con masa fija $M_G$.

==== Balance de Masa y Geometría

La masa de líquido varía según el caudal neto de los caños conectados ($W_("net")$):

$ (d M_L) / (d t) = W_("net") $

Conocida la masa de líquido, calculamos los volúmenes y el nivel hidrostático ($z$) si el tanque tiene un área transversal constante ($A$):

$V_L = M_L / rho_L$$ z = V_L / A $

$ V_G = V_("tot") - V_L $

==== Dinámica de Presión (Compresión del Gas)

Asumiendo que el gas se comprime de forma adiabática (proceso rápido sin transferencia de calor, $gamma = 1.4$ para el aire) o isotérmica ($gamma = 1.0$), la presión del gas $P_G$ evoluciona como:

$P_G dot V_G^gamma = C => P_G = P_(G,0) dot (V_(G,0) / V_G)^gamma$

==== Presión en el Fondo del Tanque

La presión que "ven" los caños conectados en la base incluye la presión del colchón de gas más el peso de la columna líquida:

$ P_("base") = P_G + rho_L dot g dot z $

=== Estructura de Datos en Rust

```rust
pub struct TankBlock {
    v_total: f64,
    area: f64,
    g: f64,

    // Colchón de gas constante
    m_gas: f64,
    r_gas: f64,
    t_gas: f64,
    gamma: f64,
    p_gas_nominal: f64,
    v_gas_nominal: f64,

    // VARIABLES DE ESTADO INTEGRADAS
    pub m_liq: f64,
    pub h_liq: f64,

    // SALIDAS DEDUCIDAS
    pub p_base: f64,
    pub level: f64,
    pub rho_l: f64,
}
```

=== Implementación del Bloque (tick)

```rust
impl TankBlock {
    pub fn new(v_total: f64, area: f64, initial_level: f64, initial_p_gas: f64, initial_h: f64) -> Self {
        let rho_l = 1000.0;
        let v_liq = initial_level * area;
        let v_gas = v_total - v_liq;
        let m_liq = v_liq * rho_l;

        Self {
            v_total,
            area,
            g: 9.81,
            m_gas: 0.0,
            r_gas: 287.05,
            t_gas: 293.15,
            gamma: 1.4,
            p_gas_nominal: initial_p_gas,
            v_gas_nominal: v_gas,
            m_liq,
            h_liq: initial_h,
            p_base: initial_p_gas + rho_l * 9.81 * initial_level,
            level: initial_level,
            rho_l,
        }
    }

    pub fn tick(&mut self, w_net: f64, wh_net: f64, q_externo: f64, dt: f64) {
        // 1. INTEGRACIÓN DE MASA
        self.m_liq += w_net * dt;
        if self.m_liq < 0.0 { self.m_liq = 0.0; }
        let max_mass = self.v_total * self.rho_l * 0.99;
        if self.m_liq > max_mass { self.m_liq = max_mass; }

        // 2. INTEGRACIÓN DE ENERGÍA
        let dh_dt = (wh_net + q_externo - (self.h_liq * w_net)) / (self.m_liq + 1e-3);
        self.h_liq += dh_dt * dt;

        // 3. ACTUALIZACIÓN DE SALIDAS
        self.refresh_geometry();
    }

    fn refresh_geometry(&mut self) {
        self.rho_l = 1000.0;
        let v_liq = self.m_liq / self.rho_l;
        let v_gas = self.v_total - v_liq;
        self.level = v_liq / self.area;

        // Ley politrópica del gas
        let p_gas = self.p_gas_nominal * (self.v_gas_nominal / v_gas).powf(self.gamma);
        self.p_base = p_gas + self.rho_l * self.g * self.level;
    }
}
```

