= Arquitectura en Capas del Proyecto

El sistema está estructurado bajo una arquitectura desacoplada en múltiples capas, donde cada nivel cumple una responsabilidad especializada dentro del flujo de telemetría, control y administración de estado. En la @fig-capas se ilustran los componentes principales y los protocolos de enlace que intercomunican cada nivel.

#figure(
  image("../assets/images/project-layers.svg", width: 95%),
  caption: [Diagrama de arquitectura en capas: flujo de datos, emulación de red, pasarela de conciliación en Rust y aprovisionamiento declarativo],
) <fig-capas>

== Capa de Dispositivos (Hardware Físico y Emulador de Red)
Esta capa comprende los extremos emisores de telemetría y receptores de comandos de control. El proyecto contempla dos alternativas operativas totalmente intercambiables:

1. *Dispositivo Físico STM32 (NEWLab):*
   Implementado sobre una placa de desarrollo STM32F103 @stm32f103. Ejecuta firmware en C estructurado sobre la capa HAL de STM32Cube. Gestiona la adquisición analógica del sensor LDR vía I2C mediante el conversor PCF8591 @pcf8591, la generación de señales PWM para el servomotor horizontal ($0 degree - 180 degree$) y vertical ($0 degree - 90 degree$), la conmutación de relés de potencia de 12V DC, la pantalla gráfica LCD12864 y la interfaz serial UART hacia el módulo de conectividad WiFi M-WiFi-A.

2. *Emulador de Dispositivos en Rust (`simulator.rs`):*
   Para permitir el desarrollo, validación y pruebas de la plataforma sin dependencia obligatoria del hardware físico, el proyecto incluye un emulador nativo implementado en Rust. Este emulador opera *estrictamente a nivel de red*, abriendo un socket TCP persistente hacia el puerto 8600 del gateway de NLECloud @nlecloud e implementando la misma máquina de estados, tramas JSON y respuestas a comandos de telemando que el microcontrolador. Cuenta además con una interfaz interactiva en terminal (TUI) basada en `ratatui` con visualización gráfica del histórico de luminosidad y monitoreo de actuadores en vivo.

== Protocolo de Transporte TCP y Estructura de Payloads (api_c_doc.md)
De acuerdo con la especificación técnica de conexión TCP documentada en `Core/api_c_doc.md`, los dispositivos se comunican con el gateway de NLECloud (puertos `8600`, `8700` u `8800`) mediante tramas estructuradas en formato JSON. Esta convención de paquetes es idéntica tanto en el firmware en C del microcontrolador STM32 (`Core/Src/main.c`) como en el emulador de red en Rust (`backend/src/simulator.rs`).

#figure(
  table(
    columns: (auto, auto, 1fr, auto),
    inset: 6pt,
    align: (center, left, left, center),
    stroke: 0.5pt + rgb("#cbd5e1"),
    fill: (x, y) => if y == 0 { rgb("#e0e7ff") } else if calc.even(y) { rgb("#f8fafc") } else { none },
    [*Tipo (`t`)*], [*Identificador*], [*Descripción Técnica*], [*Dirección*],
    [`1`], [`CONN_REQ`], [Solicitud de conexión y autenticación], [Dispositivo $->$ NLECloud],
    [`2`], [`CONN_RESP`], [Respuesta de autenticación del gateway], [NLECloud $->$ Dispositivo],
    [`3`], [`PUSH_DATA`], [Publicación periódica de telemetría de sensores], [Dispositivo $->$ NLECloud],
    [`4`], [`PUSH_ACK`], [Confirmación de recepción de telemetría], [NLECloud $->$ Dispositivo],
    [`5`], [`CMD_REQ`], [Comando de telemando hacia actuadores], [NLECloud $->$ Dispositivo],
    [`6`], [`CMD_RESP`], [Respuesta de ejecución de comando del actuador], [Dispositivo $->$ NLECloud],
    [`7`], [`PING_REQ`], [Comprobación de enlace activo (Heartbeat)], [NLECloud $<->$ Dispositivo],
    [`8`], [`PING_RESP`], [Respuesta de confirmación de Heartbeat], [Dispositivo $<->$ NLECloud],
  ),
  caption: [Tipos de paquetes TCP definidos por el protocolo NLECloud],
)

=== Payloads Utilizados en el Proyecto y Justificación

- *Petición de Conexión (`t: 1`):*
Enviado inmediatamente tras abrir el socket TCP. Permite a NLECloud identificar al nodo y validar su llave criptográfica de transmisión:
```json
{
  "t": 1,
  "device": "p1476967_dev1",
  "key": "726637791de84a41bf0a8c45c16afc36",
  "ver": "v1.1"
}
```
*Por qué se utiliza:* Sin esta autenticación previa, el gateway descarta cualquier paquete entrante. Tanto la función `build_connection_request()` en `main.c` como la tarea de conexión en `simulator.rs` esperan el mensaje de respuesta `t: 2` con `"status": 0` antes de iniciar la telemetría.

- *Publicación de Telemetría (`t: 3`):*
Reporta las variables analógicas y digitales hacia la nube:
```json
{
  "t": 3,
  "datatype": 1,
  "datas": { "brightness": 142 },
  "msgid": 42
}
```
*Por qué se utiliza:* El firmware STM32 lee el conversor PCF8591 y remite el nivel de luz periódico; a su vez, cuando el emulador o el hardware operan un cambio de ángulo en `servo_x` o conmuta un relé, publican inmediatamente el nuevo valor para garantizar la sincronización del gemelo digital en la nube.

- *Recepción y Confirmación de Comandos (`t: 5` y `t: 6`):*
Cuando un usuario pulsa un botón o desliza un actuador en la interfaz web, NLECloud inyecta un comando asíncrono al socket TCP:
```json
{
  "t": 5,
  "cmdid": 1001,
  "apitag": "lamp",
  "data": 1
}
```
El dispositivo procesa la instrucción (p. ej., conmutando el pin GPIO del relé de la lámpara a nivel alto o posicionando el servomotor al ángulo especificado) y responde obligatoriamente con una trama `t: 6`:
```json
{
  "t": 6,
  "cmdid": 1001,
  "status": 0,
  "data": 1
}
```
*Por qué se utiliza:* Permite a la plataforma verificar que la orden física fue ejecutada satisfactoriamente, cerrando el ciclo de control en bucle cerrado.

- *Latido de Enlace (Heartbeat):*
A diferencia de los paquetes JSON anteriores, el protocolo establece cadenas ASCII delimitadas para el mantenimiento del socket:
- Solicitud de verificación: `"$#AT#\r"`
- Respuesta de confirmación: `"$OK##\r"`
*Por qué se utiliza:* Los cortafuegos y el gateway de NLECloud cierran las conexiones inactivas tras 50 segundos. El bucle del STM32 y el temporizador en `simulator.rs` transmiten periódicamente esta trama para sostener el socket permanentemente abierto.

== Capa Cloud: Plataforma NLECloud
La plataforma en la nube NLECloud @nlecloud actúa como el *backend primario y broker central de datos IoT* del ecosistema:
- *Gateway TCP (Puerto 8600):* Canal de comunicación en tiempo real para dispositivos embebidos y emuladores, responsable de recibir tramas periódicas de telemetría y despachar instrucciones de telemando hacia los nodos en línea.
- *API REST:* Servicio web para la gestión de proyectos, cuentas de usuario, catálogo de sensores, actuadores y consulta de datos históricos.

== Capa de Agregación y Pasarela (Backend Relay en Rust)
A diferencia de las arquitecturas web tradicionales, el backend propio implementado en Rust (utilizando el runtime asíncrono Tokio y el framework web Axum) *no dispone de una base de datos persistente*. En su lugar, opera como un *relay / agregador especializado* con las siguientes directrices:
- *Administración de Estado en Memoria:* Centraliza un snapshot reactivo de los dispositivos (`DeviceSnapshot`) protegido por bloqueos de lectura/escritura concurrentes (`Arc<RwLock<...>>`).
- *Conciliación de Estado (`state.rs`):* Administra y armoniza en tiempo real las diferencias de estado entre los comandos recibidos desde la web y el estado efectivo reportado por NLECloud mediante comparaciones de equivalencia laxa (`values_are_equivalent`). Incorpora un mecanismo de tiempo de gracia (_settling TTL_) en comandos pendientes para evitar que el sondeo periódico sobreescriba una acción recién solicitada antes de su asentamiento en la nube.
- *Consumo de NLECloud mediante SDK Abierto Propio:* Para interactuar con la API REST de la nube, el backend se apoya en `nle-cloud-rust-sdk` @nle-cloud-rust-sdk, un SDK fuertemente tipado desarrollado y publicado como crate abierto en Cargo por el autor del proyecto:

```toml
# Dependencia en backend/Cargo.toml
nle-cloud-sdk = { git = "https://github.com/vect-maker/nle-cloud-rust-sdk" }
```

=== Métodos del SDK Utilizados y Justificación Técnica
A lo largo de los módulos `project.rs`, `seed.rs`, `state.rs` y `server.rs`, el backend invoca de forma exhaustiva las capacidades de `nle-cloud-rust-sdk`:

1. `client.login_with_credentials(&account, &password, false)`:
   - *Uso:* Autentica la cuenta de usuario contra NLECloud al iniciar el servicio.
   - *Por qué:* Obtiene el token de portador JWT (_Bearer Token_) indispensable para autorizar todas las llamadas posteriores hacia la API REST.

2. `client.with_token(token)`:
   - *Uso:* Instancia y clona el cliente HTTP inyectándole el token autenticado.
   - *Por qué:* Permite reutilizar el cliente de forma segura entre múltiples hilos y tareas asíncronas de Tokio sin reautenticarse constantemente.

3. `client.get_projects(&query, None)` y `client.add_project(&dto, None)`:
   - *Uso:* En la fase de aprovisionamiento declarativo (`project.rs`).
   - *Por qué:* Verifica si el proyecto IoT objetivo ya existe en NLECloud; si no existe, lo crea automáticamente de forma idempotente con su nombre configurado.

4. `client.delete_projects(&ids, None)`:
   - *Uso:* Comando de mantenimiento de proyecto (`just delete-project`).
   - *Por qué:* Permite reiniciar completamente el entorno de desarrollo y eliminar artefactos obsoletos con un solo comando.

5. `client.get_devices(&dev_query, None)`, `client.add_device(&dto, None)` y `client.delete_device(device_id, None)`:
   - *Uso:* Sincronización del catálogo de nodos en `seed.rs`.
   - *Por qué:* Lee la lista de dispositivos registrados en la nube, elimina aquellos no deseados y crea los dispositivos declarados en `seed/devices.json` vinculándolos al protocolo de transporte `TCP`.

6. `client.get_device_info(device_id, None)`:
   - *Uso:* Inspección detallada de periféricos de un dispositivo existente.
   - *Por qué:* Recupera el conjunto actual de sensores y actuadores asociados al nodo para comparar contra la especificación declarativa.

7. `client.add_sensor(device_id, &sensor_dto, None)` y `client.delete_sensor(device_id, api_tag, None)`:
   - *Uso:* Provisión atómica de sensores y actuadores.
   - *Por qué:* Registra los tags de telemetría (`brightness`, `servo_x`, `lamp`, etc.) con sus unidades, rangos y tipos de operación (Switch, Scale, Button) definidos en `seed/peripherals.json`.

8. `client.get_devices_status(&dev_ids_str, None)`:
   - *Uso:* Sondeo continuo del estado de conectividad en `state.rs`.
   - *Por qué:* Determina instantáneamente si el hardware físico o el emulador se encuentran conectados (`is_online`) mediante una consulta de bajo costo.

9. `client.get_project_sensors_realtime(project_id, Some(device_id), None)`:
   - *Uso:* Adquisición periódica en lote de los valores de telemetría y actuadores.
   - *Por qué:* Permite refrescar en una sola petición HTTP todos los valores vigentes de los sensores del proyecto, alimentando el snapshot en memoria sin saturar la red con consultas individuales.

10. `client.send_cmd(device_id, api_tag, &value, None)`:
    - *Uso:* Inyección de comandos de telemando en `state.rs::control_actuator()`.
    - *Por qué:* Cuando un usuario interactúa con la interfaz web (p. ej., encendiendo la lámpara o moviendo un servo), esta llamada solicita a NLECloud que construya y envíe la trama TCP `t: 5` directamente al socket del dispositivo en tiempo real.

== Capa de Configuración Declarativa y Aprovisionador Idempotente
Para evitar la configuración manual y propensa a errores en paneles web, el proyecto implementa un formato declarativo en archivos JSON que funciona como la *única fuente de verdad* (_Source of Truth_) del sistema:

- *`seed/peripherals.json`:* Define plantillas reutilizables de sensores y actuadores con sus atributos técnicos:
```json
{
  "smart_home": {
    "sensors": [
      {
        "name": "brightness",
        "api_tag": "brightness",
        "data_type": "Float",
        "trans_type": "ReportOnly",
        "unit": "flux"
      }
    ],
    "actuators": [
      {
        "name": "servo_x",
        "api_tag": "servo_x",
        "data_type": "Float",
        "oper_type": "Scale",
        "min": 0.0,
        "max": 180.0
      },
      {
        "name": "lamp",
        "api_tag": "lamp",
        "data_type": "Boolean",
        "oper_type": "Switch"
      }
    ]
  }
}
```

- *`seed/devices.json`:* Define la lista de nodos y el conjunto de periféricos asignado a cada uno:
```json
[
  {
    "name": "dev1",
    "tag": "dev1",
    "protocol": "TCP",
    "peripheral_set": "smart_home"
  }
]
```

- *Motor de Aprovisionamiento (`seed.rs` / `project.rs`):*
El aprovisionador resuelve automáticamente la asociación de periféricos cruzando las definiciones declarativas y sincroniza el proyecto en NLECloud de manera idempotente:

```rust
// Resolución declarativa en backend/src/seed.rs
pub fn load_resolved_device_seeds(
    devices_path: &Path,
    peripherals_path: &Path,
) -> Result<Vec<DeviceSeed>> {
    let raw_devices: Vec<RawDeviceSeed> = serde_json::from_str(&fs::read_to_string(devices_path)?)?;
    let sets: HashMap<String, PeripheralSetSeed> = serde_json::from_str(&fs::read_to_string(peripherals_path)?)?;
    // Resuelve y asocia cada sensor/actuador al dispositivo...
    Ok(resolved_devices)
}
```

El usuario o el proceso de CI/CD puede ejecutar `just provision`, el cual crea o actualiza de forma desatendida el proyecto, dispositivos, sensores y actuadores en NLECloud.

== Capa de Presentación (Frontend Web)
Desarrollada como una Single Page Application (SPA) con Vue 3, Vite, Tailwind CSS y componentes DaisyUI. Se comunica mediante HTTP y Server-Sent Events (SSE) con el backend de Rust para brindar monitoreo y control telemático en tiempo real con latencia mínima.

== Flujos de Procesos y Ciclos de Vida (Dispositivo, Backend y Frontend)

Para comprender el funcionamiento dinámico del sistema más allá de su estructura estática, en la @fig-flujos se modelan los dos bucles principales de intercambio de información: el ciclo periódico de telemetría y el bucle de telemando interactivo con conciliación de estado.

#figure(
  image("../assets/images/process-flows.svg", width: 95%),
  caption: [Diagrama de secuencia y flujos de procesos: telemetría reactiva periódica y bucle cerrado de telemando con tiempo de gracia (settling TTL)],
) <fig-flujos>

=== 1. Flujo de Ejecución en el Dispositivo (Firmware STM32 y Emulador)
Tanto el microcontrolador físico STM32F103 (`Core/Src/main.c`) como el emulador de red en Rust (`backend/src/simulator.rs`) operan bajo un ciclo de vida estructurado en cuatro fases concurrentes:

1. *Inicialización de Hardware y Periféricos:*
   Al encender el microcontrolador, se configuran los osciladores del sistema (72 MHz), los puertos GPIO, la interfaz I2C para el ADC PCF8591, los temporizadores TIM3 para modulación PWM de los servomotores y las comunicaciones UART (UART1 para depuración y UART4 para el módulo WiFi). Los servomotores se calibran y posicionan de forma suave en su coordenada de reposo ($90 degree, 90 degree$).
2. *Establecimiento de Red y Handshake TCP:*
   El firmware envía comandos AT al módulo M-WiFi-A (`AT+CWJAP_CUR` para asociarse a la red WiFi y `AT+CIPSTART` para abrir el socket TCP hacia `SERVER_IP:8600`). Tras conectar el socket, transmite la trama `t: 1` (`CONN_REQ`) con la credencial secreta del dispositivo y aguarda la confirmación `t: 2` (`CONN_RESP`, `status: 0`).
3. *Bucle Periódico de Telemetría y Latido:*
   En el bucle principal (`while(1)`), el firmware muestrea periódicamente el sensor LDR a través de I2C (`PCF8591_ReadLDR()`), calcula el promedio de muestras y despacha el paquete `t: 3` (`PUSH_DATA`). Paralelamente, evalúa temporizadores para transmitir el latido de enlace (`$#AT#\r`) cada 40 segundos, garantizando que el socket permanezca abierto ante cortafuegos intermedios.
4. *Interrupción de Comandos y Bucle de Telemando:*
   Cuando llega una trama por UART desde la nube con `t: 5` (`CMD_REQ`), el parser extrae el `cmdid` y el tag objetivo:
   - Si el tag corresponde a un relé (`lamp`, `fan`, `lock`), conmuta el estado lógico del pin GPIO respectivo y actualiza el LCD local.
   - Si el tag es `servo_x` o `servo_y`, ajusta el ancho de pulso PWM entre 500 µs y 2500 µs (0° a 180°).
   - Si el tag es `calibrate_light`, desencadena la rutina de calibración lumínica automática: ejecuta un barrido angular bidireccional mediante un algoritmo de búsqueda por gradiente local (`Servo_LocalSearch()`), muestreando la luz en cada orientación hasta bloquear los servos en el ángulo de máxima luminosidad.
   - Al completar la acción, responde de inmediato con `t: 6` (`CMD_RESP`) para confirmar la ejecución.

=== 2. Flujo de Procesos en el Backend en Rust
El backend relay orquesta el flujo de información entre NLECloud y los clientes web sin intermediación de bases de datos tradicionales:

1. *Flujo de Aprovisionamiento Desatendido (`just provision`):*
   Lee los esquemas declarativos `seed/devices.json` y `seed/peripherals.json`. Utilizando `nle-cloud-rust-sdk`, se autentica en la nube, valida si el proyecto existe (creándolo si es necesario con `client.add_project()`), compara los dispositivos y sensores existentes, y aplica las altas o bajas necesarias de forma idempotente.
2. *Flujo de Sondeo Asíncrono y Conciliación (`poll_all` / `state.rs`):*
   Una tarea asíncrona dedicada en Tokio consulta periódicamente la API REST de NLECloud:
   - Primero consulta `client.get_devices_status()` para determinar si los nodos están en línea.
   - Si están conectados, invoca `client.get_project_sensors_realtime()` para adquirir en un solo lote todas las lecturas actuales.
   - Ejecuta la conciliación en memoria: si un comando fue enviado recientemente, el backend activa un tiempo de gracia (_settling TTL_) que ignora lecturas transitorias de la nube hasta que el valor se asiente, evitando que la interfaz web "parpadee" o revierta el valor.
   - Si se detecta un cambio efectivo de estado, el backend serializa el nuevo `DeviceSnapshot` y lo publica al canal broadcast de Tokio.
3. *Flujo de Despacho de Comandos (`POST /api/devices/control`):*
   Al recibir una solicitud HTTP desde el frontend, el backend verifica que el nodo esté en línea (devolviendo error 409 si está desconectado). Registra el valor pendiente en memoria, emite la orden a NLECloud mediante `client.send_cmd()` y difunde inmediatamente la actualización por SSE, logrando retroalimentación visual instantánea.

=== 3. Flujo de Procesos en el Frontend Web (Vue 3)
La interfaz de usuario interactúa de forma continua y desacoplada con el backend:

1. *Inicialización y Suscripción Reactiva:*
   Al cargar la aplicación, se establece una conexión unidireccional persistente mediante Server-Sent Events (`/api/sse`). El frontend recibe un evento inicial `init` con la fotografía completa del sistema y renderiza dinámicamente las tarjetas de dispositivos.
2. *Consumo de Eventos en Tiempo Real:*
   Cada vez que el backend emite un evento `update`, la tienda reactiva de Pinia en Vue 3 actualiza el estado local sin recargar la página. Los gráficos de Chart.js anexan los nuevos puntos de luz, los medidores angulares rotan a la nueva posición y los indicadores visuales reflejan el estado del relé.
3. *Emisión de Acciones del Usuario:*
   Cuando el operador mueve un deslizador angular o conmuta un interruptor, el componente despacha una petición asíncrona hacia `/api/devices/control`. Si el dispositivo está apagado, se presenta un toast de notificación bloqueando el control; si está en línea, la interfaz asume optimísticamente el valor mientras el backend y el firmware consolidan la orden física.
