#set page(
  paper: "a4",
  margin: (x: 2.5cm, top: 2.5cm, bottom: 2.5cm),
  header: context {
    if counter(page).get().first() > 1 [
      #align(right)[#text(8pt, fill: luma(120))[Taller Luban Nicaragua • INATEC | Informe Técnico]]
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
  font: ("Liberation Sans", "Noto Sans"),
  size: 10.5pt,
  lang: "es",
  spacing: 120%,
)

#set par(justify: true, leading: 0.7em)
#set heading(numbering: "1.1")

// ==========================================
// PORTADA (PÁGINA 1)
// ==========================================
#align(center)[
  #v(1.5cm)
  #text(1.25em, weight: "bold", fill: rgb("#1e3a8a"))[CENTRO TECNOLÓGICO TALLER LUBAN NICARAGUA] \
  #v(0.2em)
  #text(1.05em, weight: "bold", fill: rgb("#1e3a8a"))[INSTITUTO NACIONAL TECNOLÓGICO - INATEC] \
  #v(0.5em)
  #text(0.95em, weight: "medium", fill: rgb("#475569"))[CURSO: APLICACIONES DE DESARROLLO CON MICROCONTROLADORES] \
  #text(0.9em, fill: rgb("#64748b"))[Turno: Vespertino]

  #v(1cm)
  #line(length: 60%, stroke: 1.5pt + rgb("#1e3a8a"))
  #v(2.5cm)

  #text(0.95em, tracking: 0.15em, weight: "bold", fill: rgb("#2563eb"))[INFORME TÉCNICO DE PROYECTO] \
  #v(0.8em)
  #text(2.0em, weight: "bold", fill: rgb("#0f172a"))[Sistema de Control y Monitoreo IoT con STM32 y NLECloud] \
  #v(0.6em)
  #text(1.05em, fill: rgb("#475569"))[Adquisición I2C, Control PWM de Servos, Conciliación de Estado y Plataforma Web]

  #v(4.5cm)

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

// ==========================================
// SALTO A PÁGINA 2
// ==========================================
#pagebreak()

// --- Resumen Ejecutivo ---
#block(
  fill: rgb("#f8fafc"),
  inset: 12pt,
  radius: 6pt,
  stroke: 1pt + rgb("#e2e8f0"),
)[
  #text(weight: "bold", fill: rgb("#1e293b"))[Resumen del Proyecto:] \
  Este informe documenta el desarrollo de un sistema embebido IoT basado en el microcontrolador STM32, integrando adquisición de señales analógicas (sensor de luz LDR vía conversor PCF8591 por I2C), control de servomotores PWM bidireccionales con algoritmo de búsqueda de máxima luminosidad, y actuadores digitales de control domiciliario. El dispositivo se comunica mediante comandos AT hacia el gateway TCP de NLECloud, respaldado por un backend en Rust y una interfaz gráfica interactiva en Vue 3.
]

#v(1.2em)

= Introducción
El desarrollo de soluciones embebidas conectadas representa uno de los pilares fundamentales del Internet de las Cosas (IoT). En este proyecto se integran conceptos de programación de microcontroladores en C (HAL/STM32Cube), comunicación serial con módulos de conectividad WiFi, protocolos de transporte en red y diseño de interfaces de usuario para la supervisión y telemando en tiempo real.

= Objetivos
== Objetivo General
Diseñar e implementar un sistema embebido de control y monitoreo en tiempo real basado en microcontrolador STM32 conectado a una plataforma IoT en la nube.

== Objetivos Específicos
- Configurar periféricos de hardware en STM32: comunicación I2C (conversor ADC/DAC PCF8591), salidas PWM para servomotores horizontal y vertical, y puertos GPIO para control de actuadores.
- Implementar la pila de comunicación TCP hacia NLECloud mediante comandos AT y tramas JSON de telemetría y telemando.
- Desarrollar la lógica de calibración automática de seguimiento solar/lumínico mediante barrido angular y muestreo de intensidad de luz.
- Implementar y validar la arquitectura de software (backend de conciliación y frontend web) para visualización y control interactivo.

= Arquitectura del Sistema
== Hardware y Periféricos Embebidos
En la @fig-diagrama se presenta el diagrama general de interconexión física entre los diferentes módulos del banco de trabajo NEWLab y la placa de desarrollo STM32:

#figure(
  image("images/project-diagram.webp", width: 85%),
  caption: [Diagrama de conexionado e interconexión física del sistema (Módulos NEWLab, STM32, Sensores, Relés y Servomotores)],
) <fig-diagrama>

El conexionado comprende los siguientes bloques funcionales:
- *Placa de Control Principal (M3 Core Board):* Módulo STM32F103 responsable de la adquisición de señales, control de actuadores y gestión de comunicaciones.
- *Módulo de Comunicaciones WiFi (M-WiFi-A):* Interfaz UART conectada a la placa central para el intercambio de tramas con NLECloud.
- *Módulo de Expansión de Periféricos (M-Peripheral):* Integra el convertidor analógico-digital I2C (PCF8591) para la lectura del sensor de luz.
- *Módulo Sensor de Luz (SENSOR-RESIS-A):* Circuito con fotorresistencia (LDR) conectado a las entradas del conversor I2C.
- *Módulo Gimbal Pan-Tilt (Servomotores):* Dos servomotores PWM alimentados a +6V DC para orientar el sensor lumínico en los ejes horizontal y vertical.
- *Módulos de Relés (M-RELAY):* Conmutación de cargas de potencia de 12V DC (ventilador, cerradura eléctrica y lámpara de iluminación).
- *Alimentación Centralizada NEWLab:* Rieles de distribución de tensión (3.3V, 5V y 12V) con referencia de tierra compartida (GND).

== Firmware y Protocolo de Comunicación
// Detallar ciclo de vida del firmware, recepción por UART, parser de tramas TCP NLECloud (tipos 1, 2, 3, 5 y 6).

== Plataforma Web y Backend de Sincronización
// Explicar la capa Rust (conciliación de estado y pasarela) y frontend en Vue 3 con DaisyUI.

= Especificación de Periféricos y Variables IoT
A continuación se detalla la configuración de las señales y tags configurados en la nube:

#figure(
  table(
    columns: (auto, auto, 1fr, auto),
    inset: 8pt,
    align: (left, center, left, center),
    stroke: 0.5pt + rgb("#cbd5e1"),
    fill: (x, y) => if y == 0 { rgb("#e0e7ff") } else if calc.even(y) { rgb("#f8fafc") } else { none },
    [*Nombre*], [*Tag API*], [*Descripción / Periférico*], [*Tipo*],
    [Luminosidad], [`brightness`], [Sensor LDR analógico vía PCF8591 (I2C)], [Sensor (0 - 255)],
    [Servo X], [`servo_x`], [Servomotor horizontal (0° a 180°)], [Actuador rango],
    [Servo Y], [`servo_y`], [Servomotor vertical (0° a 90°)], [Actuador rango],
    [Lámpara], [`lamp`], [Iluminación principal], [Actuador switch],
    [Ventilador], [`fan`], [Sistema de ventilación], [Actuador switch],
    [Cerradura], [`lock`], [Cerradura electrónica], [Actuador switch],
    [Posición Home], [`servo_home`], [Centrado de servos a posición base (90°, 90°)], [Actuador botón],
    [Calibrar Luz], [`calibrate_light`], [Barrido y rastreo del punto de mayor luz], [Actuador botón],
  ),
  caption: [Relación de periféricos, variables de telemetría y actuadores del nodo],
)

= Pruebas y Resultados
// Sección para incluir capturas del frontend, fotos del prototipo, mediciones y logs de telemetría.
// Para insertar imágenes almacenadas en document/images/:
// #figure(
//   image("images/prototipo_hardware.png", width: 80%),
//   caption: [Montaje físico del prototipo STM32],
// )

= Conclusiones
// Conclusiones del aprendizaje práctico, retos superados y aplicaciones potenciales.

= Bibliografía y Referencias
// Enlaces a datasheets (STM32, PCF8591), documentación NLECloud y librerías utilizadas.
