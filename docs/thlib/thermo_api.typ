#import "theme.typ": nota

== Diseño de la API Termodinámica (thermo-api)

Para garantizar que todos los bloques de la simulación utilicen propiedades físicas consistentes y permitir el intercambio de motores de cálculo (ej. IAPWS-IF97 real vs. Tablas de búsqueda optimizadas), se propone la siguiente arquitectura de software en Rust.

=== Estructura de Estado Termodinámico

En lugar de solicitar propiedades individuales (lo que obligaría a recalcular la región de estado repetidamente), la API devuelve un objeto `ThermoState` que contiene todas las propiedades relevantes para el punto solicitado.

```rust
pub struct ThermoState {
    pub p: f64,         // Presión [Pa]
    pub t: f64,         // Temperatura [K]
    pub rho: f64,       // Densidad [kg/m³]
    pub h: f64,         // Entalpía específica [J/kg]
    pub u: f64,         // Energía interna específica [J/kg]
    
    // Propiedades de transporte (opcionales)
    pub cp: f64,        // Calor específico a P constante [J/kg·K]
    pub cv: f64,        // Calor específico a V constante [J/kg·K]
    
    // Derivadas parciales para estabilidad numérica (Jacobianos)
    pub drho_dp_h: f64, // (∂ρ/∂p)_h -> Compresibilidad a entalpía constante
    pub drho_dh_p: f64, // (∂ρ/∂h)_p -> Expansión térmica a presión constante
}
```

=== La Trait ThermoLibrary

Esta interfaz define el contrato que debe cumplir cualquier motor termodinámico.

```rust
pub trait ThermoLibrary: Send + Sync {
    /// Identificador del motor (ej. "IAPWS-IF97", "WaterTable-100x100")
    fn name(&self) -> &str;

    /// Cálculo desde variables intensivas estándar (Presión, Entalpía)
    fn from_p_h(&self, p: f64, h: f64) -> ThermoState;

    /// Cálculo desde variables conservativas (Densidad, Energía Interna)
    /// Este es el método principal usado por los Headers y Celdas 1D.
    fn from_rho_u(&self, rho: f64, u: f64) -> ThermoState;

    /// Cálculo desde condiciones de borde (Presión, Temperatura)
    fn from_p_t(&self, p: f64, t: f64) -> ThermoState;
}
```

=== Implementaciones Previstas

1. *`IF97Backend`*: Utiliza la librería `seuif97` para cálculos exactos de agua/vapor según el estándar industrial. Ideal para validación y transitorios lentos donde la precisión es crítica.
2. *`TableBackend`*: Utiliza una grilla precalculada (Lookup Table) con interpolación bilineal o bicúbica. Es el motor recomendado para producción y tiempo real por su tiempo de ejecución constante ($O(1)$) y predecible.
3. *`LinearBackend`*: Modelo de fluido incompresible con propiedades constantes (ej. $rho = 1000$, $C_p = 4184$). Útil para pruebas rápidas y depuración de la lógica de control.

=== Uso en los Bloques de Simulación

Los bloques no instancian su propia librería, sino que reciben una referencia a una `dyn ThermoLibrary` a través del contexto de simulación o como un recurso inyectado.

```rust
impl SimulationBlock for HeaderBlock {
    fn tick(&mut self, ..., ctx: &Context) {
        // ... integración de m y u ...
        let thermo = ctx.get_thermo();
        let state = thermo.from_rho_u(self.mass / self.volume, self.u_total / self.mass);
        
        self.p = state.p;
        self.h = state.h;
        self.rho = state.rho;
    }
}
```

#nota[
  *Optimización:* El `TableBackend` es especialmente potente porque las derivadas parciales como `drho_dp_h` se pueden precalcular y almacenar directamente en la tabla, eliminando la necesidad de diferenciación numérica en tiempo de ejecución.
]
