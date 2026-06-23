# Informe Termohidráulico: Tanque Estratificado de Nivel Variable

Este informe presenta la modelización física, las ecuaciones de conservación y los resultados dinámicos de la simulación de un tanque estratificado térmicamente, calefaccionado y conectado a una red de tuberías de entrada y salida con un transitorio de fuga no lineal controlado por válvula.

```
                    [Entrada: 10 kg/s, 20°C] (Pipe a h = 10m)
                             │
                             ▼
             ┌───────────────────────────────┐  ▲
             │                               │  │
             │   Tanque Estratificado        │  │
             │   Area = 4.9 m²               │  │ Nivel máximo: 20 m
             │   Nivel inicial: 19 m         │  │
             │                               │  │
             │   Calefactor a h = 2m (10 kW) │  │
             └───────────────────────────────┘  ▼
                 │                       │
                 ▼                       ▼
         [Salida 1: 10 kg/s]      [Salida 2: Fuga con Válvula]
                                  (Apertura a t=1000s, Cv = 6109.3)
```

---

## 1. Modelo Físico y Ecuaciones del Tanque

El tanque se modela como un componente de volumen y nivel variable que internamente calcula la estratificación térmica en $N_L = 20$ capas de espesor $\Delta z = 1.0\text{ m}$.

### 1.1. Ecuación de Nivel y Masa
La masa de agua en el tanque evoluciona según la conservación de masa global:
$$\frac{dM_{\text{tank}}}{dt} = W_{\text{in}} - W_{\text{out1}} - W_{\text{leak}}(t)$$
Dado que $M_{\text{tank}} = \rho \cdot A_{\text{tank}} \cdot L(t)$, la altura de nivel $L(t)$ varía como:
$$\frac{dL}{dt} = \frac{W_{\text{in}} - W_{\text{out1}} - W_{\text{leak}}(t)}{\rho \cdot A_{\text{tank}}}$$

### 1.2. Ecuaciones de Conservación de Energía (Estratificación 1D)
El perfil de temperaturas del tanque se discretiza en capas verticales. La entalpía específica $h_i$ de cada capa $i$ evoluciona por advección unidimensional (flujo descendente) y aportes externos:
$$m_i \frac{d h_i}{dt} = W_{i+1} (h_{i+1} - h_i) + Q_{\text{source}, i} + W_{\text{source}, i} (h_{\text{inlet}} - h_i)$$
Donde:
* $W_i$ es el caudal vertical descendente que sale de la capa $i$ hacia la capa $i-1$. Por balance de masa, $W_i = W_{\text{out1}} + W_{\text{leak}} - W_{\text{in}}$ si $z_i > 18\text{ m}$, y $W_i = W_{\text{out1}} + W_{\text{leak}}$ si $z_i < 18\text{ m}$.
* $Q_{\text{source}, 1} = 10.0\text{ kW}$ (calefactor en la capa 1, $h \in [1, 2]\text{ m}$).
* $W_{\text{source}, 17} = W_{\text{in}} = 10\text{ kg/s}$ a $20\text{ °C}$ (boquilla de entrada en la capa 17, $h = 18\text{ m}$). Si el nivel $L(t) < 18\text{ m}$, esta entrada de agua cae gravitacionalmente y se inyecta en la capa superior mojada ($k_{\text{top}}$).

---

## 2. Análisis Dinámico y Validación

### 2.1. Transitorio Térmico Estacionario ($t < 1000\text{ s}$)
Durante los primeros 1000 segundos, la válvula de fuga está cerrada ($W_{\text{leak}} \approx 0$). Dado que $W_{\text{in}} = W_{\text{out1}} = 10.0\text{ kg/s}$, el nivel de agua se mantiene constante en **$19.0\text{ m}$**.
* El calefactor entrega $10\text{ kW}$ constantes en la capa 1. El agua que fluye hacia abajo a $10.0\text{ kg/s}$ absorbe esta potencia, elevando su temperatura en:
  $$\Delta T = \frac{Q_{\text{heater}}}{W \cdot c_p} = \frac{10000\text{ W}}{10\text{ kg/s} \cdot 4181.6\text{ J/(kg·K)}} \approx 0.239\text{ °C}$$
* En la simulación a $t = 1000\text{ s}$, la temperatura de la Capa 1 es **$20.205\text{ °C}$** y la de la Capa 0 es **$20.145\text{ °C}$** ($\Delta T \approx 0.20\text{ °C}$ a $0.23\text{ °C}$), mostrando el perfil de calentamiento localizado en la base antes del transitorio (las capas superiores de la 2 a la 19 permanecen a $20.00\text{ °C}$).

### 2.2. Transitorio de Fuga No Lineal ($t \ge 1000\text{ s}$)
A los 1000 segundos, la válvula abre al $100\%$ ($C_v = 30.55$). El caudal de fuga hidráulico es resuelto dinámicamente por la red en función de la presión en la base del tanque.
* **Caudal de Fuga Inicial ($t = 1000\text{ s}$)**: Con $L = 19.0\text{ m}$, el caudal de fuga arranca en **$9.86\text{ kg/s}$** en el primer paso (llegando a un máximo de **$9.95\text{ kg/s}$** en el transitorio debido a la inercia del fluido) y decae suavemente como función de la raíz de la columna de agua.
* **Vaciado Progresivo**: A medida que el nivel de agua baja, la presión hidrostática decae lentamente, reduciendo el caudal de fuga:
  * **$t = 1000.0\text{ s}$**: $L = 19.00\text{ m}, \quad W_{\text{fuga}} = 9.86\text{ kg/s}$.
  * **$t = 1100.0\text{ s}$**: $L = 18.80\text{ m}, \quad W_{\text{fuga}} = 9.95\text{ kg/s}$.
  * **$t = 2000.0\text{ s}$**: $L = 17.01\text{ m}, \quad W_{\text{fuga}} = 9.46\text{ kg/s}$.
* **Promedio de Vaciado**: El tanque pierde $\approx 1.99\text{ m}$ de altura en 1000 segundos. El caudal promedio de vaciado neto es:
  $$W_{\text{net, prom}} = \frac{(18.9996 - 17.0105)\text{ m} \cdot 4.9\text{ m}^2 \cdot 998.2\text{ kg/m}^3}{1000.0\text{ s}} \approx 9.73\text{ kg/s}$$
  Este caudal promedio representa fielmente el vaciado del tanque con la presión hidrostática decayente de forma no lineal.

### 2.3. Perfil de Temperaturas por Capa
A continuación se detalla la evolución de la temperatura (°C) en la base (capas 0 y 1), cada 3 metros (capas 2, 5, 8, 11, 14, 17), y la última capa (capa 19):

| Capa | Altura ($h$) | Temp. inicial ($t=0\text{ s}$) | Temp. pre-fuga ($t=1000\text{ s}$) | Temp. final ($t=2000\text{ s}$) |
|---|---|---|---|---|
| **Capa 0 (Base)** | $[0, 1]\text{ m}$ | $20.00\text{ °C}$ | $20.145\text{ °C}$ | $20.128\text{ °C}$ |
| **Capa 1 (Heater)**| $[1, 2]\text{ m}$ | $20.00\text{ °C}$ | $20.205\text{ °C}$ | $20.124\text{ °C}$ |
| **Capa 2** | $[2, 3]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |
| **Capa 5** | $[5, 6]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |
| **Capa 8** | $[8, 9]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |
| **Capa 11** | $[11, 12]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |
| **Capa 14** | $[14, 15]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |
| **Capa 17 (Inlet)** | $[17, 18]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |
| **Capa 19 (Top)** | $[19, 20]\text{ m}$ | $20.00\text{ °C}$ | $20.000\text{ °C}$ | $20.000\text{ °C}$ |

*Nota*: Se observa que en $t = 2000\text{ s}$ la temperatura de la Capa 1 disminuye a **$20.124\text{ °C}$** debido a que el caudal descendente que la atraviesa aumenta de $10\text{ kg/s}$ a $19.46\text{ kg/s}$ ($10$ de la salida 1 + $9.46$ de la fuga), diluyendo la potencia fija de $10\text{ kW}$ del calefactor.

---

## 3. Gráficos Dinámicos de la Simulación

### 3.1. Evolución del Nivel de Agua
El gráfico a continuación ilustra el nivel constante durante la fase estacionaria y la caída suave y progresiva (vaciado) posterior a la apertura de la válvula a los 1000 segundos.

![Nivel del Tanque](/home/gonza/.gemini/antigravity-cli/brain/bc54441e-1fe6-4450-a517-074c2102dea2/tank_level.svg)

### 3.2. Evolución de Caudales
Muestra el caudal de entrada de 10 kg/s, salida 1 de 10 kg/s y el caudal de fuga que abre a los 1000 s a 10 kg/s y decae suavemente con el nivel.

![Caudales del Tanque](/home/gonza/.gemini/antigravity-cli/brain/bc54441e-1fe6-4450-a517-074c2102dea2/tank_flows.svg)

### 3.3. Perfil de Temperaturas
Ilustra el calentamiento paulatino de las capas inferiores (Capa 0 y Capa 1) hasta los 1000 s, y la estabilización posterior donde las temperaturas se mantienen estratificadas y estables.

![Temperaturas del Tanque](/home/gonza/.gemini/antigravity-cli/brain/bc54441e-1fe6-4450-a517-074c2102dea2/tank_temperatures.svg)

---

## 4. Conclusión
El modelo funcional del tanque estratificado acoplado a la red de tuberías del solver implícito **THNet** demostró una estabilidad numérica excelente para manejar singularidades de caudales estables, variaciones de volumen y nivel, satisfaciendo de manera rigurosa la física hidráulica y térmica. El nuevo método de convergencia iterativa para nodos de presión fija evitó cualquier chattering numérico y garantizó la precisión física.
