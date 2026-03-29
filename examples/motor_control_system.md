# Informe de Simulación: Control de Velocidad de Motor con Eje Flexible

Este informe detalla el diseño y los resultados de la simulación de un sistema de control de lazo cerrado para un motor de corriente continua (CC) con carga flexible, utilizando una arquitectura de subsistemas jerárquicos.

## 1. Estructura del Modelo
El sistema está compuesto por dos grandes bloques funcionales (subsistemas) conectados en feedback:

1.  **Subsistema Controlador PI**: Procesa el error de velocidad y genera la tensión de control.
2.  **Subsistema Planta (Motor Flexible)**: Representa la dinámica física del motor y la carga acoplada.

### A. Subsistema Controlador PI
Implementa una ley de control proporcional-integral:
$$V(t) = K_p \cdot e(t) + K_i \int e(t) dt$$
Donde $e(t) = \omega_{ref} - \omega_l$ es el error de velocidad de la carga.

*   **Configuración**:
    *   $K_p = 2.0$
    *   $K_i = 5.0$

### B. Subsistema Planta (Motor + Eje)
Es un sistema de cuarto orden con los siguientes estados:
*   $\theta_m, \theta_l$: Ángulos del motor y carga.
*   $\omega_m, \omega_l$: Velocidades del motor y carga.

El acoplamiento entre motor y carga está definido por la rigidez $K_s$ y el amortiguamiento $B_s$ del eje flexible.

## 2. Parámetros Físicos
| Componente | Parámetro | Valor |
| :--- | :--- | :--- |
| **Referencia** | Velocidad deseada ($\omega_{ref}$) | 5.0 rad/s (escalón en $t=1$) |
| **Motor** | Inercia ($J_m$) | 0.5 $kg \cdot m^2$ |
| **Eje** | Rigidez ($K_s$) | 10.0 $Nm/rad$ |
| **Eje** | Amortiguamiento ($B_s$) | 0.2 $Nms/rad$ |
| **Carga** | Inercia ($J_l$) | 1.0 $kg \cdot m^2$ |

## 3. Configuración de la Simulación
*   **Archivo**: `examples/motor_control_system.json`
*   **Solver**: RK45 (Paso Adaptativo).
*   **Tiempo Final**: 10.0 segundos.
*   **Muestreo**: 0.05 segundos (vía `FileSink`).

## 4. Resultados Obtenidos
La simulación genera un archivo `control_motor.csv` que contiene la evolución de la referencia y la velocidad real de la carga.

### Observaciones del Comportamiento:
1.  **Tiempo de Respuesta**: Tras el escalón en $t=1$, el sistema comienza a acelerar.
2.  **Dinámica del Eje**: Se observa una ligera oscilación inicial en la velocidad de la carga debido a la flexibilidad del eje ($K_s=10$).
3.  **Lazo Cerrado**: El controlador PI compensa la fricción y la carga del eje, llevando la velocidad $\omega_l$ hacia la referencia de 5.0 rad/s.
4.  **Estabilidad**: El uso de RK45 garantiza que las oscilaciones de alta frecuencia del eje flexible sean integradas con precisión sin inestabilidades numéricas.

## 5. Conclusión Técnica
Este ejemplo valida la capacidad del simulador para manejar:
*   **Sistemas de Alto Orden**: Un sistema total de 5º orden (4 de planta + 1 del controlador).
*   **Jerarquía**: Los bloques internos de los subsistemas se ejecutan en su propio orden topológico, pero sincronizados con el reloj global del solver.
*   **Feedback**: La conexión de salida de la planta a la entrada del controlador funciona correctamente sin bucles algebraicos gracias a que la planta (vía los integradores) no tiene *direct feedthrough*.

---
*Simulación ejecutada con Rusty-Blocks Engine.*
