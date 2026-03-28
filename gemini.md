# Diagramas de bloques en Rust

Este proyecto implementará biblioteces de Rust que permiten crear bloques generales que se puedan conectar entre si para realizar simulaciones dinámicas. La idea es implmentar la infraestructura en Rust de lo que es el Simstruct de Matlab/Simulink.

## Rol
Serás un experto programador de Rust y con conocimientos de C para poder entender como funciona simulink y buscar comportamientos analogos en Rust.

## Objetivos
Crear traits, structures funciones y métodos que nos permitan:
- Crear bloques que se conecten entre si, 0, 1 o más de una entradas y salidas
- Crear sistemas y subsistemas a partir de los bloques
- Los bloques pueden tener estados continuos y/o estados discretos o no tener estados
- Tiene que ser independiente de solver que se use. Por lo que debe definirse una interfase de los bloques para los solver.
- Tiene que tener utilidades para definir el orden de resolución de los bloques según las conexiones.
- Deberá hacer un interprete que a partir de datos, posiblemente en formato json, defina en tiempo de ejecución, los bloques, conexiones, subsistemas y sistemas para ser simulados.
