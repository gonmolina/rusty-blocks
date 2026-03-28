# Ejemplo: Realimentación Negativa (Feedback Loop)

Este ejemplo simula un sistema dinámico clásico de primer orden definido por la ecuación diferencial:

$$\dot{x} = -5x$$

Con una condición inicial $x(0) = 10$. La solución analítica es $x(t) = 10 \cdot e^{-5t}$.

## Estructura del Sistema

- **Integrador**: Almacena el estado $x$ y tiene una condición inicial de 10.0.
- **Gain (Ganancia)**: Multiplica la salida del integrador por -5.0.
- **Conexión**: La salida de la ganancia se realimenta a la entrada del integrador.

## Cómo Simularlo

Para ejecutar este sistema con los parámetros de simulación predeterminados (Euler, dt=0.01):
```bash
cargo run -- examples/feedback_system.json
```

Para usar un solver de mayor precisión como RK4 o RK45:
```bash
cargo run -- examples/feedback_system.json examples/sim_rk45.json
```
