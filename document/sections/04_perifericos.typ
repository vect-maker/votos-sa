#let peripherals = csv("../assets/peripherals.csv")

= Especificación de Periféricos y Variables IoT
A continuación se detalla la configuración de las señales y tags configurados en la nube:

#figure(
  table(
    columns: (auto, auto, 1fr, auto),
    inset: 8pt,
    align: (left, center, left, center),
    stroke: 0.5pt + rgb("#cbd5e1"),
    fill: (x, y) => if y == 0 { rgb("#e0e7ff") } else if calc.even(y) { rgb("#f8fafc") } else { none },
    ..peripherals.at(0).map(h => [*#h*]),
    ..peripherals.slice(1).map(row => (
      [#row.at(0)],
      raw(row.at(1)),
      [#row.at(2)],
      [#row.at(3)],
    )).flatten()
  ),
  caption: [Relación de periféricos, variables de telemetría y actuadores del nodo],
)
