# Propuestas de Visualización Profesional - Rusty-Blocks

Este documento detalla la estrategia para transformar la visualización de datos en Rusty-Blocks, pasando de un gráfico estático a un sistema de monitoreo dinámico y profesional.

---

## 1. Bloque "Scope" Interactivo (Visualización en Tiempo Real)

**Concepto**: Un bloque dedicado a la inspección de señales que permite ver resultados específicos sin saturar el lienzo principal.

- **Doble Clic para Ver**: Al hacer doble clic en un bloque `Scope`, se abrirá un **Modal o Ventana Emergente** con un gráfico de alta resolución.
- **Persistencia**: El bloque recordará su configuración de visualización (escalas, colores de línea, límites de tiempo).
- **Múltiples Entradas**: Soporte para hasta 4 entradas por Scope para comparar señales (ej: Referencia vs Salida Real).
- **Implementación**: 
  - Usar un componente personalizado de React Flow para el `ScopeNode`.
  - Integrar un modal de `Headless UI` o `Radix UI`.
  - Utilizar `Recharts` con soporte para zoom y pan.

---

## 2. Recuperación de Sinks (Acceso a Archivos CSV)

**Concepto**: Puentes de comunicación entre el motor de simulación (que escribe archivos físicos) y la interfaz web.

- **Endpoint de Resultados**: Implementar en el servidor Axum un endpoint `/results/:filename`.
  - El motor de Rust guarda archivos como `output.csv`.
  - El frontend solicita el archivo mediante `fetch('http://localhost:3000/results/output.csv')`.
- **Parser Automático**: El frontend convertirá automáticamente el CSV crudo a un formato JSON compatible con los gráficos del navegador.
- **Descarga Directa**: Botón en la UI para descargar el CSV generado por la simulación directamente a la carpeta de descargas del usuario.

---

## 3. Centro de Resultados (Signal Monitoring Center)

**Concepto**: Una vista dedicada tipo "Dashboard" para analizar el comportamiento global del sistema después de la simulación.

- **Galería de Gráficos**: Una cuadrícula ajustable donde cada gráfico representa un `Scope` o un `FileSink`.
- **Sincronización de Tiempo**: Al mover el cursor sobre un gráfico, se mostrará una línea vertical guía en **todos** los gráficos de la galería para comparar estados en el mismo instante de tiempo.
- **Exportación de Reportes**: Capacidad de exportar la vista actual como un PDF o una imagen (PNG) para documentación técnica.
- **Implementación**:
  - Crear una nueva "Pestaña" o "Modo" en el frontend (alternar entre Canvas y Dashboard).
  - Usar `React Grid Layout` para permitir al usuario organizar sus gráficos.

---

## 4. Próximos Pasos Técnicos

### Backend (Rust)
1.  Crear carpeta `/sim_results` para centralizar archivos de salida.
2.  Añadir ruta `GET /results/:filename` en el servidor Axum.
3.  Asegurar que los Sinks tengan permisos de escritura en la carpeta de resultados.

### Frontend (React)
1.  Añadir el bloque `Scope` a la `BlockRegistry`.
2.  Implementar la lógica de "Pop-out" para gráficos.
3.  Desarrollar el componente `SignalGallery`.

---

**Autor**: Gemini CLI Agent  
**Fecha**: 14 de Mayo, 2026
