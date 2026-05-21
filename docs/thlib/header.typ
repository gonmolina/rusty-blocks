#import "@preview/gentle-clues:1.3.1": *

== Bloque Header (Volumen de Control Capacitivo)

Este documento detalla la arquitectura, el modelado matemático y la implementación en Rust del componente Header (o nodo volumétrico de acumulación). En una simulación termohidráulica por diagramas de bloques distribuidos, el Header actúa como una capacitancia acoplada. Es el encargado de absorber los cambios bruscos de caudal impuestos por las resistencias (Pipe1D, válvulas, bombas), actuando como un amortiguador numérico que "ablanda" las ecuaciones de presión y energía del sistema discreto.

=== Rol Arquitectónico y Topología

A diferencia del caño discretizado espacialmente, el Header es un modelo de parámetros concentrados (0D).

Variables de Estado: Es el dueño de la presión ($p$), la entalpía ($h$) y la densidad ($rho$) en un punto de conexión común.

Conectividad: Funciona como un nodo central ("hub") al que se pueden conectar $M$ componentes de flujo. No presupone una dirección única; recolecta caudales de masa ($W$) y flujos de energía ($Phi$) netos provenientes de todas sus fronteras conectadas, aplicando una convención de signos estricta: flujo entrante es positivo (+), flujo saliente es negativo (-).

=== Modelado Matemático y Estrategia de Ablandamiento

Para resolver el estado en tiempo discreto sin recurrir a un solver implícito global, el Header expande sus ecuaciones de conservación analíticamente a través de derivadas parciales de la densidad.

==== Conservación de Masa y Energía

Las ecuaciones continuas que gobiernan el volumen fijo $V$ del Header son:

$ (d M) / (d t) = sum W_("in") - sum W_("out") = W_("net") $

$ (d U) / (d t) = sum (W dot h)_("in") - sum (W dot h)_("out") + Q = Phi_("net") + Q $

Para fluidos monofásicos líquidos a presiones moderadas, aproximamos la energía interna total como $U approx M dot h$. Expandiendo la derivada temporal de la energía por la regla del producto:

$ (d U) / (d t) = M (d h) / (d t) + h (d M) / (d t) = (V dot rho) (d h) / (d t) + h dot W_("net") $

Despejando directamente el avance de la entalpía específica:

$ (d h) / (d t) = (Phi_("net") + Q - h dot W_("net")) / (V dot rho) $

==== El Lazo de Presión Ablandada

Para actualizar la presión de forma explícita y estable, expresamos el cambio de densidad en función de sus coordenadas termodinámicas $rho(p, h)$:

$ (d rho) / (d t) = ((partial rho) / (partial p))_h (d p) / (d t) + ((partial rho) / (partial h))_p (d h) / (d t) $

Como $(d rho) / (d t) = W_("net") / V$, podemos sustituir y despejar la derivada temporal de la presión ($(d p) / (d t)$):

$
  (d p) / (d t) = 1 / (V dot ((partial rho) / (partial p))_h) [ W_("net") - V dot ((partial rho) / (partial h))_p (d h) / (d t) ]
$

#tip(
  title: "Interpretación Física",
)[El término $V dot display((partial rho)/(partial h))_p display((d h) / (d t))$ captura la expansión
  térmica confinada. Si el líquido se calienta rápido $(display((d h) / (d t) > 0))$, la densidad tiende a bajar, lo que se traduce en una inyección de presión
  adicional dentro del volumen cerrado.]

#tip(title: "Sintonía Numérica")[
  Multiplicar el término de compresibilidad isotérmica $(display((partial rho)/(partial p))_h)$ por un factor de ganancia virtual mayor a 1actúa como un filtro paso bajo para las ondas de presión. Esto "ablanda" el golpe de ariete numérico y permite correr la simulación con un $Delta t$
  significativamente mayor.]



=== Estructura de Datos en Rust

El bloque almacena su geometría, sus variables de estado integradas y los coeficientes de acoplamiento termodinámico.

```rust
pub struct HeaderBlock {
    // Geometría fija del nodo
    volume: f64,

    // VARIABLES DE ESTADO (Salidas del bloque hacia los caños)
    pub p: f64,   // Presión actual (Pa)
    pub h: f64,   // Entalpía específica actual (J/kg)
    pub rho: f64, // Densidad actual (kg/m³)

    // COEFICIENTES DE ACOPLAMIENTO (Ablandamiento)
    drho_dp_virtual: f64, // (∂ρ/∂p)_h modificado para estabilidad
    drho_dh: f64,         // (∂ρ/∂h)_p real o aproximado
}
```

=== Implementación del Bloque (Método tick)

```rust
impl HeaderBlock {
    pub fn new(volume: f64, initial_p: f64, initial_h: f64) -> Self {
        let mut header = Self {
            volume,
            p: initial_p,
            h: initial_h,
            rho: 1000.0,
            drho_dp_virtual: 4.5e-10 * 1000.0,
            drho_dh: -0.0002 * 1000.0,
        };
        header.refresh_thermo_properties();
        header
    }

    pub fn tick(&mut self, w_net: f64, wh_net: f64, q_externo: f64, dt: f64) {
        let mass = self.volume * self.rho;

        // 1. INTEGRACIÓN DISCRETA DE LA ENERGÍA (Entalpía)
        let net_energy_flow = wh_net + q_externo;
        let dh_dt = (net_energy_flow - (self.h * w_net)) / mass;
        self.h += dh_dt * dt;

        // 2. INTEGRACIÓN DISCRETA DE LA PRESIÓN
        let thermal_expansion_buffer = self.volume * self.drho_dh * dh_dt;
        let dp_dt = (w_net - thermal_expansion_buffer) / (self.volume * self.drho_dp_virtual);
        self.p += dp_dt * dt;

        // 3. SINCRONIZACIÓN TERMODINÁMICA
        self.refresh_thermo_properties();
    }

    #[inline(always)]
    fn refresh_thermo_properties(&mut self) {
        // Enlace real a librería iapws97 u otra tabla de agua líquida
        self.rho = 1000.0;
        let drho_dp_real = 4.5e-10 * self.rho;
        let coef_ablandamiento = 50.0;
        self.drho_dp_virtual = drho_dp_real * coef_ablandamiento;
        self.drho_dh = -0.0002 * self.rho;
    }
}
```



=== Acoplamiento de Frontera y Flujo Bidireccional

El Header no necesita conocer qué componentes están conectados a él. Su interfaz expone de forma pública sus tres salidas actualizadas: p, h y rho. La bidireccionalidad se resuelve en las conexiones gracias al esquema de transporte Upwind implementado en los caños:

Si un caño genera un caudal hacia afuera del Header, el caño lee la entalpía h del Header para calcular la energía que se está llevando.

Si un caño empuja fluido hacia adentro del Header, el caño le inyectará su propia entalpía de salida. El Header recibirá un w_net positivo y un wh_net positivo, absorbiendo la masa y mezclando la energía de forma automática.
