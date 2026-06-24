# THNet — Simulador de Redes Termohidráulicas en Rust
## Documento de Arquitectura v0.1
**Fecha:** 2026-06-23

---

## 1. Problema a Resolver y Filosofía de Diseño

### 1.1 Diagnóstico de los Modelos Anteriores

Los modelos con `DiscreteHeader` y `DiscretePipe` sufrían de tres problemas fundamentales:

1. **Acoplamiento explícito presión-caudal**: Cada tubería calculaba su caudal
   usando las presiones de los headers del paso de tiempo **anterior**. Esto
   introduce un retardo que genera oscilaciones numéricas crecientes.

2. **Headers como capacitores artificiales**: Para suavizar ese retardo se
   necesitaban volúmenes de header grandes (≥100 L), distorsionando la física.

3. **Expansor virtual**: El filtro de presión de primer orden del Header
   introducía desfase que, bajo ciertas condiciones, generaba retroalimentación
   positiva y divergencia.

### 1.2 Solución: Red Hidráulica Global con Acoplamiento Implícito

**THNet** resuelve la red completa en cada paso como un **sistema lineal implícito**:

- Los **nodos** almacenan presión, temperatura y entalpía.
- Las **ramas** (pipes, bombas, válvulas) transportan caudal másico.
- En cada paso se resuelve un **sistema de ecuaciones** que satisface
  simultáneamente la conservación de masa y la ecuación de momento de
  *toda la red*.

Esto elimina la necesidad de headers grandes y permite pasos de tiempo
órdenes de magnitud mayores.

---

## 2. Fundamentos Matemáticos

### 2.1 Topología de Red (Grafo Dirigido)

```
Nodos (N):   [P_i, T_i, M_i]   — Presión, Temperatura, Masa
Ramas (E):   [W_ij]             — Caudal másico (positivo = dirección nominal)
```

Matriz de incidencia `A ∈ ℝ^{N×E}`:
- `A[i,j] = +1` si la rama j sale del nodo i
- `A[i,j] = -1` si la rama j llega al nodo i
- `A[i,j] =  0` si no conectada

### 2.2 Conservación de Masa (Nodos)

Para red incompresible (caso general del agua líquida):

```
Σ_j  A[i,j] · W_j = 0    ∀ nodo i sin acumulación
```

Para nodos con acumulación (piletas abiertas, tanques):

```
dM_i/dt = Σ_j A[i,j] · W_j + W_ext_i
```

Con Euler implícito:
```
M_i^{k+1} = M_i^k + Δt · (Σ_j A[i,j] · W_j^{k+1} + W_ext_i)
```

### 2.3 Ecuación de Momento por Rama

Para la rama j que conecta nodo `up` con nodo `dn`:

```
(ρ·L/A)_j · dW_j/dt = (P_up - P_dn) - ΔP_fric_j - ρ·g·ΔZ_j + ΔP_bomba_j
```

Donde:
- `ρ·L/A` = inercia hidráulica [kg/m⁴]
- `ΔP_fric = K · W·|W|/ρ`  (Darcy-Weisbach linealizado por Newton-Raphson)
- `ΔZ` = cambio de cota (positivo = subida)
- `ΔP_bomba` = presión de la bomba

**Linealización (Hardy-Cross / Newton-Raphson):**

```
ΔP_fric_j ≈ R_j^k · W_j^{k+1}    con    R_j^k = K_j · |W_j^k| / ρ
```

Ecuación de momento discretizada semi-implícita:

```
(I_j + Δt·R_j^k) · W_j^{k+1} = I_j · W_j^k + Δt · (P_up^{k+1} - P_dn^{k+1} + ΔP_grav_j + ΔP_bomba_j)
```

Donde `I_j = ρ·L_j/A_j`.

### 2.4 Sistema Global: Conductancia Nodal (Analogía SPICE/MNA)

Despejando W_j en función de (P_up - P_dn):

```
W_j^{k+1} = G_j · (P_up - P_dn)^{k+1} + S_j^k
```

Con:
- `G_j = Δt / (I_j + Δt·R_j^k)`           — conductancia de la rama
- `S_j^k = I_j·W_j^k + Δt·(ΔP_grav + ΔP_bomba) / (I_j + Δt·R_j^k)` — término fuente

Aplicando conservación de masa (A·W = 0):

```
Σ_j A[i,j] · [G_j · (P_up(j) - P_dn(j)) + S_j] = 0
```

Esto produce el sistema **Conductancia Nodal**:

```
C · P^{k+1} = b
```

Donde:
- `C ∈ ℝ^{N×N}` = matriz de conductancia (análogo a matriz de nodos SPICE)
- `C[i,i] = Σ_{j: A[i,j]≠0} G_j`
- `C[i,l] = -G_j  si la rama j conecta nodo i con nodo l`
- `b[i] = -Σ_j A[i,j] · S_j^k`

Este sistema es **simétrico y definido positivo** (SPD) para redes sin fuentes
de presión, lo que garantiza existencia y unicidad de la solución.

Un nodo de **presión fija** (pileta abierta, tanque de referencia) se trata
como condición de Dirichlet eliminando la ecuación correspondiente.

---

## 3. Arquitectura de Software en Rust

### 3.1 Estructura del Crate

```
thnet/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── network/
│   │   ├── mod.rs          — Network: colección de nodos y ramas
│   │   ├── graph.rs        — NodeId, BranchId, matriz de incidencia
│   │   └── builder.rs      — NetworkBuilder (API fluida)
│   ├── components/
│   │   ├── mod.rs
│   │   ├── pipe.rs         — Pipe: fricción D-W, inercia, gravedad
│   │   ├── pump.rs         — CentrifugalPump: curva HQ
│   │   ├── valve.rs        — ControlValve, CheckValve, PRV
│   │   ├── hx.rs           — HeatExchanger: modelo NTU-ε
│   │   ├── tank_open.rs    — OpenTank: nivel libre
│   │   ├── tank_closed.rs  — ClosedTank: volumen fijo
│   │   └── tank_strat.rs   — StratifiedTank: celdas 1D en Z
│   ├── solver/
│   │   ├── mod.rs
│   │   ├── hydraulic.rs    — HydraulicSolver: sistema C·P=b
│   │   ├── thermal.rs      — ThermalSolver: upwind explícito
│   │   └── linalg.rs       — Gauss-Seidel sparse / interfaz a faer
│   ├── thermo/
│   │   ├── mod.rs
│   │   ├── water.rs        — Propiedades agua (tabla IF97 o lineal)
│   │   └── state.rs        — ThermoState { p, T, rho, h, u, cp, mu }
│   └── output/
│       ├── mod.rs
│       └── csv.rs          — Registro de variables en CSV
└── examples/
    ├── natural_convection.rs
    ├── pump_loop.rs
    └── stratified_tank.rs
```

### 3.2 Traits Principales

```rust
/// Nodo de la red: almacena estado termodinámico
pub trait NetworkNode: Send + Sync {
    fn pressure(&self) -> f64;            // Pa
    fn temperature(&self) -> f64;         // K
    fn enthalpy(&self) -> f64;            // J/kg
    fn density(&self) -> f64;             // kg/m³
    fn is_pressure_fixed(&self) -> bool;  // Dirichlet BC
    fn fixed_pressure(&self) -> f64;      // valor si es Dirichlet
    /// Actualizar estado térmico tras un paso de transporte
    fn update_thermal(&mut self, h_net: f64, w_net: f64,
                      q_ext: f64, dt: f64, thermo: &dyn ThermoLib);
}

/// Rama de la red: conecta dos nodos
pub trait NetworkBranch: Send + Sync {
    fn nodes(&self) -> (NodeId, NodeId);   // (upstream, downstream)
    fn flow_rate(&self) -> f64;            // W [kg/s], actual
    fn set_flow_rate(&mut self, w: f64);

    /// Conductancia hidráulica linealizada G = Δt / (I + Δt·R)
    fn conductance(&self, rho: f64, dt: f64) -> f64;

    /// Término fuente S^k (contribución independiente de P)
    fn source_term(&self, rho: f64, dt: f64) -> f64;

    /// Entalpía que transporta la rama (upwind)
    fn enthalpy_flux(&self, h_up: f64, h_dn: f64) -> f64;
}
```

### 3.3 El Solvedor Hidráulico

```rust
pub struct HydraulicSolver {
    conductance: Vec<Vec<f64>>,  // matriz C (densa para redes pequeñas)
    rhs: Vec<f64>,
    pressure: Vec<f64>,
}

impl HydraulicSolver {
    pub fn step(&mut self, net: &mut Network, dt: f64) {
        const MAX_ITER: usize = 20;
        const TOL: f64 = 1e-6; // Pa de residual

        for _iter in 0..MAX_ITER {
            // 1. Calcular R_j^k = K_j · |W_j| / rho  (resistencia linealizada)
            // 2. Calcular G_j y S_j para cada rama
            // 3. Ensamblar matriz C y vector b
            self.assemble(net, dt);

            // 4. Aplicar condiciones de Dirichlet (presiones fijas)
            self.apply_bc(net);

            // 5. Resolver C · P = b
            self.solve(); // Gauss-Seidel o LU directo

            // 6. Actualizar caudales: W_j = G_j·(P_up - P_dn) + S_j
            self.update_flows(net, dt);

            // 7. Verificar convergencia (residual de masa por nodo)
            if self.mass_residual(net) < TOL { break; }
        }
    }
}
```

### 3.4 El Solvedor Térmico (Operator Splitting)

```rust
pub struct ThermalSolver;

impl ThermalSolver {
    /// Paso 2: transporte de entalpía con caudales ya conocidos
    pub fn step(&self, net: &mut Network, dt: f64) {
        // Para cada nodo i (no fuente de presión fija):
        //   h_i^{k+1} = h_i^k + Δt/M_i · [Σ_j A[i,j] · W_j · h_upwind_j + Q_ext_i]
        // con h_upwind = h del nodo de donde viene el flujo

        // Importante: usar los h^k del paso anterior para todas las ramas
        // (explícito), excepto para el calentador que puede ser semi-implícito.

        for node_id in net.non_fixed_nodes() {
            let mut phi_net = 0.0; // [W]
            for branch_id in net.branches_at(node_id) {
                let branch = &net.branch(branch_id);
                let (up, dn) = branch.nodes();
                let w = branch.flow_rate();
                let h_up = net.node(up).enthalpy();
                let h_dn = net.node(dn).enthalpy();
                let wh = branch.enthalpy_flux(h_up, h_dn);
                let sign = if up == node_id { -1.0 } else { 1.0 };
                phi_net += sign * wh;
            }
            let q_ext = net.node(node_id).external_heat();
            let m = net.node(node_id).mass();
            net.node_mut(node_id).update_thermal(phi_net, 0.0, q_ext, dt, &thermo);
        }
    }
}
```

---

## 4. Componentes de la Red

### 4.1 Pipe

```rust
pub struct Pipe {
    pub node_up: NodeId,
    pub node_dn: NodeId,
    pub diameter: f64,       // m
    pub length: f64,         // m
    pub roughness: f64,      // m (Nikuradse)
    pub elevation_drop: f64, // m (positivo = sube hacia dn)
    pub n_thermal_cells: usize,
    flow: f64,               // kg/s estado actual
    temp_cells: Vec<f64>,    // K perfil de temperatura
}

impl Pipe {
    fn friction_factor(&self, re: f64) -> f64 {
        // Colebrook-White (iterativo) o Churchill (directo)
        // Para Re < 2300: Hagen-Poiseuille f = 64/Re
    }
    fn hydraulic_resistance(&self, rho: f64) -> f64 {
        // K = f * L / D / (2 * A^2)   =>   ΔP = K * W * |W| / rho
    }
    fn hydraulic_inertia(&self, rho: f64) -> f64 {
        // I = rho * L / A
    }
}
```

### 4.2 Pump

```rust
pub struct CentrifugalPump {
    pub node_up: NodeId,
    pub node_dn: NodeId,
    pub curve: PumpCurve, // tabla HQ o coeficientes cuadráticos
    pub speed: f64,        // rpm actual
    pub speed_nom: f64,    // rpm nominal
    flow: f64,
}

// ΔP_bomba(W, ω) = (ω/ω_nom)² · HQ_nom(W · ω_nom/ω)
// Entra en el término fuente S_j de la rama
```

### 4.3 ControlValve

```rust
pub struct ControlValve {
    pub node_up: NodeId,
    pub node_dn: NodeId,
    pub kv_max: f64,     // m³/h @ 1bar de caída
    pub opening: f64,    // 0.0 (cerrado) a 1.0 (abierto)
    pub char: ValveChar, // Linear, EqualPercentage, QuickOpening
    flow: f64,
}

// K = 1 / (Kv * f(opening))^2
// Cuando opening → 0: K → ∞  (cerrado ≈ resistencia muy alta)
```

### 4.4 OpenTank

```rust
pub struct OpenTank {
    pub id: NodeId,
    pub area: f64,         // m² (sección transversal)
    pub level: f64,        // m (nivel actual)
    pub level_min: f64,
    pub level_max: f64,
    pub z_bottom: f64,     // m (cota del fondo absoluta)
}

// Presión en el nodo = P_atm + rho * g * level
// is_pressure_fixed = true (Dirichlet, pero varía con nivel)
// dL/dt = W_net / (rho * area)
```

### 4.5 StratifiedTank

```rust
pub struct StratifiedTank {
    pub id_inlet_bottom: NodeId,
    pub id_outlet_top: NodeId,
    pub height: f64,
    pub area: f64,
    pub n_cells: usize,
    cells: Vec<ThermalCell>, // [m_i, h_i] por capa
    pub q_loss_coeff: f64,   // W/(m²·K) pérdidas con ambiente
    pub t_ambient: f64,      // K
}

// Cada capa intercambia calor con las adyacentes por conducción
// y con el manto exterior por convección.
// El flujo de carga/descarga se inyecta en la capa correspondiente
// al nivel de densidad (hot water floats on cold).
```

---

## 5. Estabilidad Numérica

### 5.1 Análisis de Estabilidad

| Restricción | Método explícito (anterior) | THNet implícito |
|---|---|---|
| CFL acústico (ondas de presión) | `Δt < L/(N·c)` ≈ 0.3ms | **Eliminado** (acoplamiento implícito) |
| CFL térmico (convección) | `Δt < Δx/v` ≈ 7s | Sigue aplicando |
| Convergencia Newton | N/A | 3–5 iter @ TOL=1e-6 |
| **Paso típico usable** | **0.5 ms** | **1–5 s** |

### 5.2 Estabilidad del Sistema Lineal

La matriz de conductancia `C` es:
- **Simétrica** por construcción
- **Definida positiva** para redes con al menos un nodo de presión fija
- **Sparse** (nnz ≈ 2·E para la parte triangular)

Esto garantiza que el sistema `C·P = b` tiene siempre solución única.

### 5.3 Robustez ante Casos Especiales

| Caso | Manejo |
|---|---|
| Válvula cerrada | K_j → ∞ → G_j → 0 (rama desconectada) |
| Check valve (flujo inverso) | K_j → ∞ si W_j < 0 |
| Bomba a velocidad 0 | ΔP_bomba = 0, K_pasivo alto |
| Tanque vacío | Condición level ≥ level_min, W_salida = 0 |
| Red desconectada | Detectar islas → presión libre en isla |

---

## 6. Plan de Implementación

### Fase 1 — Núcleo Hidráulico (MVP)
- [ ] `graph.rs`: NodeId, BranchId, sparse incidence
- [ ] `hydraulic.rs`: ensamblaje de C y b, Gauss-Seidel
- [ ] `pipe.rs`: inercia + Darcy-Weisbach (f constante primero)
- [ ] `tank_closed.rs`: nodo Dirichlet (presión fija)
- [ ] Test: lazo pipe + tank → caudal de equilibrio estático

### Fase 2 — Componentes Hidráulicos
- [ ] `pump.rs`: curva cuadrática HQ
- [ ] `valve.rs`: Kv variable
- [ ] `tank_open.rs`: nivel libre dinámico
- [ ] Test: sistema bomba + válvula + red → punto de operación dinámico

### Fase 3 — Transporte Térmico
- [ ] `thermal.rs`: upwind sobre grafo, Operator Splitting
- [ ] Perfil de T en pipes (N celdas, PDE 1D advección)
- [ ] `hx.rs`: NTU-ε básico
- [ ] `tank_strat.rs`: modelo de estratificación
- [ ] Test: convección natural → comparar estabilidad con modelos anteriores

### Fase 4 — Robustez y Output
- [ ] Newton-Raphson con control de convergencia adaptivo
- [ ] Detector de redes desconectadas
- [ ] `csv.rs`: registro configurable de señales
- [ ] Ejemplos documentados con validación

---

## 7. Analogía con Circuitos Eléctricos (SPICE)

| Electrónica | THNet |
|---|---|
| Tensión (V) | Presión (Pa) |
| Corriente (A) | Caudal másico (kg/s) |
| Resistencia R | Resistencia hidráulica `K·|W|/ρ` |
| Inductancia L | Inercia hidráulica `ρ·L/A` |
| Fuente de tensión | Bomba (ΔP impuesto) / Tanque abierto |
| Fuente de corriente | Caudal de extracción fijo |
| Nodo GND | Nodo de presión de referencia |
| Capacitancia C | Compresibilidad real del fluido / pileta |

El solvedor hidráulico es equivalente al **Modified Nodal Analysis (MNA)**
de SPICE, garantizando las mismas propiedades de convergencia.

---

## 8. Comparativa Final con el Diseño Anterior

| Aspecto | Anterior (DiscreteHeader + DiscretePipe) | THNet |
|---|---|---|
| Acoplamiento P-W | Explícito (retardado) | Implícito (sistema global) |
| Headers necesarios | Grandes (≥100L) | No necesarios (solo los físicos) |
| Paso de tiempo | ≤0.5 ms | ≥1 s (solo CFL térmico) |
| Estabilidad | Condicional (α, τ_exp) | Incondicional (hidráulica) |
| Precisión | Distorsión por α artificial | Alta fidelidad |
| Redes complejas | Difícil (headers por nodo) | Natural (grafo arbitrario) |

---

## 9. Definición de Redes desde JSON

### 9.1 Requerimiento

El módulo `thnet` debe permitir construir una red completa (`Network`) a partir
de un archivo JSON. Esto habilita:

- Separar la **definición de la red** del **código de simulación**.
- Reutilizar el mismo ejecutable con distintas configuraciones de planta.
- Integrar con interfaces gráficas o herramientas externas que generen JSON.
- Facilitar la validación de modelos contra datos experimentales sin recompilar.

El loader debe implementarse en un módulo `loader.rs` (o `builder.rs`) dentro
de `src/thnet/`, exponiendo al menos:

```rust
pub fn load_network(json: &str) -> Result<Network, String>;
```

### 9.2 Estructura General del JSON

```json
{
  "nodes": [ ... ],
  "pipes": [ ... ],
  "open_tanks": [ ... ],
  "closed_tanks": [ ... ],
  "stratified_tanks": [ ... ],
  "heat_exchangers": [ ... ],
  "simulation": { ... }
}
```

Los nodos y pipes se referencian por su **índice** (posición en el array),
que corresponde al `NodeId` / `PipeId` (`usize`) del modelo interno.

### 9.3 Esquema por Componente

#### 9.3.1 Node ✅ Implementado

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `temperature` | `f64` | K | ✅ | Temperatura inicial |
| `pressure` | `f64` | Pa | ✅ | Presión inicial |
| `volume` | `f64` | m³ | ✅ | Volumen del nodo (header) |
| `fixed_pressure` | `bool` | — | ❌ | Si `true`, nodo Dirichlet de presión |
| `fixed_temperature` | `f64 \| null` | K | ❌ | Si presente, Dirichlet térmico |
| `external_heat` | `f64` | W | ❌ | Calor externo inyectado. Default: 0 |

```json
{
  "temperature": 293.15,
  "pressure": 1e5,
  "volume": 0.001,
  "fixed_pressure": true
}
```

#### 9.3.2 Pipe ✅ Implementado

Una pipe puede ser una tubería simple, una bomba, una válvula o un check valve,
determinado por el campo `component`.

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `node_up` | `usize` | — | ✅ | Índice del nodo upstream |
| `node_dn` | `usize` | — | ✅ | Índice del nodo downstream |
| `diameter` | `f64` | m | ✅ | Diámetro interno |
| `length` | `f64` | m | ✅ | Longitud de la tubería |
| `roughness` | `f64` | m | ✅ | Rugosidad absoluta (e.g. 1.5e-5) |
| `elevation_dz` | `f64` | m | ❌ | z_dn − z_up. Default: 0 |
| `n_cells` | `usize` | — | ❌ | Celdas térmicas. Default: 1 |
| `t_init` | `f64` | K | ❌ | Temperatura inicial. Default: 293.15 |
| `component` | `object` | — | ❌ | Componente especial (ver abajo). Default: pipe simple |

**Componente `"pipe"` (default):**

Opciones adicionales dentro de `component`:

```json
{ "type": "pipe" }
```

**Componente `"pump"`:**

| Campo | Tipo | Unidad | Descripción |
|---|---|---|---|
| `type` | `"pump"` | — | — |
| `dp_max` | `f64` | Pa | ΔP a caudal cero (alternativa a `coefs`) |
| `w_max` | `f64` | kg/s | Caudal a ΔP cero (alternativa a `coefs`) |
| `coefs` | `[f64; 3]` | Pa | Coeficientes [a₀, a₁, a₂] de dP = a₀ + a₁W + a₂W² |
| `speed_ratio` | `f64` | — | ω/ω_nom. Default: 1.0 |

```json
{
  "type": "pump",
  "coefs": [300000, 0, -1200],
  "speed_ratio": 1.0
}
```

**Componente `"valve"`:**

| Campo | Tipo | Unidad | Descripción |
|---|---|---|---|
| `type` | `"valve"` | — | — |
| `cv` | `f64` | — | Coeficiente Cv (usar `cv` o `kv`, no ambos) |
| `kv` | `f64` | — | Coeficiente Kv europeo (se convierte internamente) |
| `opening` | `f64` | 0–1 | Apertura inicial |
| `characteristic` | `string` | — | `"linear"`, `"equal_pct"` o `"quick_opening"`. Default: `"linear"` |

```json
{
  "type": "valve",
  "cv": 30.0,
  "opening": 0.5,
  "characteristic": "equal_pct"
}
```

**Componente `"check_valve"`:**

```json
{ "type": "check_valve" }
```

**Opciones térmicas de la rama** (aplican a `pipe` y `check_valve`):

| Campo | Tipo | Unidad | Descripción |
|---|---|---|---|
| `heat_total` | `f64` | W | Fuente de calor distribuida. Default: 0 |
| `wall` | `object` | — | Inercia térmica de pared (ver abajo) |
| `hx_shell` | `object` | — | Intercambiador shell-side (ver abajo) |

```json
"wall": { "mass_kg": 164.0, "cp_j_kg_k": 500.0, "ua_w_k": 1500.0 }
```

```json
"hx_shell": { "ua_w_k": 5000.0, "t_coolant_k": 293.15 }
```

#### 9.3.3 OpenTank ✅ Implementado

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `node_id` | `usize` | — | ✅ | Índice del nodo asociado al fondo |
| `area` | `f64` | m² | ✅ | Sección transversal del tanque |
| `level` | `f64` | m | ✅ | Nivel de líquido inicial |
| `level_min` | `f64` | m | ✅ | Nivel mínimo |
| `level_max` | `f64` | m | ✅ | Nivel máximo |
| `z_bottom` | `f64` | m | ❌ | Cota del fondo. Default: 0 |
| `p_atm` | `f64` | Pa | ❌ | Presión del gas. Default: 1e5 |

```json
{
  "node_id": 0,
  "area": 10.0,
  "level": 5.0,
  "level_min": 0.0,
  "level_max": 10.0
}
```

#### 9.3.4 ClosedTank ✅ Implementado

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `node_id` | `usize` | — | ✅ | Índice del nodo asociado al fondo |
| `area` | `f64` | m² | ✅ | Sección transversal |
| `level` | `f64` | m | ✅ | Nivel de líquido inicial |
| `level_min` | `f64` | m | ✅ | Nivel mínimo |
| `level_max` | `f64` | m | ✅ | Nivel máximo |
| `z_bottom` | `f64` | m | ❌ | Cota del fondo. Default: 0 |
| `v_total` | `f64` | m³ | ✅ | Volumen total del tanque |
| `p_gas_init` | `f64` | Pa | ✅ | Presión inicial del gas |
| `gamma` | `f64` | — | ❌ | Exponente adiabático. Default: 1.4 |
| `compressibility_beta` | `f64 \| null` | 1/Pa | ❌ | Si presente, usa modelo de compresibilidad en vez de gas cushion |

```json
{
  "node_id": 2,
  "area": 0.5,
  "level": 1.0,
  "level_min": 0.0,
  "level_max": 4.0,
  "v_total": 2.0,
  "p_gas_init": 1e5,
  "gamma": 1.4
}
```

#### 9.3.5 StratifiedTank ✅ Implementado

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `node_id` | `usize` | — | ✅ | Índice del nodo asociado al fondo |
| `area` | `f64` | m² | ✅ | Sección transversal |
| `level` | `f64` | m | ✅ | Nivel de líquido inicial |
| `level_min` | `f64` | m | ❌ | Nivel mínimo. Default: 0 |
| `level_max` | `f64` | m | ✅ | Altura máxima del tanque |
| `z_bottom` | `f64` | m | ❌ | Cota del fondo. Default: 0 |
| `p_atm` | `f64` | Pa | ❌ | Presión del gas. Default: 1e5 |
| `n_layers` | `usize` | — | ✅ | Número de capas térmicas |
| `t_init` | `f64` | K | ✅ | Temperatura inicial uniforme |
| `heater_height` | `f64` | m | ❌ | Altura del calefactor. Default: 0 |
| `heater_power` | `f64` | W | ❌ | Potencia del calefactor. Default: 0 |
| `inlet_height` | `f64` | m | ❌ | Altura del nozzle de entrada. Default: 0 |
| `inlet_flow` | `f64` | kg/s | ❌ | Caudal de inyección inicial. Default: 0 |
| `inlet_temp` | `f64` | K | ❌ | Temperatura de inyección. Default: t_init |

```json
{
  "node_id": 1,
  "area": 4.9,
  "level": 19.0,
  "level_max": 20.0,
  "n_layers": 20,
  "t_init": 293.15,
  "heater_height": 2.0,
  "heater_power": 10000.0,
  "inlet_height": 18.0,
  "inlet_flow": 10.0
}
```

#### 9.3.6 HeatExchanger ✅ Implementado

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `pipe_hot` | `usize` | — | ✅ | Índice de la pipe del lado caliente |
| `pipe_cold` | `usize` | — | ✅ | Índice de la pipe del lado frío |
| `ua` | `f64` | W/K | ✅ | Coeficiente UA global |

```json
{
  "pipe_hot": 0,
  "pipe_cold": 1,
  "ua": 4180.0
}
```

#### 9.3.7 Simulation (Parámetros del Solvedor)

| Campo | Tipo | Unidad | Requerido | Descripción |
|---|---|---|---|---|
| `dt` | `f64` | s | ✅ | Paso de tiempo |
| `t_final` | `f64` | s | ✅ | Tiempo final de simulación |
| `max_newton_iter` | `usize` | — | ❌ | Iteraciones Newton. Default: 50 |
| `tol_flow` | `f64` | kg/s | ❌ | Tolerancia de convergencia. Default: 1e-9 |
| `output_interval` | `f64` | s | ❌ | Intervalo de escritura CSV. Default: dt |

### 9.4 Componentes por Implementar

#### 9.4.x PRV (Pressure Relief Valve) ❌ No implementada

| Campo | Tipo | Unidad | Descripción |
|---|---|---|---|
| `type` | `"prv"` | — | — |
| `set_pressure` | `f64` | Pa | Presión de apertura |
| `blowdown` | `f64` | Pa | Banda de histéresis (cierra a set − blowdown) |
| `cv_full` | `f64` | — | Cv a apertura máxima |

#### 9.4.x CsvRecorder ❌ No implementado

| Campo | Tipo | Descripción |
|---|---|---|
| `file` | `string` | Ruta del archivo CSV de salida |
| `signals` | `array` | Lista de señales a registrar (e.g. `"node.0.pressure"`, `"pipe.1.flow"`) |
| `interval` | `f64` | Intervalo de escritura [s] |

### 9.5 Ejemplo Completo

```json
{
  "nodes": [
    { "temperature": 293.15, "pressure": 1e5, "volume": 0.001, "fixed_pressure": true },
    { "temperature": 293.15, "pressure": 1e5, "volume": 0.001 },
    { "temperature": 293.15, "pressure": 1e5, "volume": 0.001, "fixed_pressure": true }
  ],
  "pipes": [
    {
      "node_up": 1, "node_dn": 0,
      "diameter": 0.25, "length": 5.0, "roughness": 1.5e-5,
      "n_cells": 1, "t_init": 293.15
    },
    {
      "node_up": 1, "node_dn": 2,
      "diameter": 0.25, "length": 5.0, "roughness": 1.5e-5,
      "component": { "type": "valve", "cv": 30.55, "opening": 0.0 }
    }
  ],
  "stratified_tanks": [
    {
      "node_id": 1,
      "area": 4.9, "level": 19.0, "level_max": 20.0,
      "n_layers": 20, "t_init": 293.15,
      "heater_height": 2.0, "heater_power": 10000.0,
      "inlet_height": 18.0, "inlet_flow": 10.0
    }
  ],
  "simulation": {
    "dt": 0.2,
    "t_final": 2000.0
  }
}
```

