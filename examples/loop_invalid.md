# Ejemplo: Bucle Algebraico (Error)

Este ejemplo está diseñado para demostrar la capacidad del simulador para detectar errores de arquitectura del sistema.

## El Problema

Un **Bucle Algebraico** ocurre cuando una salida depende directamente de una entrada que, a su vez, depende de la salida original sin ningún bloque dinámico (como un integrador) entre medio que rompa la dependencia inmediata en el tiempo.

```text
Ganancia A -> Ganancia B -> Ganancia A
```

Como tanto la ganancia A como la ganancia B tienen **Direct Feedthrough**, el solver no puede determinar quién debe calcular su salida primero.

## Comportamiento Esperado

El programa detectará el bucle algebraico durante el proceso de ordenación topológica y lanzará un mensaje de error:
`Error: Algebraic loop detected!`

## Cómo Probarlo

```bash
cargo run -- examples/loop_invalid_system.json
```
