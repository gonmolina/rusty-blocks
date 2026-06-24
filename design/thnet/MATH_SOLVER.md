# THNet — Matemática Detallada del Solvedor
## Documento Técnico v0.1

---

## 1. Formulación del Sistema Lineal en Cada Paso

### 1.1 Variables del problema

Para una red con N nodos y E ramas, en el paso k→k+1:

**Incógnitas:**
- `P^{k+1} ∈ ℝ^N`   — presiones nodales [Pa]
- `W^{k+1} ∈ ℝ^E`   — caudales másicos [kg/s]

**Datos del paso anterior:**
- `P^k, W^k, h^k, ρ^k` conocidos

### 1.2 Ecuación de momento semi-implícita (por rama j)

```
Ij · (Wj^{k+1} - Wj^k) / Δt = (P_up - P_dn)^{k+1} - Rj^k · Wj^{k+1} + ΔPgrav_j + ΔPbomba_j^k
```

Donde:
- `Ij = ρ · Lj / Aj`                      (inercia hidráulica)
- `Rj^k = Kj · |Wj^k| / ρj`              (resistencia linealizada)
- `ΔPgrav_j = -ρ · g · ΔZj`              (ΔZj > 0 si el nodo dn está arriba)
- `ΔPbomba_j^k` = presión de bomba (si aplica)

Despejando `Wj^{k+1}`:

```
(Ij + Δt · Rj^k) · Wj^{k+1} = Ij · Wj^k + Δt · (P_up - P_dn)^{k+1} + Δt · (ΔPgrav + ΔPbomba)

              ⟺

Wj^{k+1} = Gj · (P_up - P_dn)^{k+1} + Sj^k
```

Con:
```
Gj  = Δt / (Ij + Δt · Rj^k)                              [kg/s/Pa]
Sj  = (Ij · Wj^k + Δt · (ΔPgrav_j + ΔPbomba_j)) / (Ij + Δt · Rj^k)
```

### 1.3 Conservación de masa en cada nodo

Para nodos sin acumulación:
```
Σ_{j: rama j toca nodo i}  σ(i,j) · Wj^{k+1} = 0

donde σ(i,j) = +1 si j llega al nodo i
               -1 si j sale del nodo i
```

Sustituyendo la expresión de Wj:

```
Σ_j σ(i,j) · [Gj · σ(i,j) · Pi^{k+1} - Gj · P_otro(i,j)^{k+1} + Sj^k] = 0
```

Donde `P_otro(i,j)` es la presión del nodo extremo opuesto de la rama j.

Esto produce:
```
[Σ_j Gj] · Pi^{k+1}  -  Σ_j Gj · P_otro(i,j)^{k+1}  =  -Σ_j σ(i,j) · Sj^k
```

### 1.4 Sistema matricial C · P^{k+1} = b

```
C[i,i]  = Σ_{j conecta al nodo i}  Gj
C[i,l]  = -Gj   (si la rama j conecta nodo i con nodo l)
b[i]    = -Σ_{j conecta nodo i}  σ(i,j) · Sj^k
```

Para nodos con **presión fija** (Dirichlet):
- Eliminar la fila i de C y b
- Pasar el término conocido Pi^{k+1} = Pi_fijo a b en las ecuaciones vecinas

### 1.5 Actualización de caudales tras resolver P

```
Wj^{k+1} = Gj · (P_up^{k+1} - P_dn^{k+1}) + Sj^k
```

---

## 2. Iteraciones Newton-Raphson

Como `Rj^k = Kj · |Wj^k| / ρ` es linealizado, se itera dentro de cada paso Δt:

```
Algoritmo para el paso de tiempo k → k+1:

1. Inicializar: W^{(0)} = W^k
2. Para n = 0, 1, ..., MAX_ITER:
   a. Calcular Rj^{(n)} = Kj · |Wj^{(n)}| / ρj
   b. Calcular Gj^{(n)}, Sj^{(n)}
   c. Ensamblar C^{(n)}, b^{(n)}
   d. Aplicar condiciones de contorno
   e. Resolver: C^{(n)} · P^{(n+1)} = b^{(n)}
   f. Calcular: W^{(n+1)} = G^{(n)} · A^T · P^{(n+1)} + S^{(n)}
   g. Calcular residual: ε = max_j |Wj^{(n+1)} - Wj^{(n)}|
   h. Si ε < TOL: break
3. Asignar: W^{k+1} = W^{(n+1)}, P^{k+1} = P^{(n+1)}
```

Convergencia típica: 3–6 iteraciones para TOL = 1e-8 kg/s.

---

## 3. Transporte Térmico (Operator Splitting)

### 3.1 Esquema upwind explícito sobre grafo

Con los caudales `W^{k+1}` ya conocidos:

```
Para cada nodo i (no Dirichlet):

M_i · (h_i^{k+1} - h_i^k) / Δt = Φ_net_i + Q_ext_i

donde:
  Φ_net_i = Σ_{j conecta al nodo i}  σ(i,j) · Wj^{k+1} · h_upwind(Wj, i, j)

  h_upwind(Wj, i, j) = { h_upstream(j)^k   si el flujo llega al nodo i
                        { h_i^k              si el flujo sale del nodo i
```

Explícitamente:
```
Si σ(i,j) = +1  (j llega a i):
    si Wj > 0:  contribuye con  +Wj · h_upstream_del_j^k
    si Wj < 0:  contribuye con  +Wj · h_i^k         (sale del nodo i en realidad)

Si σ(i,j) = -1  (j sale de i):
    si Wj > 0:  contribuye con  -Wj · h_i^k
    si Wj < 0:  contribuye con  -Wj · h_upstream_del_j^k
```

### 3.2 Estabilidad CFL térmica

El CFL para estabilidad del transporte advectivo explícito:

```
Δt ≤ min_j { Mj / |Wj| }

donde Mj = ρ · Vj es la masa del volumen de control asociado a la rama j.
```

Para un pipe de 5m, 5 celdas, d=50mm, W=0.3 kg/s:
```
V_celda = π·(0.025)²·1.0 = 1.96e-3 m³
M_celda = 1000 · 1.96e-3 = 1.96 kg
Δt_max  = 1.96 / 0.3 = 6.5 s   ←  muchísimo mayor que la restricción acústica
```

### 3.3 Actualización de la masa del nodo

Para nodos con acumulación (tanques):
```
M_i^{k+1} = M_i^k + Δt · Σ_j σ(i,j) · Wj^{k+1}
h_i^{k+1} = (M_i^k · h_i^k + Δt · Φ_net_i + Δt · Q_ext_i) / M_i^{k+1}
```

---

## 4. Física de Cada Componente

### 4.1 Pipe — Fricción de Darcy-Weisbach

```
f = fricción_factor(Re, ε/D)
  donde Re = ρ · v · D / μ = |W| · D / (A · μ)

ΔPfric = f · (L/D) · (ρ/2) · v²
       = f · L / D · |W|·W / (2·ρ·A²)
       = K_pipe · |W|·W / ρ

con K_pipe = f · L / (D · 2 · A²)
```

Factor de fricción (Churchill 1977, sin iteración):
```
f = 8 · [(8/Re)^12 + (A+B)^{-1.5}]^{1/12}

A = [-2.457·ln((7/Re)^0.9 + 0.27·ε/D)]^16
B = (37530/Re)^16
```

Válido para todo Re (laminar y turbulento).

### 4.2 Pump — Curva HQ y Leyes de Afinidad

Curva nominal a velocidad ω_nom (polinomio cuadrático):
```
ΔH_nom(W_nom) = a₀ + a₁·W_nom + a₂·W_nom²     [m.c.a.]
ΔP_nom = ρ·g·ΔH_nom
```

Con leyes de afinidad a velocidad ω:
```
W_equiv = W · (ω_nom/ω)
ΔP_bomba(W, ω) = (ω/ω_nom)² · ΔP_nom(W · ω_nom/ω)
```

Para incluir en el solvedor, ΔP_bomba entra en el término Sj:
```
Sj_bomba = Δt · ΔP_bomba / (Ij + Δt · Rj)
```

### 4.3 ControlValve — Coeficiente de Flujo Kv

Definición del Kv (norma ISA):
```
W = Kv_eff · sqrt(ρ · ΔP)    [kg/s]

ΔP = (W / Kv_eff)² / ρ
   = K_valve · W · |W| / ρ

con K_valve = 1 / Kv_eff²
    Kv_eff  = Kv_max · f(opening)
```

Características de válvula:
```
Linear:          f(x) = x
EqualPercentage: f(x) = r^{x-1}   con r = 50 típico
QuickOpening:    f(x) = sqrt(x)
```

Para válvula cerrada (x → 0):
```
K_valve → ∞  →  Gj = Δt / (Ij + Δt·K·|W|/ρ) → 0
```
La rama queda prácticamente desconectada.

### 4.4 CheckValve (Válvula de Retención)

```
Si Wj^k ≥ 0:  K_valve = K_pasivo_pequeño  (abierta, resistencia normal)
Si Wj^k < 0:  K_valve = K_grande (1e12)   (cerrada, no pasa flujo)
```

Se evalúa con el signo del paso anterior (histórico), lo que puede producir
chattering en la inversión de flujo. Mitigación: histéresis con banda muerta
|Wj| < W_umbral → mantener estado anterior.

### 4.5 OpenTank — Nivel Libre Variable

```
Estado:    nivel H [m]
Presión:   P_nodo = P_atm + ρ·g·H    (es un nodo Dirichlet dinámico)
Nivel:     dH/dt = W_net / (ρ · A_tank)
           H^{k+1} = H^k + Δt · W_net^{k+1} / (ρ · A_tank)
```

**Importante:** Como P_nodo cambia entre iteraciones de Newton (porque H
cambia), la actualización del nivel se hace al final del paso, después de
convergencia, usando el caudal `W^{k+1}` final.

### 4.6 ClosedTank — Presión por Compresibilidad

Para un gas cushion:
```
P·V^γ = cte  →  P = P₀ · (V₀ / V)^γ
V = V_total - M_liquido / ρ_liquido
```

Para agua directamente (compresibilidad β):
```
P = P₀ + ΔM / (ρ · β · V)
```

El nodo puede ser Dirichlet (si el volumen es muy grande) o una ecuación
adicional que vincula P con M.

### 4.7 StratifiedTank — Modelo de Celdas Verticales

```
N celdas de altura Δz = H_total/N
Celda i: [M_i, h_i] — masa y entalpía

Balance de masa:
  dM_i/dt = W_in_i - W_out_i + W_mix_arriba - W_mix_abajo

Balance de entalpía:
  d(M_i·h_i)/dt = W_in_i·h_in - W_out_i·h_i + Φ_mix + Q_pared_i

Conducción entre celdas (opcional):
  Q_cond = λ_eff · A · (T_{i+1} - T_i) / Δz

Pérdidas al ambiente:
  Q_pared_i = U_wall · A_lateral_i · (T_i - T_amb)
```

El flujo de carga/descarga se inyecta en la celda cuya temperatura es
más cercana a la del fluido entrante (stratification-friendly injection).

### 4.8 HeatExchanger — Modelo NTU-ε

```
NTU  = UA / C_min
C_h  = W_h · cp_h    (calor específico hot side)
C_c  = W_c · cp_c    (calor específico cold side)
C_r  = C_min / C_max

Para contracorriente:
ε = [1 - exp(-NTU·(1 - C_r))] / [1 - C_r·exp(-NTU·(1-C_r))]

Q_transferido = ε · C_min · (T_h_in - T_c_in)
T_h_out = T_h_in - Q / C_h
T_c_out = T_c_in + Q / C_c
```

En la red, el HX aparece como dos nodos (hot in/out, cold in/out) conectados
por ramas con un Q_ext que se calcula a partir del estado termodinámico de
los cuatro nodos adyacentes.

### 4.9 Pipe — Inercia Térmica de Pared (Wall Model)

En tuberías metálicas con masa significativa, la pared actúa como un
acumulador térmico que suaviza los transitorios. El modelo agrega, por cada
celda de la tubería, un nodo de temperatura de pared `T_wall_i` acoplado
al fluido mediante un coeficiente global `UA_wall`.

**Parámetros del modelo:**
```
M_wall      — masa total de la pared metálica [kg]
cp_wall     — calor específico del metal [J/(kg·K)]  (acero ≈ 490, cobre ≈ 390)
UA_wall     — coeficiente global pared–fluido [W/K]  (conductancia total)
T_wall_i    — temperatura de la pared en la celda i [K]  (estado interno)
```

**Balance acoplado fluido–pared (por celda i):**

```
Fluido:  M_f · dh_i/dt  = W·(h_{i-1} - h_i) + UA_cell·(T_wall_i - T_f_i) + Q_ext_i
Pared:   M_w · cp_w · dT_wall_i/dt = UA_cell·(T_f_i - T_wall_i) + Q_source_i

donde:
  M_f     = ρ_f · V_cell          (masa de fluido en la celda)
  M_w     = M_wall / N            (masa de pared por celda)
  UA_cell = UA_wall / N           (conductancia por celda)
  Q_ext_i = Q_total / N           (calor externo distribuido uniformemente)
```

**Discretización semi-implícita (elimina T_wall_i analíticamente):**

Partiendo del balance de pared discretizado con Euler implícito:

```
(M_w·cp_w / Δt) · (T_wall^{k+1} - T_wall^k) = UA_cell · (T_f^{k+1} - T_wall^{k+1}) + Q_source

Despejando T_wall^{k+1}:
  T_wall^{k+1} = (K_w · T_wall^k + Q_source + UA_cell · T_f^{k+1}) / (K_w + UA_cell)

donde K_w = M_w·cp_w / Δt
```

Sustituyendo en el balance de fluido y escribiendo en el espacio de entalpía
(con `c_eff = UA_cell·K_w / (K_w + UA_cell)` y `q_eff = UA_cell·Q_source / (K_w + UA_cell)`):

```
h_new = (h_cell + CFL·h_in + NTU_w·h_wall^k + q_eff·Δt/M_f) / (1 + CFL + NTU_w)

donde:
  CFL   = W·Δt / M_f             (número de Courant)
  NTU_w = c_eff·Δt / (M_f·cp_f)  (número de unidades de transferencia efectivo)
  h_wall^k = cp_f · (T_wall^k - T_ref)  (entalpía equivalente de la pared)
```

Este esquema es **incondicionalmente estable** para cualquier `CFL`, `NTU_w` y `Δt`:
los coeficientes del numerador y denominador son siempre positivos, garantizando
que `h_new` sea una media ponderada de las entalpías involucradas.

**Actualización de la temperatura de pared al final de cada celda:**
```
T_wall_i^{k+1} = (K_w · T_wall_i^k + Q_source + UA_cell · T_f_i^{k+1}) / (K_w + UA_cell)
```

**Casos límite:**
```
UA_wall → 0:   pared adiabática, sin acoplamiento (NTU_w → 0, esquema puro advectivo)
UA_wall → ∞:   pared en equilibrio térmico instantáneo con el fluido
M_wall  → 0:   modelo sin pared (degrada al esquema upwind estándar)
```

**Guía de parametrización:**
```
Tubería de acero inoxidable DN50 × 1.5 mm, L = 10 m:
  M_wall = π · D_ext · e · L · ρ_acero ≈ π · 0.054 · 0.0015 · 10 · 7900 ≈ 20 kg
  cp_wall = 490 J/(kg·K)
  UA_wall = h_conv · A_int  (h_conv ≈ 1000–5000 W/(m²·K) para agua turbulenta)
           ≈ 3000 · π · 0.050 · 10 ≈ 4700 W/K
```

---

## 5. Ejemplo: Sistema de Convección Natural

### 5.1 Topología

```
Nodo 0 (H_bottom): OpenTank @ z=0, P fija o nivel dinámico
Nodo 1 (H_top):    OpenTank @ z=5m, P fija o nivel dinámico

Rama 0 (pipe_up):    nodo 0 → nodo 1, ΔZ = +5m, Q = +10kW
Rama 1 (pipe_down):  nodo 1 → nodo 0, ΔZ = -5m, Q = -10kW
```

### 5.2 Sistema lineal (2 ramas, 2 nodos con P fija → sistema trivial)

Con ambos nodos de presión fija (condiciones de contorno), no hay incógnitas
de presión y el sistema de momentos se resuelve directamente por rama:

```
Rama 0 (pipe_up):
  (I₀ + Δt·R₀) · W₀^{k+1} = I₀·W₀^k + Δt·(P₀ - P₁) - Δt·ρ·g·5 + Δt·0
  Con P₀ = P_atm + ρ·g·0, P₁ = P_atm + ρ·g·5:
  → (P₀ - P₁) = -ρ·g·5  → el término de presión + gravedad se cancelan inicialmente
  → El calor Q induce cambio de densidad → desequilibrio → circulación

Rama 1 (pipe_down):
  Similar, con signos opuestos de ΔZ y Q
```

**Con nodos de presión libre** (tanques pequeños o cerrados), el sistema
tiene 2 incógnitas de presión y se resuelve el sistema 2×2:

```
C = [G₀ + G₁,  -G₀ - G₁ ] = [2G, -2G]  (si G₀=G₁=G)
    [-G₀ - G₁,  G₀ + G₁ ]   [-2G,  2G]

Rango incompleto → necesita al menos un nodo Dirichlet (referencia de presión)
```

---

## 6. Implementación del Solver Lineal

Para redes pequeñas-medianas (hasta ~1000 nodos), se usa **Gauss-Seidel
con relajación sucesiva (SOR)**:

```rust
fn gauss_seidel_sor(
    c: &CsrMatrix,      // matriz de conductancia sparse
    b: &[f64],           // lado derecho
    x: &mut [f64],       // presiones (in: estimación inicial, out: solución)
    omega: f64,          // factor de relajación (1.0–1.9)
    tol: f64,
    max_iter: usize,
) -> usize {
    for iter in 0..max_iter {
        let mut max_delta = 0.0_f64;
        for i in 0..x.len() {
            let x_old = x[i];
            let mut sigma = b[i];
            for &(j, cij) in c.row(i) {
                if j != i { sigma -= cij * x[j]; }
            }
            let x_new = sigma / c.diag(i);
            x[i] = x_old + omega * (x_new - x_old);
            max_delta = max_delta.max((x[i] - x_old).abs());
        }
        if max_delta < tol { return iter; }
    }
    max_iter
}
```

Para redes grandes (>1000 nodos) se puede usar **Cholesky sparse** vía la
crate `faer` o `nalgebra-sparse`.

---

## 7. Manejo de Redes con Topología Compleja

### 7.1 Redes en Malla (Loops)

La formulación nodal por conductancia maneja naturalmente las mallas:
la matriz C es correctamente ensamblada sin necesidad de identificar los
loops explícitamente. El método de nodos satisface KCL automáticamente.

### 7.2 Redes con Múltiples Bombas en Paralelo/Serie

En paralelo: las ramas de bomba comparten nodos de presión.
El sistema resuelve automáticamente el punto de operación conjunto.

En serie: las ramas están conectadas en cadena y el sistema
suma sus presiones naturalmente.

### 7.3 Condiciones de Contorno Mixtas

- **Presión impuesta** (Dirichlet): eliminar fila/columna de C
- **Caudal impuesto**: tratar como fuente de corriente → sumar al vector b
- **Curva bomba** (P=f(W)): linearizar e iterar con Newton-Raphson

---

## 8. Pseudocódigo Completo del Paso de Tiempo

```
function step(network, dt):

    # === PASO 1: HIDRÁULICO (sistema global implícito) ===
    for newton_iter in 0..MAX_NEWTON:
        # Calcular resistencias linealizadas
        for rama j in network.branches:
            Rj = Kj * abs(Wj) / rho(nodo_up(j))

        # Ensamblar C y b
        C = zeros(N, N)
        b = zeros(N)
        for rama j:
            Ij = rho * Lj / Aj
            Gj = dt / (Ij + dt * Rj)
            Sj = (Ij * Wj + dt * (dP_grav_j + dP_bomba_j)) / (Ij + dt * Rj)
            up, dn = nodos(j)
            C[up,up] += Gj;  C[up,dn] -= Gj;  b[up] -= Sj
            C[dn,dn] += Gj;  C[dn,up] -= Gj;  b[dn] += Sj

        # Aplicar Dirichlet
        for nodo i con P fija:
            pivot_dirichlet(C, b, i, P_fijo[i])

        # Resolver C·P = b
        P_new = solve(C, b)

        # Actualizar caudales
        for rama j:
            up, dn = nodos(j)
            Wj_new = Gj * (P_new[up] - P_new[dn]) + Sj

        # Convergencia
        if max(abs(W_new - W)) < TOL: break
        W = W_new

    P = P_new;  W = W_new

    # === PASO 2: TÉRMICO (explícito, upwind) ===
    for nodo i (no Dirichlet):
        phi = 0
        for rama j conectada a i:
            sign = if j_llega_a_i then +1 else -1
            w = W[j]
            h_uw = if w * sign > 0 then h[nodo_origen(j)] else h[i]
            phi += w * sign * h_uw
        M_new[i] = M[i] + dt * W_net[i]
        h_new[i] = (M[i] * h[i] + dt * (phi + Q_ext[i])) / M_new[i]

    # Actualizar estados termodinámicos
    for nodo i:
        ThermoState[i] = thermo.from_rho_h(M_new[i]/V[i], h_new[i])

    # Para piletas abiertas: actualizar nivel
    for tank in open_tanks:
        tank.level += dt * W_net[tank.id] / (rho * tank.area)
```
