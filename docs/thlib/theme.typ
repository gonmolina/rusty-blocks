// Estilo para notas destacadas
#let nota(body) = rect(
  fill: rgb("f1f5f9"),
  stroke: (left: 4pt + rgb("0284c7")),
  inset: (x: 12pt, y: 10pt),
  radius: (right: 4pt),
  width: 100%,
  body,
)
