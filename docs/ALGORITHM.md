# Algoritmo de Resolución del Orden de Ejecución

Este documento explica cómo el sistema determina el orden en que deben ejecutarse los bloques durante cada paso de la simulación para garantizar que las señales se propaguen correctamente.

## El Problema de la Dependencia

En una simulación estilo Simulink, algunos bloques tienen lo que se denomina **Direct Feedthrough** (Transmisión Directa). Esto significa que su salida en el tiempo *t* depende directamente de su entrada en el mismo instante *t*.

Ejemplos:
- **Gain (Ganancia)**: $y(t) = k \cdot u(t)$. La salida $y$ no puede calcularse hasta que la entrada $u$ sea conocida.
- **Integrator (Integrador)**: $y(t) = x(t)$. La salida depende del estado interno acumulado, no directamente de la entrada $u(t)$ actual. Por tanto, su salida puede calcularse al inicio del paso, incluso si la entrada aún no es conocida.

## El Algoritmo: Ordenación Topológica (Kahn)

Para resolver el orden, utilizamos una variante del **Algoritmo de Kahn** para la ordenación topológica de un Grafo Acíclico Dirigido (DAG).

### Pasos del Algoritmo

1.  **Construcción del Grafo de Dependencias**:
    - Solo las conexiones que van hacia un puerto de un bloque con `has_direct_feedthrough() == true` se consideran dependencias estrictas para el paso actual.
    - Si el bloque de destino *no* tiene direct feedthrough, puede ejecutarse en cualquier momento (usualmente al principio) porque su salida ya está disponible (basada en su estado anterior).

2.  **Cálculo de Grados de Entrada (In-degree)**:
    - Para cada bloque, contamos cuántas dependencias estrictas tiene (cuántos bloques con direct feedthrough dependen de otros bloques que aún no han calculado su salida).

3.  **Cola de Listos**:
    - Insertamos en una cola todos los bloques que tienen un grado de entrada de 0 (aquellos que no dependen de nadie o cuya salida no depende de su entrada actual, como los integradores).

4.  **Procesamiento Iterativo**:
    - Mientras la cola no esté vacía:
        1. Extraemos un bloque `u` de la cola.
        2. Lo añadimos al orden de ejecución final.
        3. Para cada bloque `v` que dependía de `u`:
            - Reducimos el grado de entrada de `v`.
            - Si el grado de entrada de `v` llega a 0, lo añadimos a la cola.

## Detección de Bucles Algebraicos

Si al finalizar el algoritmo el número de bloques en el orden de ejecución es menor que el número total de bloques en el sistema, significa que existe un **Bucle Algebraico**.

Un bucle algebraico ocurre cuando hay una cadena cerrada de dependencias circulares donde todos los bloques involucrados tienen `direct feedthrough`. 

**Ejemplo de error**:
`Gain1 -> Gain2 -> Gain1`
Ninguno de los dos puede empezar porque ambos esperan la salida del otro en el mismo instante de tiempo. El algoritmo detecta esto porque ambos bloques mantendrán un grado de entrada > 0 y nunca entrarán en la cola de procesamiento.

## Integración con el Solver

El `EulerSolver` utiliza este orden precalculado en cada paso (`step`):
1.  Sigue el orden topológico para llamar a `outputs()`.
2.  Como los integradores están al principio del orden (grado de entrada 0), su salida (estado actual) está disponible inmediatamente para los bloques `Gain` que dependan de ellos.
3.  Una vez que todas las señales han fluido por el sistema, el solver calcula las `derivatives()` y actualiza los estados para el siguiente paso.
