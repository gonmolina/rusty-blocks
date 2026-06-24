# THNet — Limitaciones Físicas, Supuestos Matemáticos e Impacto en Ejemplos
## Documento Técnico v0.1

Este documento detalla las hipótesis simplificativas del modelo físico-matemático del solvedor termohidráulico **THNet** y analiza críticamente cómo afectan la precisión e interpretación de los resultados en los ejemplos de simulación provistos.

---

## 1. Clasificación de Limitaciones y Supuestos Físicos

### 1.1 Hipótesis de Incompresibilidad Hidráulica
En la formulación de conductancia nodal de red (análogo a MNA), se asume que la densidad del fluido en las ecuaciones de conservación de masa nodal responde de forma instantánea ante cambios de presión:
$$\nabla \cdot \vec{v} = 0 \quad \implies \quad \sum_{j} W_{j} = 0 \quad \text{(en nodos sin acumulación)}$$

* **Ondas de Presión Acústicas (Golpe de Ariete):**
  Al considerarse el agua como hidráulicamente incompresible en el lazo global de resolución rápida, la velocidad de propagación de las ondas de presión (velocidad del sonido $c \approx 1400\text{ m/s}$) es efectivamente **infinita**.
  * **Limitación:** El solvedor no puede modelar la propagación de ondas acústicas ni transitorios de choque rápidos provocados por maniobras rápidas en válvulas o bombas.
* **Rigidez Volumétrica en Sistemas Cerrados:**
  Si un sector de la red o la red completa se encuentra cerrada herméticamente y llena de líquido incompresible, cualquier inyección de masa o expansión térmica generará una inconsistencia matemática (matriz de conductancia singular o indeterminada), requiriendo un nodo de presión impuesta (Dirichlet) o un modelo de compresibilidad local.

### 1.2 Caudal Másico Uniforme a lo Largo de Ramas (Pipes)
El caudal másico $W_j$ de una rama `Pipe` se modela como espacialmente uniforme:
$$\frac{\partial W_j}{\partial x} = 0$$

* **Limitación:** No se permite el almacenamiento o acumulación transitoria de masa debido a dilatación o compresión dentro de una tubería, aun cuando esta se discretice en múltiples celdas térmicas. El balance de masa del lazo hidráulico rápido se evalúa únicamente en los nodos de interconexión (extremos de las tuberías).
* **Ausencia de Dilatación Térmica Local:** Si el fluido se calienta en una celda intermedia de la tubería, la expansión térmica volumétrica real debería empujar fluido hacia ambos extremos del caño. En el modelo de THNet, la masa entrante al caño es instantáneamente igual a la saliente en todo momento.

### 1.3 Acoplamiento Térmico-Hidráulico Desacoplado (Operator Splitting)
El solvedor realiza la integración temporal en dos etapas desacopladas dentro del paso de tiempo $\Delta t$:
1. **Paso Hidráulico (Implícito):** Se resuelven las presiones nodales $P^{k+1}$ y caudales másicos $W^{k+1}$ usando las temperaturas $T^k$ y densidades $\rho^k$ del paso anterior.
2. **Paso Térmico (Upwind):** Se resuelve el transporte y balance de entalpía y temperatura utilizando los caudales $W^{k+1}$ ya calculados.

* **Limitación:** Existe un desfase o retraso de un paso de tiempo ($\Delta t$) en el acoplamiento de las fuerzas de flotabilidad (termosifón). El cambio térmico ocurrido en el paso $k$ recién altera el campo hidráulico en el paso $k+1$.
* **Estabilidad del Lazo Térmico:** Si el paso de tiempo $\Delta t$ es muy grande en comparación con la velocidad del fluido y la constante térmica del sistema, este desacoplamiento puede inducir oscilaciones numéricas artificiales en sistemas con fuerte acoplamiento natural.

### 1.4 Simplificación en el Término de Inercia Hidráulica
La ecuación de momento transitoria por rama se discretiza como:
$$I_j \frac{W_j^{k+1} - W_j^k}{\Delta t} = \Delta P^{k+1} - \Delta P_{\text{fric}} + \Delta P_{\text{grav}} + \Delta P_{\text{bomba}}$$
Donde la inercia es $I_j = \frac{L_j}{A_j}$ (o multiplicada por una densidad media constante de la rama).

* **Limitación:** Se omite el acoplamiento dinámico del perfil espacial de densidad transitorio $\frac{\partial (\rho_j)}{\partial t}$ y el término convectivo de momento $\frac{\partial (W v)}{\partial x}$ a lo largo de la tubería. Se asume que la inercia depende puramente de la geometría física del caño y que las densidades medias varían lentamente.

### 1.5 Transición Laminar-Turbulento Suavizada
Se utiliza el factor de fricción de Churchill (1977) para modelar las pérdidas de carga en todo el rango de Reynolds ($Re$).
* **Limitación:** La física real en la zona de transición ($2000 < Re < 4000$) presenta inestabilidades estocásticas y bifurcaciones de flujo. La formulación de Churchill suaviza de forma continua esta transición para asegurar la estabilidad del solucionador de Newton-Raphson, eliminando oscilaciones físicas de baja frecuencia asociadas a la inestabilidad de la turbulencia transicional.

---

## 2. Impacto Específico en los Ejemplos Desarrollados

### 2.1 Ejemplo 1: Lazo de Convección Natural (`conv_natural.json`)
* **Descripción:** Lazo vertical de $12\text{ m}$ con calentamiento constante ($18\text{ kW}$) en la rama de subida y enfriamiento en la de bajada.
* **Impacto de las Limitaciones:**
  * **Desacoplamiento Térmico-Hidráulico:** Al arrancar desde el reposo ($W = 0$), el retraso de $\Delta t$ en las fuerzas de flotabilidad podría generar un retraso en la aceleración inicial. No obstante, con un $\Delta t = 1.0\text{ s}$ y una constante de tiempo térmica del caño del orden de $\sim 100\text{ s}$, el acoplamiento es físicamente representativo y el error numérico es insignificante (<0.1% en estado estacionario).
  * **Ausencia de Dilatación Térmica Local:** Al encender el calentador, el agua se expande físicamente en la rama ascendente. En un lazo real, esto causa transitorios de masa divergentes (el fluido se expande empujando hacia arriba y hacia abajo simultáneamente). THNet fuerza un flujo másico perfectamente uniforme en todo el anillo en todo instante. Esto oculta un pequeño transitorio de flujo oscilatorio de muy corta duración en el arranque, pero no altera en absoluto el estado estacionario ni las dinámicas térmicas de mediano y largo plazo.
  * **Transición de Fricción:** Durante el arranque, el flujo pasa de reposo ($Re = 0$) a turbulento ($Re \approx 28000$). La curva continua de Churchill asegura que el solvedor hidráulico converja sin oscilar en el rango transicional de $Re$, proporcionando una curva de aceleración del caudal térmico suave y físicamente coherente.

### 2.2 Ejemplo 2: Transitorio de Válvulas y Bombas (`bomba_valvula_cano.json`)
* **Descripción:** Línea de flujo con una bomba centrífuga, una válvula de control de apertura variable y una tubería larga.
* **Impacto de las Limitaciones:**
  * **Ondas Acústicas (Golpe de Ariete):** **Este es el punto crítico de mayor error.** Si la simulación incluye el cierre rápido de la válvula (en menos de $\sim 0.5\text{ s}$), el modelo hidráulico de THNet mostrará que el flujo se detiene de inmediato y la presión antes de la válvula sube instantáneamente a la presión de corte de la bomba.
    * *Error:* Oculta por completo la onda elástica de sobrepresión que superaría con creces la presión estática de la bomba, lo que podría provocar la rotura física del caño en la realidad.
    * *Mitigación:* La simulación solo es válida para maniobras lentas donde el tiempo de actuación de la válvula sea mucho mayor al tiempo de tránsito de la onda acústica en el caño ($t_{\text{actuacion}} \gg 2L/c$).
  * **Inercia del Fluido:** Al incluir el término $I \frac{dW}{dt}$, el solvedor captura adecuadamente el retardo inercial macroscópico del fluido acelerando o desacelerando en el caño tras un cambio en la válvula o bomba, limitando la tasa de cambio del caudal a niveles realistas (segundos), lo cual es correcto a nivel macroscópico.

### 2.3 Ejemplo 3: Tanque Estratificado (`tanque_estratificado.json`)
* **Descripción:** Tanque vertical con celdas de entalpía y nivel dinámico que se llena/vacía y se calienta.
* **Impacto de las Limitaciones:**
  * **Incompresibilidad:** La hipótesis de incompresibilidad del líquido es excelente. Las variaciones de masa en el tanque se calculan mediante integración temporal del nivel libre ($dH/dt = W_{\text{net}} / (\rho A)$) al final de cada paso de tiempo de forma estable y acoplada.
  * **Estratificación 1D:** Al asumir que la temperatura varía solo en la dirección vertical y que es perfectamente uniforme en la sección transversal, se omiten efectos tridimensionales de mezcla (como plumas convectivas locales o chorros de entrada con mezcla radial). Esto puede sobreestimar la nitidez de la termoclina en situaciones con altas velocidades de entrada.

### 2.4 Ejemplo 4: Dos Tanques (`dos_tanques.json`)
* **Descripción:** Trasvase de fluido por gravedad entre dos tanques abiertos a distintas alturas geométricas.
* **Impacto de las Limitaciones:**
  * **Incompresibilidad:** Las dinámicas del trasvase son sumamente lentas. La incompresibilidad es una hipótesis idónea. El error introducido por la ausencia de efectos acústicos o elásticos es nulo a fines prácticos, y el resolvedor MNA proporciona una convergencia de presiones nodales exacta.

---

## 3. Resumen y Guía para el Ingeniero de Simulación

Para asegurar que las simulaciones con **THNet** sean representativas de la realidad, se deben seguir las siguientes directrices operativas:

1. **Evitar Transitorios Ultra-Rápidos:** No utilizar el resolvedor para cuantificar tensiones estructurales por golpe de ariete o transitorios de milisegundos. Las maniobras de válvulas y bombas deben modelarse con rampas de control realistas y lentas.
2. **Uso Obligatorio de Puntos Dirichlet (Referencias):** Al ser incompresible, la red hidráulica requiere al menos un nodo de presión fija (un `OpenTank`, un `ClosedTank` o una condición Dirichlet impuesta) para servir de referencia de presión. De lo contrario, el solucionador matricial fallará por indeterminación de la presión absoluta del sistema.
3. **Control del Paso de Tiempo ($\Delta t$):** Para fenómenos termosifónicos rápidos, seleccionar un $\Delta t$ que cumpla con un criterio Courant o térmico moderado, evitando que el desacoplamiento numérico del *operator splitting* genere oscilaciones artificiales de caudal y temperatura.
