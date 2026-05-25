
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

==== Dinámica de Energía (Balance Térmico con Trabajo de Presión)

Para el volumen de líquido, la evolución de la entalpía debe considerar no solo los flujos de masa sino también el trabajo realizado por el cambio de presión (especialmente relevante en tanques presurizados):

$ M_L (d h_L) / (d t) = Phi_("net") + Q + V_L (d p) / (d t) $

Donde $V_L (d p) / (d t)$ es el término de trabajo de flujo/compresión. En líquidos, este término suele ser pequeño pero es necesario para la consistencia energética total del sistema.

=== Implementación del Bloque (tick)

```rust
impl TankBlock {
    pub fn tick(&mut self, w_net: f64, wh_net: f64, q_externo: f64, dt: f64) {
        // 1. INTEGRACIÓN DE MASA
        self.m_liq += w_net * dt;
        
        // Límites físicos
        self.m_liq = self.m_liq.clamp(0.0, self.v_total * self.rho_l * 0.99);

        // 2. CÁLCULO DE PRESIÓN (Necesario para el término de trabajo)
        let p_base_old = self.p_base;
        self.refresh_geometry(); // Actualiza p_base, level, rho_l
        let dp_dt = (self.p_base - p_base_old) / dt;

        // 3. INTEGRACIÓN DE ENERGÍA (Incluyendo trabajo de presión)
        // Usamos la masa promediada o actual para estabilidad
        let work_term = (self.m_liq / self.rho_l) * dp_dt;
        let dh_dt = (wh_net + q_externo - (self.h_liq * w_net) + work_term) / (self.m_liq + 1e-3);
        self.h_liq += dh_dt * dt;
    }

    fn refresh_geometry(&mut self) {
        // Enlace a librería de propiedades compartida
        self.rho_l = thermo::get_rho_from_p_h(self.p_base, self.h_liq);
        
        let v_liq = self.m_liq / self.rho_l;
        let v_gas = self.v_total - v_liq;
        self.level = v_liq / self.area;

        // Ley politrópica del colchón de gas
        let p_gas = self.p_gas_nominal * (self.v_gas_nominal / v_gas).powf(self.gamma);
        self.p_base = p_gas + self.rho_l * self.g * self.level;
    }
}
```


