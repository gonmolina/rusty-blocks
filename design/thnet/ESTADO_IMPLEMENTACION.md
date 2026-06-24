# THNet — Análisis: Diseño vs. Implementación

**Fuentes:**
- Diseño: [ARCHITECTURE.md](ARCHITECTURE.md) · [MATH_SOLVER.md](MATH_SOLVER.md)
- Implementación: [network.rs](../../src/thnet/network.rs) · [solver.rs](../../src/thnet/solver.rs) · [thermo.rs](../../src/thnet/thermo.rs)
- Ejemplos: [dos_tanques.rs](../../examples/thnet/dos_tanques.rs) · [tanque_estratificado.rs](../../examples/thnet/tanque_estratificado.rs) · [bomba_valvula_caño.rs](../../examples/thnet/bomba_valvula_caño.rs) · [conv_natural_thnet.rs](../../examples/thnet/conv_natural_thnet.rs)

---

## 1. Resumen Ejecutivo

La implementación cubre el **núcleo matemático del diseño de forma fiel**, pero colapsa varios componentes separados del diseño en estructuras monolíticas, y agrega física más rica que la especificada (inercia térmica de pared, solver de pared). Las capas de abstracción del diseño (traits, módulos separados por componente) **no se implementaron**.

| Categoría | Estado |
|---|---|
| Matemática hidráulica (C·P=b) | ✅ Implementada fielmente |
| Newton-Raphson interno | ✅ Implementado |
| Transporte térmico upwind | ✅ Implementado, mejorado |
| Churchill friction factor | ✅ Implementado |
| Bomba centrífuga (curva HQ) | ✅ Implementada (curva cuadrática con velocidad variable y leyes de afinidad) |
| Válvula de control (Kv) | ✅ Implementada (soporta coeficientes Kv y Cv, y curvas Lineal, Equal% y QuickOpening) |
| Check valve | ✅ Implementada |
| OpenTank (nivel dinámico) | ✅ Implementada como componente |
| ClosedTank | ✅ Implementada como componente (modelo gas cushion y compresibilidad de líquido) |
| StratifiedTank | ✅ Encapsulada como componente |
| HeatExchanger NTU-ε completo | ✅ Implementado como componente de dos fluidos (contracorriente) |
| Traits NetworkNode / NetworkBranch | ✅ Implementados |
| Estructura modular (graph.rs, builder.rs) | ✅ Implementada |
| Solver Gauss-Seidel/SOR | ✅ Implementado (configurable con SOR de omega variable) |
| Detector de redes desconectadas | ✅ Implementado (dinámico en cada paso) |
| Output CSV | ✅ Implementado (`CsvRecorder` tipado) |
| Inercia térmica de pared (wall model) | ✅ **Agregado** (documentado en MATH_SOLVER §4.9) |
| Temperatura fija por nodo (Dirichlet térmico) | ✅ **Agregado** (no estaba en diseño) |

---

## 2. Lo que Está Implementado y Coincide con el Diseño ✅

### 2.1 Núcleo Hidráulico — Sistema C·P=b

La matemática del [MATH_SOLVER.md §1.4](MATH_SOLVER.md) está implementada correctamente en `solver.rs` (líneas 94–213):

```
Gj = Δt / (Ij + Δt·Rj)
Sj = (Ij·Wj + Δt·(ΔPgrav + ΔPbomba)) / (Ij + Δt·Rj)
C[i,i] += Gj    C[i,l] -= Gj    b[i] ±= Sj
```

La condición de Dirichlet se maneja correctamente: presiones fijas se mueven al RHS (b), eliminando sus filas/columnas de C.

### 2.2 Newton-Raphson

El diseño especifica MAX_ITER=20, TOL=1e-6 Pa.
La implementación usa `max_newton_iter=50`, `tol_flow=1e-9 kg/s` (criterio más estricto, en unidades de caudal). Funciona correctamente.

> **Nota:** El diseño mide residual en **presión** (Pa), la implementación lo mide en **caudal** (kg/s). Ambos son válidos pero no son comparables directamente.

### 2.3 Factor de Fricción de Churchill (1977)

Implementado en `network.rs` con las fórmulas exactas del diseño. Incluye el tratamiento laminar (Hagen-Poiseuille, f=64/Re) para Re<2300.

### 2.4 Transporte Térmico Upwind

El esquema upwind implícito en espacio (space-marching) está implementado. Es **más avanzado** que el diseño: el diseño especifica explícito, la implementación usa upwind implícito:

```
h_new = (h_cell + CFL·h_in + NTU·h_cool) / (1 + CFL + NTU)
```

Esto elimina incluso la restricción CFL térmica, haciendo el esquema **incondicionalmente estable**.

### 2.5 Inercia Hidráulica

El diseño define `I = ρ·L/A`, la implementación usa `I = L/A` (sin ρ). La densidad se incorpora implícitamente en otras partes del cálculo. Esto es consistente.

### 2.6 Caída de Presión Gravitacional

Implementada correctamente como `ΔPgrav = -ρ·g·dz`, consistente con el diseño.

### 2.7 Propiedades Termofísicas (thermo.rs)

Cubre lo especificado:
- `density(T)` — polinomio Kell 1975 de 5° grado ✅
- `viscosity(T)` — exponencial ✅
- `enthalpy(T)` — integral de cp ✅
- `cp(T)` — lineal en T ✅
- `temperature_from_enthalpy(h)` — inversión Newton ✅

El diseño menciona "tabla IF97 o lineal" — la implementación usa polinomiales de alta precisión, más que lineal pero menos que IF97 completo.

---

## 3. Diferencias Importantes ⚠️

### 3.1 Estructura de Módulos — Colapsada

**Diseño:** arquitectura en capas separadas:
```
network/graph.rs, network/builder.rs
components/pipe.rs, components/pump.rs, components/valve.rs
components/hx.rs, components/tank_open.rs, ...
solver/hydraulic.rs, solver/thermal.rs, solver/linalg.rs
```

**Implementación:** todo colapsado en 3 archivos:
```
thnet/network.rs   ← Node + Pipe (refactorizado: componentes separados mediante BranchComponent enum)
thnet/solver.rs    ← Hidráulica + Térmica + álgebra lineal
thnet/thermo.rs    ← Propiedades físicas
```

Pragmático para la etapa actual, pero limita la extensibilidad.

### 3.2 Traits NetworkNode / NetworkBranch — ✅ Implementados

El diseño define dos traits polimórficos centrales (ARCHITECTURE.md §3.2):

```rust
pub trait NetworkNode: Send + Sync { ... }
pub trait NetworkBranch: Send + Sync { ... }
```

**Implementación:** ✅ **Completado.** Se han definido ambos traits y se implementaron en las estructuras `Node` y `Pipe` respectivamente. Esto facilita la extensión de la red para soportar otros tipos de nodos o ramas polimórficas en el resolvedor.

### 3.3 Bomba Centrífuga — Curva H-Q Real y Leyes de Afinidad

**Diseño:** curva H-Q cuadrática con leyes de afinidad:
```
ΔP_bomba(W, ω) = (ω/ω_nom)² · ΔP_nom(W · ω_nom/ω)
```

**Implementación:** ✅ **Completado.** Se da soporte a curvas cuadráticas completas configurables mediante coeficientes polinomiales $[a_0, a_1, a_2]$ tales que $\Delta P_{\text{nom}}(W) = a_0 + a_1 W + a_2 |W| W$. La velocidad variable se modela dinámicamente mediante `pump_speed_ratio` ($s = \omega/\omega_{\text{nom}}$), aplicando las leyes de afinidad:

$$\Delta P_{\text{bomba}}(W, s) = a_0 s^2 + a_1 s W + a_2 |W| W$$

El resolvedor de momento hidráulico linealiza completamente este término en el Jacobiano (matriz de conductancia) aportando una resistencia de bomba efectiva:

$$R_{\text{bomba, eff}} = - (a_1 s + 2 a_2 |W|)$$

Esto garantiza estabilidad numérica absoluta ante cambios dinámicos de velocidad e inversiones de flujo (flujo reverso).

### 3.4 Válvula de Control — Kv vs. Cv

**Diseño:** usa coeficiente `Kv` (norma ISA europea, m³/h @ 1 bar):
```
K_valve = 1 / Kv_eff²
```

**Implementación:** Se da soporte nativo a ambos coeficientes en el API del componente `Pipe`. La física se unifica de manera interna bajo el coeficiente `Cv` tras la conversión correspondiente:
- Para configurar usando $Cv$: `.with_valve(cv, opening)`
- Para configurar usando $Kv$: `.with_valve_kv(kv, opening)` (convierte automáticamente usando la relación $Cv = 1.156099 \cdot Kv$).

Se implementan todas las características de apertura:
- **Linear**: $f(x) = x$
- **EqualPercentage**: $f(x) = r^{x-1}$ con $r = 50.0$ (trim exponencial).
- **QuickOpening**: $f(x) = \sqrt{x}$.

La característica de la válvula puede configurarse usando el método `.with_valve_char(ValveChar::...)` en `Pipe`.

### 3.5 Solver Lineal: Gauss-Seidel/SOR vs. Gaussian Elimination

**Diseño (MATH_SOLVER.md §6):** Gauss-Seidel con SOR (ω=1.0–1.9), sparse.

**Implementación:** ✅ **Completado.** Se da soporte tanto a `GaussianElimination` densa (para redes pequeñas/medianas) como a `GaussSeidelSor` utilizando una representación de matriz dispersa por filas `SparseMatrix` (LIL). 

La selección de solver es configurable a través de los campos de `Solver`: `linear_solver`, `sor_omega` (default: 1.5), `sor_tol` (default: 1e-6) y `sor_max_iter` (default: 1000). Además, al usar `GaussSeidelSor` se inicializan las presiones libres utilizando un *warm start* con las presiones del paso anterior, reduciendo drásticamente las iteraciones necesarias.

### 3.6 Término Fuente de la Bomba (S_j)

**Diseño:**
```
Sj = (Ij·Wj + Δt·(ΔPgrav + ΔPbomba)) / (Ij + Δt·Rj)
```

**Implementación:** usa `pipe.pump_dp_max` directamente como `ΔPbomba` constante.
Correcto para el modelo simplificado, pero no implementa la dependencia de la bomba en W.

---

## 4. Componentes del Diseño — Estado de Implementación

### 4.1 Check Valve (Válvula de Retención)

El diseño especifica (MATH_SOLVER.md §4.4):
```
Si Wj < 0: K_valve = K_grande (1e12)   — cierra
```
Con histéresis y banda muerta para evitar chattering.

**Implementada en `Pipe` y `Solver` (`with_check_valve`).**
El solvedor evalúa el signo del caudal en el paso de tiempo previo y aplica histéresis con una banda muerta de $W_{umbral} = 10^{-4}$ kg/s para prevenir oscilaciones numéricas (*chattering*). Cuando se detecta flujo inverso, la válvula de retención se cierra aplicando una resistencia de $10^{12}$ Pa·s/kg, bloqueando por completo el flujo. Se reabre en cuanto la diferencia de presiones induce una fuerza impulsora en la dirección directa por encima del umbral.

### 4.2 OpenTank (Pileta con Nivel Libre)

El diseño define `OpenTank` como nodo Dirichlet dinámico cuya presión varía con el nivel:
```
P = P_atm + ρ·g·H
dH/dt = W_net / (ρ · A_tank)
```

**Implementado como componente (`OpenTank`) en `network.rs` y con soporte directo en `solver.rs`.**
La red mantiene un vector de tanques abiertos (`pub open_tanks: Vec<OpenTank>`). El solvedor se encarga de:
- **Pre-step:** Actualizar la presión y la condición de Dirichlet en los nodos correspondientes basándose en el nivel del líquido actual y la densidad local.
- **Post-step:** Calcular el balance de caudal neto en cada nodo de tanque, integrar el nivel de líquido de forma discreta (`dH/dt = W_net / (ρ · A_tank)`), clampear el nivel a sus límites físicos y actualizar la presión de forma inmediata.

Esto simplifica el bucle de simulación del usuario a un simple `solver.step(&mut net, dt)`, eliminando la lógica de integración manual del ejemplo [dos_tanques.rs](../../examples/thnet/dos_tanques.rs).

### 4.3 ClosedTank / StratifiedTank

**ClosedTank:** definido en ARCHITECTURE.md §4.4 y MATH_SOLVER.md §4.6.
**Implementado como componente (`ClosedTank`) en `network.rs` y con soporte directo en `solver.rs`.**
Soporta dos modelos físicos de presurización:
- **Colchón de Gas (Gas Cushion)**: Modelado mediante la ley de compresión del gas ideal ($P \cdot V_g^\gamma = \text{cte}$).
- **Compresibilidad del Líquido**: Modelado mediante el coeficiente de compresibilidad volumétrica $\beta$ del fluido ($P = P_0 + \frac{\Delta V_l}{\beta \cdot V_{\text{total}}}$).

El resolvedor actualiza las presiones en la fase pre-step y los niveles volumétricos en la fase post-step, manteniendo la compatibilidad con el resto del lazo hidráulico.

**StratifiedTank:** definido en ARCHITECTURE.md §4.5 y MATH_SOLVER.md §4.7.
**Implementado y completamente encapsulado como componente (`StratifiedTank`) en `network.rs` y con soporte en `solver.rs`.**
La lógica de integración de las celdas, balance advectivo 1D upwind implícito, calefactor localizado, inyección dinámica y variación de nivel se encuentra encapsulada dentro de sus propios métodos en la estructura (como `update_levels_and_temp`). Todos sus campos son privados y se exponen mediante getters/setters públicos. El ejemplo [tanque_estratificado.rs](../../examples/thnet/tanque_estratificado.rs) ha sido simplificado para utilizar la API encapsulada.


### 4.4 HeatExchanger NTU-ε

El diseño (MATH_SOLVER.md §4.8) especifica el modelo NTU-ε completo para contracorriente:
```
ε = [1 - exp(-NTU·(1-Cr))] / [1 - Cr·exp(-NTU·(1-Cr))]
Q = ε · Cmin · (Th_in - Tc_in)
```

**Implementado como componente de dos fluidos (`HeatExchanger`) en `network.rs` y con soporte directo en `solver.rs`.**
El solvedor calcula dinámicamente en cada paso de tiempo la efectividad $\varepsilon$ y el calor transferido $Q$ en base a las temperaturas de entrada correspondientes a las direcciones de los caudales, los calores específicos variables del fluido, y las capacidades caloríficas. Este calor se acopla como un término de fuente/sumidero térmico externo distribuido (`q_hx_external`) en las respectivas tuberías (lado caliente y frío).

### 4.5 Detector de Redes Desconectadas

### 4.5 Detector de Redes Desconectadas

✅ **Completado.** Se implementó la validación dinámica de conectividad física en cada paso temporal de resolución (`step_hydraulic`). Si un nodo queda aislado o desconectado de cualquier nodo Dirichlet (referencia de presión) debido al diseño de la red o al cierre dinámico de válvulas (apertura < 1e-5) o check valves, el resolvedor detecta el error de topología y lanza un pánico explícito reportando la lista exacta de nodos aislados.

### 4.6 NetworkBuilder (API Fluida) y IncidenceMatrix

✅ **Completado.** Se ha estructurado modularmente la red agregando:
* `builder.rs` para la API declarativa y fluida (`NetworkBuilder`), que simplifica la creación de redes.
* `graph.rs` para la matriz de incidencia (`IncidenceMatrix`), que representa algebraicamente la conectividad de la red.

### 4.7 Output / CSV

✅ **Completado.** Se implementó el módulo de salida `src/thnet/output/` con:
* `RecordSelector` (enum tipado) que permite seleccionar de forma exhaustiva qué variable de qué nodo, tubería o tanque queremos registrar (presión, temperatura, densidad, entalpía, caudales, niveles).
* `CsvRecorder` que inicializa el archivo con cabeceras auto-generadas y registra filas del estado de la red dinámicamente en el bucle.

---

## 5. Agregados Significativos (No Estaban en el Diseño) ✅+

### 5.1 Modelo de Inercia Térmica de Pared

Parámetros en `Pipe`: `wall_mass`, `wall_cp`, `wall_ua`, `wall_temp[]`.

El solver resuelve el sistema pared-fluido de forma semi-implícita eliminando
analíticamente la temperatura de pared:
```
h_new = (h_cell + CFL·h_in + NTU_w·h_wall + q_eff) / (1 + CFL + NTU_w)
```

Permite modelar el calentamiento/enfriamiento lento de tuberías metálicas.
**Modelo documentado en MATH_SOLVER.md §4.9.**

### 5.2 Temperatura Fija por Nodo (Dirichlet Térmico)

```rust
pub fixed_temperature: Option<f64>,
```

Permite fijar la temperatura de un nodo independientemente de la hidráulica.
No estaba en el diseño de los traits `NetworkNode`.

### 5.3 Caso Especial: Todos los Nodos con Presión Fija

Cuando no hay nodos libres, el solver resuelve directamente la ecuación de momento
por rama con iteración de punto fijo, sin ensamblar ningún sistema lineal.

### 5.4 Clamp Físico de Entalpía

```rust
node.h = node.h.clamp(
    thermo::enthalpy(T_REF_K + 1.0),   // 1 °C
    thermo::enthalpy(T_REF_K + 110.0), // 110 °C
);
```

Evita divergencia numérica en condiciones extremas. No estaba en el diseño.

---

## 6. Plan de Fases del Diseño — Estado Actual

```
Fase 1 — Núcleo Hidráulico (MVP):
  ✅ graph.rs y builder.rs reales (implementados en submódulos de network)
  ✅ hydraulic.rs: ensamblaje C y b, solver lineal (Gaussian Elimination o Gauss-Seidel/SOR disperso configurable)
  ✅ pipe.rs: inercia + Darcy-Weisbach
  ⚠️ tank_closed.rs: nodo Dirichlet ✅, pero sin dinámica de volumen/presión
  ✅ Test básico funcional

Fase 2 — Componentes Hidráulicos:
  ✅ pump.rs: curva cuadrática real con velocidad variable y leyes de afinidad (afinidad implícita y derivada en Jacobiano)
         ✅ Validada en bomba_valvula_cano y en test unitario `test_pump_curve_and_speed`
  ✅ valve.rs: Soporta curvas Lineal, Equal% y QuickOpening y CheckValve
  ✅ tank_open.rs: Implementado como struct `OpenTank` encapsulado
 
Fase 3 — Transporte Térmico:
  ✅ thermal.rs: upwind IMPLÍCITO (mejor que el diseño explícito)
  ✅ Perfil de T en pipes (N celdas)
  ✅ wall model: inercia térmica de pared semi-implícita (agregado)
         ✅ Validado en bomba_valvula_caño.rs (inox 304L, 164 kg, UA=1500)
  ✅ hx.rs equivalente: Modelo NTU-ε de dos fluidos contracorriente completo (`HeatExchanger`)
         ✅ Validado mediante test unitario dedicado `test_heat_exchanger_ntu_epsilon`
  ✅ tank_strat.rs: Encapsulado como componente en thnet

Fase 4 — Robustez y Output:
  ✅ Newton-Raphson con control de convergencia
  ✅ Detector de redes desconectadas dinámico
  ✅ csv.rs en módulo thnet (`CsvRecorder` y `RecordSelector`)
  ✅ Lector/Cargador de red desde JSON (`loader.rs`)
  ✅ Ejemplos documentados en examples/thnet/ (4 ejemplos completos)
```

---

## 7. Recomendaciones Prioritarias

### 🔴 Alta Prioridad

1. **Encapsular OpenTank como componente** — ✅ **Completado.** La física está integrada con lógica pre/post-step en el `Solver` y expuesta como `OpenTank` en `network.rs`.

2. **CheckValve** — ✅ **Completado.** Implementada con lógica de histéresis y banda muerta en el solver y la tubería.

3. **Curva H-Q real de bomba** — ✅ **Completado.** Implementada con coeficientes cuadráticos $[a_0, a_1, a_2]$, leyes de afinidad de velocidad variable y linealización de Jacobiano.

### 🟡 Media Prioridad

4. **Encapsular StratifiedTank como componente** — ✅ **Completado.** Lógica encapsulada, campos privados, getters/setters y simulación interna.

5. **Separar componentes en structs distintos** — ✅ **Completado.** Refactorizado mediante el enum `BranchComponent` (`Pipe`, `Pump`, `Valve`, `CheckValve`), encapsulando las configuraciones específicas en sub-structs sin mezclar estados y optimizando memoria.

6. **HX NTU-ε de dos fluidos** — ✅ **Completado.** Implementado modelo NTU-ε contracorriente de dos fluidos.

7. **Características de válvula** — ✅ **Completado.** Implementado soporte para curvas `Linear`, `EqualPercentage` y `QuickOpening`.

### 🟢 Baja Prioridad

8. **Solver sparse (Gauss-Seidel/SOR o faer)** — ✅ **Completado.** Implementada representación `SparseMatrix` y solver iterativo `gauss_seidel_sor` con relajación y warm-start.

9. **Detector de islas** — ✅ **Completado.** Diagnóstico dinámico de conectividad física en cada paso de tiempo.

10. **Output CSV centralizado** — ✅ **Completado.** Registrador tipado `CsvRecorder` implementado con cobertura completa de variables.
