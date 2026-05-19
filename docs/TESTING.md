# Guía de Tests

Este documento explica cómo ejecutar los tests del proyecto y describe cada uno de ellos.

---

## Cómo ejecutar los tests

### Suite completa

```bash
cargo test
```

Esto ejecuta tests unitarios, de integración y E2E en un solo comando.

### Solo tests unitarios (librería)

```bash
cargo test --lib
```

Incluye los tests de `src/solver.rs` y `src/system.rs`.

### Solo tests E2E

```bash
cargo test --test e2e_simulation
```

### Tests E2E con salida detallada

```bash
cargo test --test e2e_simulation -- --nocapture
```

Muestra el nombre de cada fixture, archivo, puntos generados y tiempo final.

### Un test E2E específico

Agregá fixtures nuevos en `tests/e2e/<categoria>/` y se ejecutan automáticamente. Para filtrar por nombre:

```bash
cargo test --test e2e_simulation feedback
```

---

## Tests unitarios

Ubicados en `src/solver.rs` y `src/system.rs`, corren con `cargo test --lib`.

### `test_execution_order_simple_chain`

- **Archivo**: `src/system.rs`
- **Qué valida**: El orden topológico de ejecución para una cadena de bloques con feedthrough directo.
- **Sistema**: Constant → Gain → Integrator → Gain
- **Esperado**: Orden `[Constant, Gain, Integrator, Gain]` — los bloques con feedthrough directo (Gain) se ejecutan después de sus fuentes.

### `test_constant_block`

- **Archivo**: `src/system.rs`
- **Qué valida**: Que un bloque `Constant` produce la salida esperada sin depender de entradas.
- **Sistema**: Bloque Constant con valor `[42.0]`.
- **Esperado**: `outputs()` devuelve `[42.0]`.

### `test_simulation_feedback_loop`

- **Archivo**: `src/solver.rs`
- **Qué valida**: Un lazo de realimentación continua con Euler.
- **Sistema**: Integrator (ic=10) → Gain (k=-5) → Integrator
- **Params**: `dt=0.001`, `t_final=1.0`
- **Esperado**: Estado final < 0.1 (decaimiento exponencial, solución analítica: 10·e^(−5) ≈ 0.067).

---

## Tests E2E (End-to-End)

Ubicados en `tests/e2e_simulation.rs`. Cargan fixtures JSON desde `tests/e2e/`, construyen el sistema completo, ejecutan la simulación y validan resultados contra expectativas declarativas.

### Formato de un fixture E2E

Cada archivo `.json` en `tests/e2e/` tiene esta estructura:

```json
{
  "description": "Qué hace este test",
  "system": { ... },
  "params": { "dt": 0.01, "t_final": 1, "solver": "Euler" },
  "expect": {
    "min_points": 100,
    "final_t": 1.0,
    "final_state_lt": 0.1
  }
}
```

#### Campos de `expect`

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `min_points` | `usize` | Mínimo de puntos que debe generar la simulación |
| `final_t` | `f64` | Tiempo final esperado (con tolerancia `dt`) |
| `final_state_lt` | `Option<f64>` | Para tests continuos: el estado final debe ser menor que este valor |
| `check_t_1_5` | `Option<PointCheck>` | Verifica el valor de un bloque en `t ≈ 1.5` |
| `check_t_0_5_int_approx_0_5` | `Option<bool>` | Verifica que el integrador ≈ 0.5 en t=0.5 |
| `check_t_1_int_approx_1` | `Option<bool>` | Verifica que el integrador ≈ 1.0 en t=1.0 |

`PointCheck`:
```json
{ "block": "ud1", "output": 0, "value": 1.0 }
```

---

### Fixtures existentes

#### 1. Continuous — Feedback Loop

- **Archivo**: `tests/e2e/continuous/feedback_loop.json`
- **Solver**: `Euler`
- **Sistema**:
  ```
  Integrator (ic=10) → Gain (k=-5) → Integrator
  ```
- **Descripción**: Lazo de realimentación negativa. La salida del integrador decae exponencialmente: ẋ = −5x.
- **Checks**:
  - `min_points >= 100`
  - `final_t ≈ 1.0`
  - Estado final del integrador < 0.1

#### 2. Discrete — Simple Delay

- **Archivo**: `tests/e2e/discrete/simple_delay.json`
- **Solver**: `Discrete`
- **Sistema**:
  ```
  Step (0→1 @ t=1) → UnitDelay (Ts=0.5)
  ```
- **Descripción**: Un escalón que sube en t=1 pasa por un delay discreto de 0.5s. La salida del delay refleja la entrada con medio segundo de retraso.
- **Checks**:
  - `min_points >= 5`
  - `final_t ≈ 5.0`
  - En t≈1.5, la salida del UnitDelay debe ser 1.0

  **Línea temporal**:

  | t | Step | UnitDelay out |
  |---|------|---------------|
  | 0.0 | 0 | 0 (IC) |
  | 0.5 | 0 | 0 |
  | 1.0 | 1 | 0 |
  | 1.5 | 1 | **1** ← check |
  | 2.0 | 1 | 1 |

#### 3. Hybrid — Integrator + Delay

- **Archivo**: `tests/e2e/hybrid/integrator_delay.json`
- **Solver**: `Hybrid`
- **Sistema**:
  ```
  Step (0→1 @ t=0) → Integrator (ic=0) → UnitDelay (Ts=0.5)
  ```
- **Descripción**: Un escalón unitario desde t=0 alimenta un integrador continuo (rampa ẋ=1), cuya salida es muestreada por un delay discreto cada 0.5s.
- **Checks**:
  - `min_points >= 50`
  - `final_t ≈ 1.0`
  - Integrador ≈ 0.5 en t=0.5 (rampa: ∫1 dt = t)
  - Integrador ≈ 1.0 en t=1.0

  **Línea temporal**:

  | t | Step | Integrator | UnitDelay out |
  |---|------|------------|---------------|
  | 0.0 | 1 | 0.0 (IC) | 0.0 (IC) |
  | 0.1 | 1 | 0.1 | 0.0 |
  | 0.5 | 1 | **0.5** ← check | 0.0 |
  | 0.6 | 1 | 0.6 | 0.5 |
  | 1.0 | 1 | **1.0** ← check | 0.5 |

---

## Cómo agregar un nuevo test E2E

1. Creá un archivo `.json` en `tests/e2e/<categoria>/` (o una subcarpeta nueva).
2. Definí `system`, `params` y `expect`.
3. Ejecutá `cargo test --test e2e_simulation`.

El harness (`e2e_simulation.rs`) recorre recursivamente `tests/e2e/`, carga cada `.json`, construye el `System`, corre la simulación con el solver indicado, y aplica todos los checks del campo `expect`.

Para agregar un nuevo tipo de check, extendé el struct `Expectations` en `tests/e2e_simulation.rs` y agregá la validación correspondiente en la función `e2e_all_fixtures`.

---

## Tests de UI (manuales)

La UI (React + Vite) no tiene tests automatizados. Para verificar cambios visuales:

```bash
cd ui && npm run dev
```

Abrir `http://localhost:5173`, arrastrar bloques, conectar, simular, y verificar el Scope y Dashboard.
