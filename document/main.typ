#set page(
  paper: "a4",
  margin: (x: 2.5cm, top: 2.5cm, bottom: 2.5cm),
  header: context {
    if counter(page).get().first() > 1 [
      #align(right)[#text(8pt, fill: luma(120))[Taller Luban | Informe Técnico]]
      #v(-0.3em)
      #line(length: 100%, stroke: 0.3pt + luma(210))
    ]
  },
  footer: context {
    if counter(page).get().first() > 1 [
      #line(length: 100%, stroke: 0.3pt + luma(210))
      #v(-0.3em)
      #grid(
        columns: (1fr, 1fr),
        align(left)[#text(8pt, fill: luma(120))[Aplicaciones de Desarrollo con Microcontroladores]],
        align(right)[#text(8pt, fill: luma(100))[Página #counter(page).display("1 de 1", both: true)]]
      )
    ]
  }
)

#set text(
  font: ("Liberation Sans", "Noto Sans", "Noto Sans CJK SC"),
  size: 10.5pt,
  lang: "es",
  spacing: 120%,
)

#set par(justify: true, leading: 0.7em)
#set heading(numbering: "1.1")
#show bibliography: set heading(numbering: "1.1")

// ==========================================
// ORQUESTADOR DEL DOCUMENTO
// ==========================================
#include "sections/00_portada.typ"
#pagebreak()

#include "sections/01_descripcion.typ"
#include "sections/02_capas.typ"
#include "sections/03_hardware.typ"
#include "sections/04_perifericos.typ"
#include "sections/05_pruebas.typ"
#include "sections/06_conclusiones.typ"
