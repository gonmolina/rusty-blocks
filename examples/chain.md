# Ejemplo: Cadena de Bloques (Feedforward Chain)

Este ejemplo muestra cómo fluye una señal de un bloque a otro en una secuencia lineal sin realimentación.

## Estructura del Sistema

- **Integrador (i1)**: Con una condición inicial de 1.0. Su salida crece linealmente si tiene entrada, pero aquí es constante al principio.
- **Ganancia 1 (g1)**: Multiplica la salida del integrador por 2.0.
- **Ganancia 2 (g2)**: Multiplica la salida de g1 por 0.5.

## Propósito

Este ejemplo es ideal para verificar:
- El orden de ejecución topológico (primero el integrador, luego g1, luego g2).
- Que las multiplicaciones secuenciales den el resultado esperado (en este caso, la salida de g2 es igual a la salida de i1 ya que $2.0 \cdot 0.5 = 1.0$).

## Cómo Simularlo

```bash
cargo run -- examples/chain_system.json
```
