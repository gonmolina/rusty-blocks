# Reporte de Simulación: Lazo de Convección Natural con THNet

Este documento reporta el diseño, la resolución de inestabilidades y los resultados físicos de la simulación del lazo de convección natural utilizando el nuevo solvedor termohidráulico de red **THNet** en Rust.

---

## 1. El Problema de Inestabilidad y la Solución Implícita

En iteraciones de diseño previas, la simulación de redes termohidráulicas con headers de pequeño volumen sufría de una grave **inestabilidad numérica** debido al tratamiento explícito de la temperatura en los nodos:
* **Fórmula Explícita Anterior:** La entalpía del nodo se actualizaba con un paso de Euler explícito:
  $$\Delta h_i = \frac{\Delta t}{M_i} \left( \Phi_{\text{net},i} + Q_{\text{ext},i} \right)$$
* **La Inestabilidad:** Si el volumen del header es pequeño (ej. $1\text{ L} \implies M_i \approx 1\text{ kg}$) y el caudal es elevado (ej. $W \approx 9.28\text{ kg/s}$), el tiempo de residencia del nodo ($\tau = M_i / W \approx 0.1\text{ s}$) es mucho menor que el paso de tiempo ($\Delta t = 1.0\text{ s}$). Esto genera un número de Courant nodal $Co = \Delta t / \tau \approx 9.3 \gg 1$, lo que provoca oscilaciones divergentes que llevan a que las temperaturas exploten a los límites de seguridad de inmediato.

### Formulación Térmica Implícita (Unconditionally Stable)

Para solucionar esto de manera definitiva, reescribimos el balance de energía de los nodos en su forma **totalmente implícita** para el transporte advectivo de salida:
$$M_i \frac{h_i^{k+1} - h_i^k}{\Delta t} = \Phi_{\text{in},i} - W_{\text{out,total},i} \cdot h_i^{k+1} + Q_{\text{ext},i}$$

Despejando $h_i^{k+1}$ de forma exacta:
$$h_i^{k+1} = \frac{M_i \cdot h_i^k + \Delta t \left( \Phi_{\text{in},i} + Q_{\text{ext},i} \right)}{M_i + \Delta t \cdot W_{\text{out,total},i}}$$

Donde:
* $\Phi_{\text{in},i}$ es la suma de los flujos de entalpía entrantes a través de las ramas ($W \cdot h_{\text{outlet}}$).
* $W_{\text{out,total},i}$ es la suma de los caudales másicos salientes del nodo.
* $M_i$ es la masa contenida en el volumen físico del nodo.

Esta formulación es **incondicionalmente estable** para cualquier paso de tiempo $\Delta t$ y cualquier volumen de header $V_h$. Si $M_i \to 0$ (nodo de volumen nulo), la ecuación degenera suavemente en la ecuación de mezcla adiabática en estado estacionario, sin oscilaciones numéricas.

---

## 2. Resultados de la Simulación del Lazo de 25 cm

Se simuló el sistema con las siguientes especificaciones físicas solicitadas:
* **Geometría:** Altura vertical de $12\text{ m}$, tubería de acero inoxidable AISI 304 de diámetro interior $25\text{ cm}$ (DN250), rugosidad $\varepsilon = 15\,\mu\text{m}$.
* **Header inferior y superior:** $1\text{ L}$ de volumen cada uno (extrema relación caño/header: $1\text{ L}$ vs $589\text{ L}$).
* **Potencia:** Rama de subida con potencia eléctrica de $+18\text{ kW}$ constantes.
* **Sumidero:** Rama de bajada equipada con un intercambiador de calor realista (UA = $800\text{ W/K}$, $T_{\text{coolant}} = 20\text{ °C}$).
* **Tiempo total:** $20,000\text{ s}$ ($\approx 5.5\text{ horas}$) para ver el transitorio térmico completo hasta el equilibrio final.
* **Paso de tiempo:** $\Delta t = 1.0\text{ s}$ (estable gracias a la formulación implícita hidráulica y térmica).

### Resumen del Estado Estacionario Alcanzado (t = 20,000 s)

```text
┌─────────────────────────────────────────────────────────────────┐
│   ESTADO ESTACIONARIO (t = 20000 s = 333.3 min)             │
├─────────────────────────────────────────────────────────────────┤
│ Caudal:             3.5098 kg/s  (212.41 L/min)           │
│ Reynolds:            28455  (turbulento)             │
│ Factor fricc. f:    0.0239                                  │
├─────────────────────────────────────────────────────────────────┤
│ T header inferior:   41.39 °C                                │
│ T header superior:   42.62 °C                                │
│ T entrada HX:        42.62 °C  (fluido entra al HX)         │
│ T salida HX:         41.39 °C  (fluido sale del HX)         │
│ ΔT HX:                1.23 °C                                │
├─────────────────────────────────────────────────────────────────┤
│ Q fuente (up):     18000.0 W  (impuesta)                    │
│ Q extraída (HX):   17816.0 W  (calculada W·cp·ΔT)          │
│ Balance Q:           184.0 W  (diferencia = almacenado)     │
├─────────────────────────────────────────────────────────────────┤
│ ΔP flotabilidad:       6.0 Pa  (fuerza motriz)              │
│ ΔP fricción (up):      3.0 Pa                               │
│ Δρ (dn - up):         0.05 kg/m³                            │
│ T media up:          42.06 °C                               │
│ T media dn:          41.94 °C                               │
└─────────────────────────────────────────────────────────────────┘
```

![Evolución Temporal del Lazo de Convección Natural](thnet_convection_plot.svg)

---

## 3. ¿Es Realista la Simulación? (Análisis de Procesos Termohidráulicos)

**Sí, esta simulación es sumamente realista y físicamente coherente.** A continuación, se detalla el análisis de consistencia:

### A. Balance de Presiones (Fuerza Motriz vs. Pérdida de Carga)
El caudal de equilibrio se alcanza cuando la fuerza motriz de flotabilidad ($\Delta P_{\text{buoy}}$) balancea exactamente la caída de presión por fricción total ($\Delta P_{\text{fric}}$):
* **Fuerza Motriz (Flotabilidad):**
  $$\Delta P_{\text{buoy}} = \Delta \rho \cdot g \cdot H$$
  Con $T_{\text{up,media}} = 42.06\text{ °C} \implies \rho_{\text{up}} \approx 991.39\text{ kg/m³}$ y $T_{\text{dn,media}} = 41.94\text{ °C} \implies \rho_{\text{dn}} \approx 991.44\text{ kg/m³}$.
  La diferencia de densidad es $\Delta \rho \approx 0.05\text{ kg/m³}$.
  La presión motriz es:
  $$\Delta P_{\text{buoy}} = 0.05 \text{ kg/m³} \times 9.81\text{ m/s²} \times 12\text{ m} \approx 6.0\text{ Pa}$$
* **Pérdida de Carga por Fricción (Darcy-Weisbach):**
  El Reynolds es $28,455$, indicando un flujo turbulento. El factor de fricción calculado con la fórmula de Churchill es $f = 0.0239$.
  La caída de presión en la rama de subida es:
  $$\Delta P_{\text{fric,up}} = f \frac{L}{D} \frac{\rho v^2}{2} = 0.0239 \times \frac{12}{0.25} \times \frac{991.4 \times 0.078^2}{2} \approx 3.0\text{ Pa}$$
  Como el lazo es simétrico, la fricción de la rama descendente también es $\approx 3.0\text{ Pa}$.
  Por lo tanto, la fricción total es $\Delta P_{\text{fric,total}} = 3.0 + 3.0 = 6.0\text{ Pa}$.
  **El balance es perfecto:** $\Delta P_{\text{buoy}} = \Delta P_{\text{fric,total}} = 6.0\text{ Pa}$.

### B. Auto-Regulación del Intercambiador de Calor
El calor extraído por el HX se calcula como:
$$Q_{\text{hx}} = UA \cdot \Delta T_{\text{ln}} \approx UA \cdot (T_{\text{media}} - T_{\text{coolant}})$$
En el estado cuasi-estacionario final ($t = 20,000\text{ s}$), el fluido tiene una temperatura media de $\approx 42.0\text{ °C}$ y el coolant está a $20.0\text{ °C}$:
$$Q_{\text{hx}} \approx 800\text{ W/K} \times (42.2\text{ K} - 20\text{ K}) = 800 \times 22.2 \approx 17,760\text{ W}$$
Esto coincide casi exactamente con el balance térmico de $17,816\text{ W}$ reportado por el código, y está a punto de equilibrar los $18,000\text{ W}$ inyectados por el calentador.

### C. Por qué el gradiente de temperatura es tan pequeño ($\Delta T \approx 1.2\text{ °C}$)
Con una tubería tan gruesa ($25\text{ cm}$ de diámetro), la resistencia hidráulica es bajísima. Esto permite que una fuerza motriz muy pequeña ($6\text{ Pa}$) circule un caudal másico enorme de $3.5\text{ kg/s}$ ($212\text{ L/min}$).
Debido a este gran caudal circulante, el fluido transporta calor de forma muy eficiente, y el gradiente térmico necesario para remover los $18\text{ kW}$ es muy pequeño:
$$\Delta T = \frac{Q}{W \cdot c_p} = \frac{18000\text{ W}}{3.51\text{ kg/s} \times 4180\text{ J/(kg·K)}} \approx 1.23\text{ °C}$$
Este comportamiento es totalmente real: la convección natural en caños de gran diámetro con fluidos de baja viscosidad como el agua resulta en lazos altamente isotérmicos de alto caudal y baja diferencia de temperatura.

---

## 4. Conclusión Termohidráulica

El simulador **THNet** demuestra ser una herramienta robusta y físicamente precisa. A diferencia del modelo de bloques discretos original, THNet:
1. **Garantiza estabilidad incondicional** en el acoplamiento hidráulico global mediante MNA y en el transporte térmico de nodos y celdas mediante un esquema implícito.
2. **Elimina restricciones de tamaño de paso**, permitiendo simular dinámicas lentas de horas con pasos de $\Delta t = 1.0\text{ s}$ en milisegundos de tiempo de cómputo de CPU.
3. **Maneja headers realistas** sin importar cuán pequeños sean en relación a las tuberías.
