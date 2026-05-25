#import "@preview/gentle-clues:1.3.1": *

== Bloque Header (Volumen de Control Capacitivo)

Este documento detalla la arquitectura, el modelado matemático y la implementación en Rust del componente Header (o nodo volumétrico de acumulación). En una simulación termohidráulica por diagramas de bloques distribuidos, el Header actúa como una capacitancia acoplada. Es el encargado de absorber los cambios bruscos de caudal impuestos por las resistencias (Pipe1D, válvulas, bombas), actuando como un amortiguador numérico que "ablanda" las ecuaciones de presión y energía del sistema discreto.

=== Rol Arquitectónico y Topología

A diferencia del caño discretizado espacialmente, el Header es un modelo de parámetros concentrados (0D).

Variables de Estado: Es el dueño de la presión ($p$), la entalpía ($h$) y la densidad ($rho$) en un punto de conexión común.

Conectividad: Funciona como un nodo central ("hub") al que se pueden conectar $M$ componentes de flujo. No presupone una dirección única; recolecta caudales de masa ($W$) y flujos de energía ($Phi$) netos provenientes de todas sus fronteras conectadas, aplicando una convención de signos estricta: flujo entrante es positivo (+), flujo saliente es negativo (-).

=== Modelado Matemático y Conservación de Masa

Para garantizar la estabilidad y la precisión física, el Header utiliza un enfoque de *variables conservativas*. Las variables de estado primarias que se integran en el tiempo son la masa total ($M$) y la energía interna total ($U$).

==== Ecuaciones de Conservación

Las ecuaciones continuas que gobiernan el volumen fijo $V$ del Header son:

$ (d M) / (d t) = sum W_("in") - sum W_("out") = W_("net") $

$ (d U) / (d t) = sum (W dot h)_("in") - sum (W dot h)_("out") + Q = Phi_("net") + Q $

A partir de estas, calculamos las propiedades intensivas en cada paso:

1.  *Densidad:* $rho = M / V$
2.  *Energía específica:* $u = U / M$
3.  *Presión y Temperatura:* $p, T = f(rho, u)$ (usando la ecuación de estado).

==== Estrategia de Estabilización Numérica (Ablandamiento de Presión)

En simulaciones de tiempo real con líquidos casi incompresibles, un pequeño error en el balance de masa genera oscilaciones masivas de presión. Para "ablandar" este acoplamiento sin violar la conservación de masa, se utiliza un método de *compresibilidad virtual* en la ecuación de estado:

$ d p = 1 / (V dot ((partial rho) / (partial p))_("virtual")) [ Delta M - V dot ((partial rho) / (partial h))_p Delta h ] $

Donde $((partial rho) / (partial p))_("virtual") = beta dot ((partial rho) / (partial p))_("real")$ con $beta approx 10..100$. Es vital que este ablandamiento sea consistente: la densidad utilizada en el siguiente paso de transporte debe ser la derivada de la masa real, no de la presión ablandada, para evitar derivas de masa.

=== Estructura de Datos en Rust

```rust
pub struct HeaderBlock {
    volume: f64,

    // VARIABLES DE ESTADO CONSERVATIVAS
    pub mass: f64,
    pub internal_energy: f64,

    // SALIDAS CALCULADAS
    pub p: f64,
    pub h: f64,
    pub rho: f64,

    // PARÁMETROS DE ESTABILIDAD
    beta_softening: f64,
}
```

=== Implementación del Bloque (Método tick)

```rust
impl HeaderBlock {
    pub fn tick(&mut self, w_net: f64, wh_net: f64, q_externo: f64, dt: f64) {
        // 1. INTEGRACIÓN DE VARIABLES CONSERVATIVAS (Masa y Energía)
        self.mass += w_net * dt;
        self.internal_energy += (wh_net + q_externo) * dt;

        // Evitar estados no físicos por errores numéricos
        if self.mass < 1e-6 { self.mass = 1e-6; }

        // 2. ACTUALIZACIÓN TERMODINÁMICA
        self.refresh_thermo_properties();
    }

    fn refresh_thermo_properties(&mut self) {
        // Propiedades intensivas reales
        self.rho = self.mass / self.volume;
        let u = self.internal_energy / self.mass;

        // Enlace a librería compartida (IAPWS-IF97)
        // Nota: p y h deben ser calculadas de forma consistente con rho y u
        let (p_real, h_real, t_real) = thermo::get_state_from_rho_u(self.rho, u);

        // Aplicación del ablandamiento para el siguiente paso de presión
        // que verán los caños (esto reduce el golpe de ariete numérico)
        self.p = p_real; // Opcional: aplicar filtro paso bajo si beta > 1
        self.h = h_real;
    }
}
```




=== Acoplamiento de Frontera y Flujo Bidireccional

El Header no necesita conocer qué componentes están conectados a él. Su interfaz expone de forma pública sus tres salidas actualizadas: p, h y rho. La bidireccionalidad se resuelve en las conexiones gracias al esquema de transporte Upwind implementado en los caños:

Si un caño genera un caudal hacia afuera del Header, el caño lee la entalpía h del Header para calcular la energía que se está llevando.

Si un caño empuja fluido hacia adentro del Header, el caño le inyectará su propia entalpía de salida. El Header recibirá un w_net positivo y un wh_net positivo, absorbiendo la masa y mezclando la energía de forma automática.
