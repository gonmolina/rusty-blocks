# Informe de Simulación: Motor CC con Eje Flexible (4º Orden)

## 1. Descripción del Sistema
El sistema simula la dinámica de un motor de corriente continua conectado a una carga mecánica a través de un eje con cierta flexibilidad (torsión). Este es un problema clásico de control de vibraciones en sistemas electromecánicos.

El modelo se descompone en tres partes principales:
*   **Motor:** Recibe una tensión de entrada $V$ y genera un par motor, afectado por su inercia ($J_m$) y fricción ($B_m$).
*   **Eje Flexible:** Actúa como un muelle torsional con una constante de rigidez ($K_s$) y un amortiguamiento interno ($B_s$). Transmite par entre el motor y la carga.
*   **Carga:** Posee su propia inercia ($J_l$) y fricción ($B_l$), y es arrastrada por el par transmitido a través del eje.

## 2. Modelo Matemático
El sistema de cuarto orden está definido por las siguientes ecuaciones diferenciales:

$$
\begin{aligned}
\dot{\theta}_m &= \omega_m \\
\dot{\omega}_m &= \frac{1}{J_m} [V - B_{tot,m} \omega_m - \tau_{eje}] \\
\dot{\theta}_l &= \omega_l \\
\dot{\omega}_l &= \frac{1}{J_l} [\tau_{eje} - B_l \omega_l] \\
\tau_{eje} &= K_s (\theta_m - \theta_l) + B_s (\omega_m - \omega_l)
\end{aligned}
$$

Donde:
*   $\theta_m, \theta_l$: Ángulos del motor y la carga (rad).
*   $\omega_m, \omega_l$: Velocidades angulares (rad/s).
*   $\tau_{eje}$: Par transmitido por el eje flexible (Nm).

## 3. Parámetros de la Simulación
Basado en la configuración `motor_flexible.json`:

| Parámetro | Símbolo | Valor | Descripción |
| :--- | :--- | :--- | :--- |
| Tensión Entrada | $V$ | 10.0 V | Escalón en $t = 1.0s$ |
| Inercia Motor | $J_m$ | 0.5 $kg \cdot m^2$ | ($1/J_m = 2.0$) |
| Fricción Motor | $B_{tot,m}$ | 1.1 $Nms/rad$ | Incluye efecto contra-electromotriz |
| Rigidez Eje | $K_s$ | 10.0 $Nm/rad$ | Constante de muelle del eje |
| Amort. Eje | $B_s$ | 0.2 $Nms/rad$ | Fricción interna del eje |
| Inercia Carga | $J_l$ | 1.0 $kg \cdot m^2$ | ($1/J_l = 1.0$) |
| Fricción Carga | $B_l$ | 0.1 $Nms/rad$ | Pérdidas en la carga |

## 4. Resultados de la Simulación
La simulación se realizó con el solver **RK45** (paso variable) desde $t=0$ hasta $t=10s$.

**Estado Final ($t \approx 10.16$ s):**
*   **Velocidad Motor ($\omega_m$):** 8.44 rad/s
*   **Ángulo Motor ($\theta_m$):** 66.11 rad
*   **Velocidad Carga ($\omega_l$):** 8.30 rad/s
*   **Ángulo Carga ($\theta_l$):** 66.07 rad

## 5. Conclusiones
1.  **Sincronismo:** Al final de la simulación, las velocidades $\omega_m$ y $\omega_l$ son casi idénticas, lo que indica que el sistema está alcanzando el estado estacionario después de la transitoria provocada por el escalón.
2.  **Flexibilidad:** La pequeña diferencia entre $\theta_m$ y $\theta_l$ (aprox. 0.04 rad) representa la deformación por torsión del eje necesaria para transmitir el par necesario para vencer la fricción de la carga.
3.  **Estabilidad:** El uso de RK45 permitió capturar la dinámica oscilatoria inicial sin inestabilidades numéricas, ajustando el paso de tiempo automáticamente ante el cambio brusco de voltaje.

---
*Informe generado automáticamente por el Simulador de Bloques.*
