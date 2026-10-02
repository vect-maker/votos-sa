= Pruebas y Resultados

Para validar la interoperabilidad integral del ecosistema, se realizaron ensayos funcionales que comprenden la simulación de red en consola con Ratatui, la conciliación reactiva en el backend y la supervisión y telemando desde la aplicación web interactiva.

== Emulación en Capa de Red y Simulación de Telemetría (TUI)
Para verificar la robustez del sistema y el comportamiento de la pasarela sin requerir hardware físico, se ejecutó el emulador de dispositivos (`just simulate`). El emulador establece una conexión socket TCP hacia el puerto `8600` del gateway de NLECloud, respondiendo a las órdenes de telemando emitidas desde la interfaz web y reportando fluctuaciones de intensidad lumínica en tiempo real.

En la @fig-simulador se aprecia la interfaz de terminal (TUI) desarrollada en Rust con `ratatui` y `tui-logger`. La interfaz despliega en tiempo real:
- La gráfica de evolución histórica de luminosidad (curva en tiempo real del sensor LDR simulado).
- El estado y posición angular de los servomotores horizontal (`servo_x`) y vertical (`servo_y`).
- El estado de los actuadores de potencia (lámpara, ventilador y cerradura).
- Los registros de eventos (_logs_) de red, autenticación TCP (`t: 1` y `t: 2`), reportes de telemetría (`t: 3`) y comandos despachados (`t: 5`).

#figure(
  image("../assets/images/device_simulator.png", width: 90%),
  caption: [Interfaz de usuario en terminal (TUI) del emulador de dispositivos en Rust, mostrando telemetría en tiempo real y registro de tramas TCP],
) <fig-simulador>

== Verificación de la Plataforma Web y Conciliación de Estado
La plataforma web interactiva desarrollada en Vue 3 y DaisyUI permite monitorear y gobernar los periféricos en tiempo real a través del backend relay en Rust.

=== Panel Principal de Dispositivos (Home)
Al acceder a la plataforma, se presenta el listado de nodos registrados en el proyecto de NLECloud, reflejando su estado de conectividad en tiempo real (@fig-web-home).

#figure(
  image("../assets/images/web_platform_home.png", width: 90%),
  caption: [Vista principal del panel web de administración con el catálogo de dispositivos provistos],
) <fig-web-home>

=== Control Activo y Telemetría en Tiempo Real (Dispositivo Conectado)
Cuando el dispositivo se encuentra en línea (ya sea el hardware físico STM32 o el emulador de red), el dashboard habilita la vista completa de telemetría y telemando (@fig-web-device-on):
- *Monitor de Luminosidad:* Gráfico dinámico en tiempo real que traza las mediciones del sensor LDR con indicación de valor actual (0 - 255) y nivel de escala.
- *Control de Servomotores Pan-Tilt:* Deslizadores angulares interactivos para `servo_x` ($0 degree - 180 degree$) y `servo_y` ($0 degree - 90 degree$), junto con botones de acción directa como *Centrar Servos* (`servo_home` a 90°/90°) y *Calibrar Luz* (`calibrate_light`), ejecutando la búsqueda automática del punto de máxima iluminación.
- *Conmutadores de Actuadores (Relés de 12V):* Switches reactivos para la lámpara, el ventilador y la cerradura eléctrica. Al conmutar un interruptor, el backend registra el comando pendiente con TTL de asentamiento y lo propaga a NLECloud sin inconsistencias de estado.

#figure(
  image("../assets/images/web_platform_device_view.png", width: 85%),
  caption: [Vista interactiva del dispositivo en línea, mostrando controles angulares de servomotores, conmutadores de relés y gráfica de telemetría en tiempo real],
) <fig-web-device-on>

=== Detección de Estado Fuera de Línea (Dispositivo Desconectado)
Si el microcontrolador o el simulador pierden la conexión TCP o se apagan, el backend detecta el cambio mediante `client.get_devices_status()` y actualiza el snapshot en memoria. La interfaz web refleja de inmediato el estado inactivo (@fig-web-device-off), deshabilitando los controles interactivos y alertando al usuario de que el nodo se encuentra fuera de línea para evitar órdenes fallidas.

#figure(
  image("../assets/images/web_platform_device_off_view.png", width: 85%),
  caption: [Vista del dispositivo fuera de línea, alertando la desconexión del nodo y restringiendo el envío de comandos de telemando],
) <fig-web-device-off>
