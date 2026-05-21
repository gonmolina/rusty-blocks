#set document(title: "Simulación Termohidráulica en Tiempo Real con Rust", author: "Simulación e Ingeniería")// --- CONFIGURACIÓN DE DISEÑO EDITORIAL ---#set page(paper: "a4",margin: (top: 3cm, bottom: 3cm, left: 2.5cm, right: 2.5cm),header: align(right)[#text(size: 8pt, fill: rgb("666666"))[Simulación Termohidráulica en Rust | Código de Componentes]],footer: locate(loc => {let page_number = counter(page).at(loc).first()let total_pages = counter(page).final(loc).first()if page_number > 1 {align(center)[#text(size: 9pt, fill: rgb("444444"))[Página #page_number de #total_pages]]}}))#set text(font: "Liberation Serif",size: 11pt,lang: "es")#set heading(numbering: "1.1.")#show heading: it => {set text(font: "Liberation Sans", fill: rgb("1b365d"))block(above: 1.5em, below: 1em)[#it#v(0.2em)]}// Estilo para bloques de código#show raw.where(block: true): it => block(fill: rgb("f8f9fa"),inset: 12pt,radius: 4pt,stroke: 0.5pt + rgb("e9ecef"),width: 100%,text(size: 8.5pt, font: "DejaVu Sans Mono", it))// Estilo para notas destacadas#let nota(body) = rect(fill: rgb("f1f5f9"),stroke: (left: 4pt + rgb("0284c7")),inset: (x: 12pt, y: 10pt),radius: (right: 4pt),width: 100%,body)// --- PORTADA ---#place(top + left, dy: 5cm)[#align(center)[#text(size: 28pt, weight: "bold", font: "Liberation Sans", fill: rgb("1b365d"))[Simulación Termohidráulica\ de Tiempo Real en Rust]#v(1cm)
#text(size: 16pt, style: "italic", fill: rgb("475569"))[
  Arquitectura de Componentes, Modelos Matematicos y Código Fuente para Sistemas 0D/1D Distribuidos
]

#v(2cm)
#line(length: 60%, stroke: 2pt + rgb("1b365d"))
#v(2cm)

#grid(
  columns: (1fr, 1fr),
  align(right)[
    #text(weight: "bold")[Preparado para:] \
    Biblioteca de Componentes Físicos \
    Orquestación por Diagrama de Bloques
  ],
  align(left)[
    #hspace(2cm)
    #text(weight: "bold")[Lenguaje de Implementación:] \
    #hspace(2cm) Rust (Edición 2021) \
    #hspace(2cm) Rendimiento Monofásico Explicito
  ]
)

#v(4cm)
#text(size: 10pt, fill: rgb("64748b"))[
  Generado automáticamente | Entorno de Compilación Typst
]
]]#pagebreak()// --- TABLA DE CONTENIDOS ---#outline(depth: 3, indent: 1.5em)#pagebreak()// --- INCLUSIÓN DE CAPÍTULOS ---= Componentes de Conexión y Distribución de Flujo (Resistencias)#include "pipe_1d.typ"#pagebreak()#include "cent_pump.typ"#pagebreak()= Componentes Volumétricos de Acumulación (Capacitancias)#include "header.typ"#pagebreak()#include "open_tank.typ"#pagebreak()#include "closed_tank.typ"#pagebreak()#include "stratified_tank.typ"
