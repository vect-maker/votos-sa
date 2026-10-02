// ==========================================
// PORTADA (PÁGINA 1)
// ==========================================
#align(center)[
  #v(1.5cm)
  #text(1.35em, weight: "bold", fill: rgb("#1e3a8a"))[TALLER LUBAN] \
  #v(0.5em)
  #text(0.95em, weight: "medium", fill: rgb("#475569"))[CURSO: APLICACIONES DE DESARROLLO CON MICROCONTROLADORES] \
  #text(0.9em, fill: rgb("#64748b"))[Turno: Vespertino]

  #v(1cm)
  #line(length: 60%, stroke: 1.5pt + rgb("#1e3a8a"))
  #v(2.5cm)

  #text(0.95em, tracking: 0.15em, weight: "bold", fill: rgb("#2563eb"))[INFORME TÉCNICO DE PROYECTO] \
  #v(0.8em)
  #text(2.0em, weight: "bold", fill: rgb("#0f172a"))[Sistema de Control y Monitoreo IoT con STM32 y NLECloud]

  #v(5cm)

  #align(center)[
    #block(width: 80%)[
      #grid(
        columns: (auto, 1fr),
        column-gutter: 1.5cm,
        row-gutter: 0.6em,
        align: (left, left),
        [*Elaborado por:*], [José Daniel Miranda Pérez],
        [*Curso:*], [Aplicaciones de Desarrollo con Microcontroladores],
        [*Turno:*], [Vespertino],
        [*Fecha:*], [#datetime.today().display("[day]/[month]/[year]")],
      )
    ]
  ]

  #v(1.5cm)
  #text(0.9em, fill: luma(100))[Managua, Nicaragua]
]
