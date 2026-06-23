# Propuesta de Modelado de Cavitación en Redes Monofásicas (Válvulas y Bombas)

Este documento detalla los esquemas numéricos y las ecuaciones físicas necesarias para incorporar la simulación de cavitación en válvulas y bombas dentro del resolvedor monofásico **THNet**, junto con las limitaciones de aproximar un fenómeno inherentemente bifásico mediante formulaciones fenomenológicas.

---

## 1. Cavitación en Válvulas de Control (ISA-75.01)

En un resolvedor monofásico líquido, la cavitación en una válvula de control se manifiesta macroscópicamente como el **bloqueo de caudal** o *choked flow*. Al descender la presión aguas abajo ($P_d$), el caudal aumenta hasta que la presión estática en la sección contraída de la válvula (*vena contracta*) cae por debajo de la presión de saturación ($P_{\text{sat}}$). En este punto, la formación de burbujas limita el caudal máximo, el cual se vuelve independiente de $P_d$.

### 1.1. Ecuaciones del Modelo Fenomenológico
Basado en la norma **ISA-75.01**, se define el **factor de recuperación de presión del líquido ($F_L$)**, propio de la geometría de la válvula. La caída de presión crítica a partir de la cual el caudal se bloquea es:

$$\Delta P_{\text{crit}} = F_L^2 \left( P_u - F_F P_{\text{sat}}(T_u) \right)$$

Donde:
* **$P_u$**: Presión estática aguas arriba.
* **$P_{\text{sat}}(T_u)$**: Presión de vapor/saturación del líquido evaluada a la temperatura local de entrada $T_u$.
* **$F_F$**: Factor de relación crítica de presiones del líquido, aproximado por:
  $$F_F = 0.96 - 0.28 \sqrt{\frac{P_{\text{sat}}(T_u)}{P_c}}$$
  *(con $P_c = 22.06\text{ MPa}$ para el agua).*

### 1.2. Implementation Numérica en el Solver
Para incorporar esto sin alterar la estructura del resolvedor hidráulico, se redefine el coeficiente de la válvula ($C_v$) nominal a un **$C_v$ efectivo ($C_{v,\text{eff}}$)** en cada iteración del lazo de Newton-Raphson:

$$C_{v,\text{eff}} = C_v \cdot \min\left(1.0, \frac{F_L \sqrt{P_u - F_F P_{\text{sat}}(T_u)}}{\sqrt{P_u - P_d}}\right)$$

De esta forma, en el cálculo de la resistencia linealizada en [network.rs](file:///home/gonza/dev/CNEAProjects/rusty-blocks/src/thnet/network.rs):
1. Si $\Delta P_{\text{actual}} < \Delta P_{\text{crit}}$, se utiliza el $C_v$ nominal y el flujo es no-cavitante.
2. Si $\Delta P_{\text{actual}} \ge \Delta P_{\text{crit}}$, el $C_{v,\text{eff}}$ disminuye automáticamente, forzando a que el término de pérdida de carga cuadrática límite el caudal al valor máximo de bloqueo (*choked flow*):
   $$W_{\text{max}} = C_v \cdot F_L \cdot 2.4026 \times 10^{-5} \sqrt{\rho \left( P_u - F_F P_{\text{sat}} \right)}$$

---

## 2. Cavitación en Bombas Centrífugas (NPSH)

La cavitación en una bomba centrífuga ocurre cuando la presión en la succión (ojo del rodete) cae por debajo de la presión de vapor, reduciendo severamente la altura manométrica ($H$) generada y el caudal.

### 2.1. Ecuaciones del Modelo de Degradación
El fenómeno se evalúa comparando el NPSH Disponible de la instalación con el NPSH Requerido por la bomba.

1. **NPSH Disponible ($\text{NPSH}_A$)**: Calculado con las variables estáticas y dinámicas del nodo de succión ($u$):
   $$\text{NPSH}_A = \frac{P_u - P_{\text{sat}}(T_u)}{\rho g} + \frac{V_u^2}{2g}$$

2. **NPSH Requerido ($\text{NPSH}_R$)**: Obtenido a partir de la curva de cavitación de la bomba (generalmente modelada como función cuadrática del caudal másico $W$):
   $$\text{NPSH}_R(W) = A_{\text{npsh}} W^2 + B_{\text{npsh}} W + C_{\text{npsh}}$$

3. **Factor de Degradación de Carga ($\sigma$)**:
   Representa la pérdida de rendimiento de la bomba. Si el $\text{NPSH}_A$ cae por debajo del $\text{NPSH}_R$ (o del umbral de caída del 3%), la altura de presión disminuye mediante un factor $\sigma \in [0, 1]$:
   * Si $\text{NPSH}_A \ge \text{NPSH}_R \implies \sigma = 1.0$ (sin degradación).
   * Si $\text{NPSH}_A < \text{NPSH}_R \implies \sigma = \left[ 1.0 - \left( \frac{\text{NPSH}_R - \text{NPSH}_A}{\Delta \text{NPSH}_{\text{collapse}}} \right)^a \right]^+$, donde $\Delta \text{NPSH}_{\text{collapse}}$ es el rango en el cual la bomba pierde totalmente su capacidad de bombeo y $a$ es un exponente de ajuste de la velocidad de colapso.

### 2.2. Implementación Numérica en el Solver
El resolvedor de momento hidráulico utiliza el término de ganancia de presión de la bomba (`dp_pump`). Este término se escala en cada iteración del resolvedor en [solver.rs](file:///home/gonza/dev/CNEAProjects/rusty-blocks/src/thnet/solver.rs):

$$\Delta P_{\text{bomba, eff}} = \sigma \cdot \Delta P_{\text{bomba, nominal}}$$

Como $\sigma$ depende del caudal $W$, esto introduce un acoplamiento no lineal adicional que el esquema de Newton-Raphson puede resolver de forma implícita actualizando la derivada parcial en el Jacobiano de conductancia.

---

## 3. Limitaciones del Modelo Monofásico

Aunque las ecuaciones fenomenológicas anteriores permiten simular con excelente precisión los caudales y presiones a nivel de sistema de tuberías, existen limitaciones físicas severas debido al supuesto de fase única:

1. **Ausencia de Fracción de Vacío (Void Fraction)**: El modelo no calcula el volumen de vapor generado localmente en las tuberías. En sistemas reales, una línea cavitando acumula bolsas de vapor que alteran la capacitancia hidráulica (amortiguación de pulsaciones) y la compresibilidad del lazo.
2. **Desprecio del Calor Latente**: El cambio de fase líquido-vapor absorbe calor (entalpía de vaporización). Un resolvedor monofásico no captura el enfriamiento local del fluido ni los perfiles de temperatura resultantes de la evaporación y condensación.
3. **Dinámica de colapso y golpe de ariete**: Las burbujas de vapor colapsan violentamente al recuperar la presión aguas abajo de la restricción, generando micro-jets y ondas de presión transitorias muy destructivas. Estas ondas mecánicas y acústicas de alta frecuencia están totalmente fuera del alcance de las ecuaciones de momento promediadas de **THNet**.
4. **Histéresis y Descebado Físico**: Cuando una bomba sufre cavitación severa ("descebado"), suele requerir una purga manual para remover el aire/vapor acumulado en la carcasa antes de volver a operar. En el modelo fenomenológico, la bomba se recuperará instantáneamente en cuanto el $\text{NPSH}_A$ supere al $\text{NPSH}_R$, ignorando la presencia física del gas atrapado.

---

## 4. Conclusión

| Elemento | Parámetro Clave | Implementación en `THNet` | Fenómeno Capturado | Limitación |
| :--- | :--- | :--- | :--- | :--- |
| **Válvula** | Factor de recuperación $F_L$ | Escala $C_{v,\text{eff}}$ en la resistencia cuadrática | Bloqueo de caudal máximo (*Choked Flow*) | Sin volumen de vapor, sin golpe de ariete local |
| **Bomba** | Curva $\text{NPSH}_R(W)$ | Escala la curva H-Q (`dp_pump`) por factor $\sigma$ | Pérdida de carga y caudal, oscilaciones de cavitación | Recuperación instantánea sin descebado real |

Para análisis globales de transitorios termohidráulicos en plantas de proceso, el acoplamiento fenomenológico es sumamente robusto, numéricamente eficiente y suficiente. Si se requiere estudiar el daño estructural por implosión de burbujas o el transporte tridimensional de bolsas de vapor, se debe migrar a resolvedores bifásicos compresibles complejos (e.g. *Drift-Flux* o códigos CFD específicos).
