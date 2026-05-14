# Especificación Técnica del Formato JSON

Este documento define el esquema de los archivos JSON utilizados por el simulador Rusty-Blocks para configurar el motor de simulación y definir la topología del sistema.

## 1. Configuración del Solver (Simulation Params)
Define el comportamiento del motor de integración numérica.

```json
{
  "dt": 0.01,        // Paso de tiempo (fijo para Euler/RK4, inicial para RK45)
  "t_final": 10.0,   // Tiempo total de simulación
  "solver": "RK45",  // Tipo: "Euler", "RK4", "RK45"
  "atol": 1e-8,      // Tolerancia absoluta (solo RK45)
  "rtol": 1e-4       // Tolerancia relativa (solo RK45)
}
```

## 2. Definición del Sistema (System Config)
Es el contenedor principal que agrupa bloques y sus interconexiones.

```json
{
  "name": "Nombre del Sistema",
  "blocks": [ ... ],       // Lista de objetos Block
  "connections": [ ... ]   // Lista de objetos Connection
}
```

### Bloques (Block Definition)
Cada bloque tiene un identificador único, un tipo y un objeto de parámetros específicos.

```json
{
  "id": "mi_bloque",
  "type": "TipoDeBloque",
  "params": { ... }
}
```

### Conexiones (Connection Definition)
Define el flujo de señales entre los puertos de salida y entrada.

```json
{
  "from": "id_bloque_origen",
  "from_port": 0,
  "to": "id_bloque_destino",
  "to_port": 0
}
```

---

## 3. Catálogo de Bloques Estándar

### Fuentes (Sources)
| Tipo | Parámetros | Descripción |
| :--- | :--- | :--- |
| `Constant` | `value: [f64]` | Genera un vector constante. |
| `Step` | `initial_value, final_value, step_time` | Salida escalar que cambia en un instante dado. |

### Matemáticos (Math)
| Tipo | Parámetros | Descripción |
| :--- | :--- | :--- |
| `Gain` | `k: f64, width: usize` | Multiplica un vector de entrada por un escalar. |
| `Sum` | `signs: String, width: usize` | Suma/resta entradas. Ej: `signs: "+-"` (2 entradas). |

### Tiempo Continuo (Continuous)
| Tipo | Parámetros | Descripción |
| :--- | :--- | :--- |
| `Integrator` | `ic: [f64]` | Integra el vector de entrada. `ic` son las cond. iniciales. |

### Ruteo (Routing)
| Tipo | Parámetros | Descripción |
| :--- | :--- | :--- |
| `Mux` | `input_widths: [usize]` | Combina múltiples puertos en un solo puerto vectorial. |
| `Demux` | `output_widths: [usize]` | Divide un puerto vectorial en múltiples puertos. |

### Sinks
| Tipo | Parámetros | Descripción |
| :--- | :--- | :--- |
| `FileSink` | `filename: String, interval: f64, width: usize` | Graba la señal en un CSV cada `interval` segundos. |

---

## 4. Subsistemas (Subsystems)
Un subsistema es un bloque cuyo tipo es `"Subsystem"`. Su objeto `params` es una definición de **System Config** completa.

```json
{
  "id": "planta",
  "type": "Subsystem",
  "params": {
    "name": "Caja Negra",
    "blocks": [
      { "id": "in1", "type": "InPort", "params": { "width": 1 } },
      { "id": "out1", "type": "OutPort", "params": { "width": 1 } }
    ],
    "connections": [ ... ]
  }
}
```

### Puertos de Interfaz
Se usan **exclusivamente** dentro de un subsistema para definir sus fronteras:
*   `InPort`: Recibe señales desde el exterior del subsistema.
*   `OutPort`: Envía señales hacia el exterior del subsistema.

---

## 5. Ejemplo Completo: Lazo de Control
```json
{
  "name": "Feedback Loop",
  "blocks": [
    { "id": "ref", "type": "Constant", "params": { "value": [1.0] } },
    { "id": "sum", "type": "Sum", "params": { "signs": "+-", "width": 1 } },
    { "id": "int", "type": "Integrator", "params": { "ic": [0.0] } },
    { "id": "sink", "type": "FileSink", "params": { "filename": "log.csv", "interval": 0.1, "width": 1 } }
  ],
  "connections": [
    { "from": "ref", "from_port": 0, "to": "sum", "to_port": 0 },
    { "from": "int", "from_port": 0, "to": "sum", "to_port": 1 },
    { "from": "sum", "from_port": 0, "to": "int", "to_port": 0 },
    { "from": "int", "from_port": 0, "to": "sink", "to_port": 0 }
  ]
}
```
