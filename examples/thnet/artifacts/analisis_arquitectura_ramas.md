# Análisis de Arquitectura: Separación de Componentes de Rama en THNet

Este documento presenta una evaluación de diseño sobre la conveniencia de refactorizar la estructura actual de ramas (`Pipe` como struct monolítico) hacia un esquema desacoplado mediante **Enums** o **Traits** en Rust, analizando las ventajas, desventajas e impacto en el resolvedor termohidráulico **THNet**.

---

## 1. Estado Actual (Modelo Monolítico)

Actualmente, no existe una distinción en los tipos de ramas; todas las ramas de la red son instancias de la estructura `Pipe`, la cual contiene campos para todos los componentes posibles simultáneamente:

```rust
pub struct Pipe {
    // 1. Tubería estándar y térmica
    pub diameter: f64,
    pub length: f64,
    pub roughness: f64,
    pub elevation_dz: f64,
    pub n_cells: usize,
    pub cell_temp: Vec<f64>,
    
    // 2. Inercia de pared
    pub wall_mass: f64,
    pub wall_cp: f64,
    pub wall_ua: f64,
    pub wall_temp: Vec<f64>,

    // 3. Bomba
    pub pump_dp_max: f64,
    pub pump_w_max: f64,
    pub pump_coefs: Option<[f64; 3]>,
    pub pump_speed_ratio: f64,

    // 4. Válvula de control
    pub valve_cv: f64,
    pub valve_opening: f64,
    pub valve_char: ValveChar,

    // 5. Válvula de retención
    pub is_check_valve: bool,
    pub check_valve_closed: bool,

    // 6. Intercambiador de calor e inyección
    pub ua_hx: f64,
    pub t_coolant: f64,
    pub heat_total: f64,
}
```

### Problemas del Modelo Monolítico:
1. **Desperdicio de Memoria (Bloat)**: Para una tubería simple (el 90% de las ramas de una red grande), la estructura almacena espacio en memoria para coeficientes de bomba, estados de válvula de control y de retención, vectores de temperatura de pared, etc.
2. **Violación del Principio de Responsabilidad Única (SRP)**: La lógica para linealizar la resistencia en `resistance_linearized` y calcular el momento en `solver.rs` está repleta de bloques condicionales (`if self.valve_cv > 0.0`, `if self.pump_dp_max > 0.0`, etc.) apilando efectos de manera acoplada.
3. **Limitación de Topología**: No es posible definir una rama que contenga, por ejemplo, dos válvulas en serie sin crear nodos intermedios artificiales.
4. **Dificultad de Extensión**: Agregar un nuevo tipo de componente (por ejemplo, un filtro, una turbina o una restricción de placa de orificio) exige agregar más campos a `Pipe` y modificar el código del resolvedor central, aumentando el riesgo de regresiones.

---

## 2. Alternativas de Diseño

Para desacoplar las ramas, podemos optar por dos patrones principales en Rust:

### Alternativa A: Polimorfismo basado en Enums (Algebraic Data Types)

Bajo este enfoque, definimos una estructura general `Branch` que maneja la conectividad y el caudal, y delega el comportamiento específico a un `enum`:

```rust
pub struct Branch {
    pub node_up: NodeId,
    pub node_dn: NodeId,
    pub flow: f64,
    pub component: BranchComponent,
}

pub enum BranchComponent {
    Pipe(PipeComponent),
    Pump(PumpComponent),
    Valve(ValveComponent),
    CheckValve(CheckValveComponent),
    Orifice(OrificeComponent),
}
```

* **PipeComponent**: Contiene solo geometría, rugosidad, discretización y celdas térmicas.
* **PumpComponent**: Contiene coeficientes de curva, velocidad y leyes de afinidad.
* **ValveComponent**: Contiene $Cv$, apertura y curvas características.

### Alternativa B: Polimorfismo dinámico basado en Traits (Abstracción Limpia)

Definimos un trait en `thnet` que expone el comportamiento que el solver MNA requiere de cualquier rama:

```rust
pub trait NetworkBranch: Send + Sync {
    fn resistance_linearized(&self, flow: f64) -> f64;
    fn pressure_gain(&self, flow: f64) -> f64;
    fn pressure_gain_derivative(&self, flow: f64) -> f64;
    fn step_thermal(&mut self, flow: f64, dt: f64, temp_up: f64) -> f64;
}
```

La red almacenaría colecciones de objetos polimórficos:
```rust
pub struct Network {
    pub nodes: Vec<Node>,
    pub branches: Vec<Box<dyn NetworkBranch>>,
}
```

---

## 3. Tabla Comparativa

| Criterio | Modelo Monolítico | Alternativa A (Enums) | Alternativa B (Traits) |
| :--- | :--- | :--- | :--- |
| **Rendimiento CPU / Caché** | ⭐⭐⭐ (Rápido, datos contiguos y planos) | ⭐⭐⭐ (Muy rápido, sin dynamic dispatch, tamaño de enum acotado) | ⭐⭐ (Menor, debido a redirección por punteros vtable y falta de inlining) |
| **Uso de Memoria** | ⭐ (Malo, cada rama tiene overhead de campos vacíos) | ⭐⭐⭐ (Excelente, se asigna solo la memoria del componente activo) | ⭐⭐ (Uso de memoria óptimo por elemento, pero con overhead del Box/punteros) |
| **Simplicidad de Extensión** | ⭐ (Requiere modificar `Pipe` y el core del `Solver`) | ⭐⭐ (Requiere agregar variante al enum y manejarla en los match del resolvedor) | ⭐⭐⭐ (Excelente, permite agregar nuevos componentes en otros crates sin tocar `thnet`) |
| **Claridad del Código (SRP)** | ⭐ (Acoplado, funciones gigantescas llenas de condicionales) | ⭐⭐⭐ (Excelente, cada tipo de componente tiene su propia lógica en su módulo) | ⭐⭐⭐ (Excelente, total aislamiento de lógica por componente) |
| **Clonación y Serialización** | ⭐⭐⭐ (Fácil y directo) | ⭐⭐⭐ (Fácil y directo) | ⭐ (Complejo, requiere patrones "prototype" para clonar `Box<dyn>`) |

---

## 4. ¿Qué se gana y qué se pierde?

### Ventajas de Separar (Ganancia)
1. **Lógica Aislada y Legible**: El resolvedor hidráulico no tiene que saber qué es una bomba o una válvula; simplemente pide la conductancia y los términos fuentes de la rama.
2. **Eficiencia en Memoria**: Para redes grandes, la reducción de la huella de memoria es de aproximadamente un **60% a 70%** por tubería.
3. **Escalabilidad del Resolvedor**: Agregar un nuevo componente como un intercambiador de calor complejo o una turbina hidráulica no toca el código de resolución de MNA ni de Newton-Raphson.

### Desventajas de Separar (Pérdidas/Costo)
1. **Boilerplate**: Se introduce una pequeña capa de indirección para acceder a variables comunes.
2. **Esquema Térmico**: El transporte de calor es continuo a lo largo de las ramas. Si una rama es una bomba y otra es una válvula, ¿tienen capacidad térmica o celdas de temperatura?
   * *Solución típica*: Solo el tipo `Pipe` contiene celdas térmicas y volumen de fluido real. Válvulas y bombas se tratan como componentes "concentrados" (sin volumen físico ni inercia térmica de pared), lo cual simplifica enormemente la térmica del lazo.

---

## 5. Recomendación Arquitectónica

Para un resolvedor de simulación numérica en ingeniería, la **Alternativa A (Enums)** es la opción recomendada. 
* **Por qué**: En simulación de sistemas termohidráulicos, el conjunto de componentes básicos es sumamente acotado y estable en el tiempo (tuberías, bombas, válvulas, orificios, check-valves). La velocidad de procesamiento en bucles de cálculo intensivos es crítica, y los Enums en Rust compilan a instrucciones de máquina extremadamente eficientes sin el overhead de despacho dinámico y des-referenciación de memoria que introducen los `Box<dyn Trait>`.
