#import "theme.typ": nota

== Ejemplo: Circuito de Control Térmico Industrial

Este ejemplo demuestra la capacidad de la librería para modelar un sistema completo que integra hidráulica, termodinámica y control automático. Se simula un lazo de enfriamiento/calentamiento donde una bomba impulsa fluido a través de una tubería calefaccionada bajo un lazo de control proporcional.

=== Esquema del Sistema

```
      [Fuente 20°C]
          |
          v
    [CentrifugalPump] --- (RPM: 1500)
          |
          v
      [Header Mid] (1.0 m3)
          |
          v
    [Pipe1D Heater] <--- (Control Q_ext)
          |
          v
      [Header Hot] (1.0 m3) --- [Sensor T_hot]
          |                         |
          v                         | (Feedback)
    [Descarga/Sink]                 |
                                    v
                            [Controlador P] <--- [Setpoint: 60°C]
```

=== Componentes y Parámetros

==== 1. Fuentes de Borde
- *Presión de Succión:* Constant 1.0 bar ($10^5$ Pa).
- *Temperatura de Entrada:* Constant 20°C.

==== 2. Bomba Centrífuga (CentrifugalPump)
- *Curva de Presión:* $"dP" ["Pa"] = 0.2222 dot n^2 - 20 dot w^2$
- *Inercia Geométrica:* 0.01 m.
- *Fricción Pasiva:* 0.1.
- *Velocidad:* 1500 RPM.

==== 3. Tubería de Calentamiento (Pipe1D)
- *Discretización:* 5 celdas.
- *Geometría:* 5.0m de longitud, 0.1m de diámetro.
- *Calor Externo:* Recibe la señal del controlador (vatios).

==== 4. Nodos de Acumulación (Header)
- *Headers (Mid y Hot):* Volumen de 1.0 m³ cada uno.
- *Rol:* Actúan como capacitancias numéricas para estabilizar las ondas de presión y puntos de mezcla de entalpía.

=== Lazo de Control

Se utiliza un *Controlador Proporcional (P)* para regular la temperatura del `Header Hot`.

1. *Error de Temperatura ($e$):* $T_("set") - T_("hot")$.
2. *Potencia del Calefactor ($Q$):* $K_p dot e$.
3. *Ganancia ($K_p$):* $10000 text( W/°C)$.

#nota[
  *Análisis de Resultados:* Debido a que el control es puramente proporcional, el sistema presenta un *error de estado estacionario*. En la simulación, la temperatura se estabiliza cerca de los 39°C en lugar de los 60°C nominales, lo cual es físicamente correcto para este tipo de controlador ante una carga de flujo constante.
]

=== Estabilidad Numérica

El ejemplo utiliza el integrador *RK45* con un paso de sincronismo de 10 segundos. El motor adapta el paso interno para manejar la rigidez de las ecuaciones de momento de la bomba y los balances térmicos, garantizando una simulación libre de oscilaciones espurias.
