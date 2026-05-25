#import "theme.typ": nota

== Integración con el Motor de Diagramas de Bloques

Este documento describe cómo la biblioteca termohidráulica (`thlib`) se acopla al motor de ejecución de bloques del proyecto `bloques`. La integración se basa en un esquema de *co-simulación de parámetros concentrados y distribuidos* utilizando una arquitectura de puertos de potencia.

=== El Paradigma de Acoplamiento (Esfuerzo-Flujo)

Para mantener la modularidad, los bloques termohidráulicos se dividen en dos categorías siguiendo la teoría de *Bond Graphs*:

1. *Capacitancias (Headers, Tanks):*
   - *Entradas:* Caudal de masa ($W$) y Caudal de energía ($W dot h$) desde los puertos.
   - *Salidas:* Estado termodinámico (Presión $P$, Entalpía $h$, Densidad $rho$).
   - *Rol:* Actúan como nodos de estado.

2. *Resistencias (Pipes, Bombas, Válvulas):*
   - *Entradas:* Estados de los nodos fronterizos ($P_("in")$, $P_("out")$, $h_("in")$, $h_("out")$).
   - *Salidas:* Caudales resultantes ($W$, $W dot h$).
   - *Rol:* Actúan como conductores de flujo.

=== Mapeo de Puertos en el Sistema `bloques`

Cada componente de `thlib` debe envolverse en un `Block` de Rust que implemente la trait `SimulationBlock`.

```rust
pub struct ThPipeBlock {
    inner: Pipe1D,
}

impl SimulationBlock for ThPipeBlock {
    fn tick(&mut self, inputs: &[Value], outputs: &mut [Value], dt: f64) {
        // 1. Extraer presiones y entalpías de los puertos de entrada
        let p_in = inputs[0].as_f64();
        let h_in = inputs[1].as_f64();
        let p_out = inputs[2].as_f64();
        let h_out = inputs[3].as_f64();

        // 2. Ejecutar la física del componente
        let (w_in, w_out) = self.inner.tick(p_in, h_in, p_out, h_out, dt);

        // 3. Escribir caudales en los puertos de salida
        outputs[0] = Value::F64(w_in);
        outputs[1] = Value::F64(w_out);
    }
}
```

=== Orden de Ejecución y Estabilidad Numérica

Dado que el motor de `bloques` utiliza una ejecución secuencial, el orden de los bloques en el grafo es crítico para evitar el retraso de un paso de tiempo ($z^-1$) innecesario que podría desestabilizar las ondas de presión.

#nota[
  *Orden Recomendado:*
  1. *Fuentes:* Actualizar señales de control (ej. velocidad de bomba, apertura de válvulas).
  2. *Resistencias:* Calcular caudales basados en las presiones del paso anterior.
  3. *Capacitancias:* Integrar masas y energías basadas en los nuevos caudales.
  4. *Sinks:* Registrar resultados.
]

=== Manejo de la Red de Presión (Algebraic Loops)

En sistemas puramente incompresibles, la presión es una variable algebraica que se transmite instantáneamente. Sin embargo, en nuestro motor:
- Utilizamos *Capacitancias Numéricas* (Headers) para romper los lazos algebraicos y desacoplar las ecuaciones de momento.
- Cada conexión entre dos resistencias *debe* pasar obligatoriamente por un Header o Tanque.
- *Prohibición de Conexión Directa:* No se permite la conexión directa entre dos componentes de tipo resistencia (ej. Pipe1D -> Pipe1D). El orquestador de bloques debe validar la topología y emitir un error de compilación del grafo si detecta una conexión de este tipo, obligando al usuario a insertar un nodo capacitivo intermedio para garantizar la estabilidad física.


=== Sincronización Termodinámica Global

Para evitar inconsistencias, todos los bloques deben acceder a una única instancia (o implementación estática) de la librería de propiedades. En el sistema de bloques, esto se maneja mediante un `Resource` compartido:

```rust
// Ejemplo de acceso en el tick
let thermo = ctx.get_resource::<ThermoLibrary>();
let (p, h, t) = thermo.get_state_from_rho_u(rho, u);
```
