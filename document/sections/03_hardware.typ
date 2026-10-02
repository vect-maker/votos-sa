#let pin-table(data, caption-text) = figure(
  table(
    columns: (1fr, 1fr),
    inset: 5pt,
    align: center,
    stroke: 0.5pt + rgb("#cbd5e1"),
    fill: (x, y) => if y == 0 { rgb("#e0e7ff") } else if calc.even(y) { rgb("#f8fafc") } else { none },
    ..data.at(0).map(h => [*#h*]),
    ..data.slice(1).map(row => (
      raw(row.at(0)),
      raw(row.at(1)),
    )).flatten()
  ),
  caption: caption-text,
)

#let exp-data = csv("../assets/pin_mapping/expansion_module.csv")
#let led1-data = csv("../assets/pin_mapping/led_1.csv")
#let led2-data = csv("../assets/pin_mapping/led_2.csv")
#let led3-data = csv("../assets/pin_mapping/led_3.csv")

= Hardware y Conexionado Físico
== Módulos e Interconexión
En la @fig-diagrama se presenta el diagrama general de interconexión física entre los diferentes módulos del banco de trabajo NEWLab y la placa de desarrollo STM32:

#figure(
  image("../assets/images/project-diagram.webp", width: 85%),
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

== Montaje Físico del Prototipo en Hardware
En la @fig-foto-dispositivo y la @fig-foto-pantalla se documenta el montaje físico real implementado en el laboratorio sobre el banco de desarrollo NEWLab, así como la visualización local en la pantalla LCD:

#grid(
  columns: (3fr, 2fr),
  gutter: 14pt,
  [
    #figure(
      image("../assets/images/device_photo.jpeg", height: 8cm, fit: "contain"),
      caption: [Montaje del prototipo en banco NEWLab con placa STM32, módulo WiFi, relés y gimbal pan-tilt],
    ) <fig-foto-dispositivo>
  ],
  [
    #figure(
      image("../assets/images/device_photo_screen.jpg", height: 8cm, fit: "contain"),
      caption: [Detalle de visualización local en la pantalla gráfica LCD12864],
    ) <fig-foto-pantalla>
  ],
)

== Tablas de Mapeo de Pines
Para facilitar la verificación del conexionado mostrado en el diagrama físico, las siguientes tablas detallan el mapeo exacto de pines entre la placa central M3, el módulo de expansión y el módulo de visualización LCD:

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  pin-table(led1-data, [Módulo Pantalla a Placa M3 (Bus 1)]),
  pin-table(led2-data, [Módulo Pantalla a Placa M3 (Bus 2)]),
)

#v(0.6em)

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  pin-table(exp-data, [Módulo de Expansión a Placa M3]),
  pin-table(led3-data, [Módulo de Expansión a Pantalla]),
)
