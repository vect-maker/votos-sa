/* USER CODE BEGIN Header */
/**
  ******************************************************************************
  * @file           : main.c
  * @brief          : NLE CLOUD - LAMPARA (PA1) + VENTILADOR (PA0) + LDR (PCF8591)
  * @version        : 28.0 FINAL - LIGERO (MODO 100% MANUAL)
  ******************************************************************************
  */
/* USER CODE END Header */

/* Inclusi?n de librer?as necesarias */
#include "main.h"       // Definiciones principales del microcontrolador
#include <string.h>     // Para manipular cadenas de texto (strcpy, strstr, etc.)
#include <stdio.h>      // Para funciones de entrada/salida formateada (sprintf)
#include <stdlib.h>     // Librer?a est?ndar (conversiones, memoria)
#include "lcd12864.h"   // Librer?a personalizada para controlar la pantalla LCD 128x64a

/* Declaraci?n de manejadores (handlers) para perif?ricos de hardware */
RTC_HandleTypeDef hrtc;   // Manejador para el Reloj de Tiempo Real (RTC)
UART_HandleTypeDef huart4;// Manejador para el puerto serie UART4 (Comunicaci?n con ESP8266)
UART_HandleTypeDef huart1;// Manejador para el puerto serie USART1 (Nuevo - STM32CubeMX)
TIM_HandleTypeDef htim3;  // Manejador para el temporizador TIM3 (Nuevo - STM32CubeMX)

/* USER CODE BEGIN PV */
/* === CONSTANTES DE CONFIGURACI?N === */
#define MAX_AT_RX_LEN       512               // Tama?o m?ximo del buffer de recepci?n UART
#define WIFI_SSID           "IoT-Room10H"     // Nombre de la red WiFi
#define WIFI_PASSWORD       "IoT-Room10H"     // Contrase?a del WiFi
#define SERVER_IP           "121.37.241.174"  // Direcci?n IP del servidor NLE Cloud
#define SERVER_PORT         8600              // Puerto del servidor
#define DEVICE_TAG          "smarthomejdmp"  // Identificador del dispositivo en la nube
#define SECRET_KEY          "726637791de84a41bf0a8c45c16afc36" // Clave secreta para autenticaci?n
#define LAMP_TAG            "nl_lampState"         // Etiqueta del rel? de la l?mpara en la nube
#define FAN_TAG             "nl_fanState"          // Etiqueta del rel? del ventilador en la nube
#define LDR_TAG             "brightness"      // Etiqueta del sensor de luz en la nube
#define LOCK_TAG            "nl_lockState"
#define KEY2_LONG_THRESHOLD_MS  800   // >= 800 ms = Lock, < 800 ms = Fan
/* === SERVO CONFIG === */
#define SERVO_MIN_PULSE     500
#define SERVO_MAX_PULSE     2500
#define SERVO1_MIN_ANGLE        0
#define SERVO1_MAX_ANGLE        180
#define SERVO1_PHYSICAL_MAX     180

#define SERVO2_MIN_ANGLE        0
#define SERVO2_MAX_ANGLE        90
#define SERVO2_PHYSICAL_MAX     180

/* === BEST LIGHT TRACKING === */
#define BEST_LIGHT_DECAY_AMOUNT   5     // Cu?nto restar al best_light cada ciclo
#define BEST_LIGHT_DECAY_MS       10000 // Cada cu?ntos ms se aplica el decay (10s)
#define LOCAL_SEARCH_STEP         10    // Grados que se mueve en cada direcci?n
#define LOCAL_SEARCH_INTERVAL_MS  5000 // Cada cu?ntos ms corre la b?squeda local (20s)

#define SERVO1_TAG          "nl_servo1"
#define SERVO2_TAG          "nl_servo2"
#define SERVO1_STATE_TAG    "nl_servoState1"
#define SERVO2_STATE_TAG    "nl_servoState2"


/* === PINES I2C POR SOFTWARE (Para leer el chip PCF8591 que tiene el LDR) === */
#define PCF8591_ADDR_WRITE  0x90      // Direcci?n I2C para escribir en el chip
#define PCF8591_ADDR_READ   0x91      // Direcci?n I2C para leer del chip
#define PCF8591_CHANNEL_0   0x00      // Canal 0 (donde est? conectado el LDR)
#define I2C_SCL_PIN         GPIO_PIN_4 // Pin de reloj I2C (PA4)
#define I2C_SDA_PIN         GPIO_PIN_5 // Pin de datos I2C (PA5)
#define I2C_PORT            GPIOA      // Puerto A

/* === SERVO LIGHT CALIBRATION CONFIG === */
#define CALIB_ANGLE_STEP    10   // Salto en grados (1 = todos los grados, 10 = cada 10?, etc.)
#define CALIB_SETTLE_MS     250  // ms de espera para que el servo se mueva y estabilice
#define CALIB_LDR_SAMPLES   3    // Muestras del LDR para promediar (reduce ruido)

/* === VARIABLES GLOBALES === */
volatile char     AT_RX_BUF[MAX_AT_RX_LEN]; // Buffer para guardar lo que recibe el ESP8266
volatile uint16_t AT_RX_COUNT = 0;          // Contador de caracteres recibidos
uint8_t  rx_byte            = 0;            // Variable temporal para guardar el ?ltimo byte recibido
uint8_t  cloud_connected    = 0;            // Bandera: 1 = conectado a la nube, 0 = desconectado
uint32_t last_heartbeat     = 0;            // Marca de tiempo del ?ltimo 'ping' enviado al servidor
uint32_t last_sensor_report = 0;            // Marca de tiempo del ?ltimo reporte de sensores
uint32_t last_ldr_read      = 0;            // Marca de tiempo de la ?ltima lectura del sensor de luz
uint8_t  lampState          = 0;            // Estado actual de la l?mpara (1=ON, 0=OFF)
uint8_t  fanState           = 0;            // Estado actual del ventilador (1=ON, 0=OFF)
uint8_t lockState = 0;
uint8_t  valorLDR           = 0;            // ?ltimo valor le?do del sensor de luz (0-255)
uint8_t  valorLDRAnterior   = 0;            // Valor anterior del sensor (para detectar cambios)
uint32_t last_lcd_update    = 0;            // Marca de tiempo de la ?ltima actualizaci?n de pantalla
uint16_t servo1_angle = 90;
uint16_t servo2_angle = 90;
char     IpData[256];                       // Buffer para extraer datos ?tiles de los mensajes de red

/* === BEST LIGHT TRACKING GLOBALES === */
uint8_t  g_best_light      = 0;   // Mejor lectura de luz encontrada hasta ahora
uint16_t g_best_s1         = 90;  // ?ngulo del servo 1 donde se encontr? la mejor luz
uint16_t g_best_s2         = 90;  // ?ngulo del servo 2 donde se encontr? la mejor luz
uint32_t last_best_decay   = 0;   // Timestamp del ?ltimo decay
uint32_t last_local_search = 0;   // Timestamp de la ?ltima b?squeda local
/* USER CODE END PV */

/* Prototipos de funciones de inicializaci?n generadas por STM32CubeMX */
void SystemClock_Config(void);
static void MX_GPIO_Init(void);
static void MX_RTC_Init(void);
static void MX_UART4_Init(void);
static void MX_USART1_UART_Init(void);  // Nuevo - STM32CubeMX
static void MX_TIM3_Init(void);          // Nuevo - STM32CubeMX

/* USER CODE BEGIN 0 */

// Helpers para construir payloads
int build_connection_request(char* out_buf, size_t buf_size,
                              const char* device_tag,
                              const char* secret_key) {
    if (out_buf == NULL || buf_size == 0) return -1;

    int n = snprintf(out_buf, buf_size,
        "{\"t\":1,\"device\":\"%s\",\"key\":\"%s\",\"ver\":\"v1.1\"}",
        device_tag, secret_key);

    return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}

int build_data_upload_request(char* out_buf, size_t buf_size, const char* device_tag, uint8_t value) {
		if (out_buf == NULL || buf_size == 0) return -1;
		int n  = sprintf(out_buf, "{\"t\":3,\"data\":{\"%s\":%d}}", device_tag, value);

		return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}


/* ====================================================
 *  RUTINAS DE UART / ESP8266 (Comunicaci?n WiFi)
 * ==================================================== */

// Limpia el buffer donde se guardan los mensajes recibidos por WiFi
void ClearRxBuffer(void) {
    __disable_irq(); // Deshabilita interrupciones para no corromper datos al borrar
    memset((char *)AT_RX_BUF, 0, MAX_AT_RX_LEN); // Llena el buffer con ceros
    AT_RX_COUNT = 0; // Reinicia el contador a cero
    __enable_irq();  // Vuelve a habilitar interrupciones
}

// Env?a un comando "AT" al m?dulo WiFi ESP8266
void SendAtCmd(char *cmd) {
    ClearRxBuffer(); // Limpia respuestas anteriores
    // Env?a el comando de texto
    HAL_UART_Transmit(&huart4, (uint8_t*)cmd, strlen(cmd), 1000);
    // Env?a un salto de l?nea (Enter) para que el m?dulo lo ejecute
    HAL_UART_Transmit(&huart4, (uint8_t*)"\r\n", 2, 100);
    HAL_Delay(100); // Espera un poco a que el m?dulo procese
}

// Env?a datos en bruto sin salto de l?nea adicional
void SendAtData(char *data) {
    HAL_UART_Transmit(&huart4, (uint8_t*)data, strlen(data), 1000);
}

// Espera a que aparezca una palabra clave en el texto recibido del m?dulo WiFi
uint8_t WaitFor(char *keyword, uint32_t timeout_ms) {
    uint32_t start = HAL_GetTick(); // Toma el tiempo actual
    // Mientras no pase el tiempo l?mite (timeout)
    while((HAL_GetTick() - start) < timeout_ms) {
        // Busca la palabra clave dentro del buffer de recepci?n
        if(strstr((const char *)AT_RX_BUF, keyword) != NULL) return 1; // Encontrada! Retorna ?xito (1)
        HAL_Delay(10); // Espera 10ms antes de volver a buscar
    }
    return 0; // No se encontr? en el tiempo l?mite. Retorna fallo (0)
}

// Extrae la informaci?n ?til (JSON) de un mensaje recibido por la red
void ESP8266_GetIpData(uint8_t *src, char *dest) {
    char *p = strchr((char *)src, '{'); // Busca el inicio de un objeto JSON
    if(p != NULL) { strcpy(dest, p); return; } // Si lo encuentra, copia desde ah?
    p = strchr((char *)src, ':'); // Si no, busca dos puntos
    if(p != NULL) { strcpy(dest, p + 1); return; }
    dest[0] = '\0'; // Si no hay nada, devuelve vac?o
}

// Env?a un mensaje de texto al servidor a trav?s de la conexi?n WiFi TCP
uint8_t SendDataToServer(char *data) {
    char cmd[64];
    uint8_t attempts = 0;
    while(attempts < 3) { // Intenta hasta 3 veces
        ClearRxBuffer();
        sprintf(cmd, "AT+CIPSEND=%d", strlen(data)); // Prepara el comando indicando cu?ntos bytes se enviar?n
        SendAtCmd(cmd);
        if(!WaitFor(">", 3000)) { attempts++; continue; } // Espera el s?mbolo '>' que indica que puede escribir
        HAL_Delay(50);
        SendAtData(data); // Env?a los datos reales
        HAL_UART_Transmit(&huart4, (uint8_t*)"\r\n", 2, 100); // Manda Enter
        if(WaitFor("SEND OK", 5000)) return 1; // Si el m?dulo confirma, retorna ?xito
        attempts++;
        HAL_Delay(200);
    }
    return 0; // Fall? tras 3 intentos
}

// Funci?n envoltorio para enviar datos de IP (compatibilidad)
void ESP8266_IpSend(char *data) { SendDataToServer(data); }

// Env?a el valor de un sensor en formato JSON al servidor
uint8_t ESP8266_SendSensor(char *tag, uint8_t value) {
    char json[128];
    // Formatea el mensaje JSON requerido por NLE Cloud para tipo 4 (telemetr?a)
		if (build_data_upload_request(json, sizeof(json), tag, value) < 0) {
        LCD_Clr();
        LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: JSON Build");
        return -8;
    }

    return SendDataToServer(json); // Env?a al servidor
}

/* ====================================================
 *  I2C POR SOFTWARE (Para leer SENSOR LDR)
 *  (Se simula el protocolo I2C encendiendo y apagando pines)
 * ==================================================== */
// Peque?o retardo para dar tiempo a los niveles l?gicos del I2C
static inline void I2C_Delay(void) { for(volatile int i=0;i<10;i++); }

// Genera la condici?n de INICIO (Start) de I2C
void I2C_Start(void) {
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_RESET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
    I2C_Delay();
}

// Genera la condici?n de PARADA (Stop) de I2C
void I2C_Stop(void) {
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_RESET);
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
    I2C_Delay();
}

// Escribe 1 byte (8 bits) en el bus I2C bit por bit
void I2C_WriteByte(uint8_t byte) {
    for(int i=0;i<8;i++) {
        HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, (byte & 0x80) ? GPIO_PIN_SET : GPIO_PIN_RESET); // Saca el bit m?s significativo
        byte <<= 1; // Rota a la izquierda para el siguiente bit
        I2C_Delay();
        HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET); // Sube Reloj
        I2C_Delay();
        HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET); // Baja Reloj
        I2C_Delay();
    }
}

// Lee 1 byte (8 bits) desde el bus I2C
uint8_t I2C_ReadByte(uint8_t ack) {
    uint8_t byte = 0;
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET); // Libera l?nea de datos
    I2C_Delay();
    for(int i=0;i<8;i++) {
        byte <<= 1; // Prepara espacio
        HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET); // Sube reloj
        I2C_Delay();
        // Lee el estado del pin de datos
        if(HAL_GPIO_ReadPin(I2C_PORT, I2C_SDA_PIN) == GPIO_PIN_SET) byte |= 0x01;
        HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET); // Baja reloj
        I2C_Delay();
    }
    // Env?a confirmaci?n (ACK) o no confirmaci?n (NACK)
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, ack ? GPIO_PIN_RESET : GPIO_PIN_SET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET); // Pulso de reloj para el ACK
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
    return byte;
}

// Espera a que el chip esclavo confirme (ACK) que recibi? un dato
uint8_t I2C_WaitAck(void) {
    uint8_t ack;
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
    I2C_Delay();
    ack = HAL_GPIO_ReadPin(I2C_PORT, I2C_SDA_PIN); // Lee el ACK
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
    I2C_Delay();
    return (ack == GPIO_PIN_RESET) ? 1 : 0; // Si es 0, s? hubo ACK
}

// Funci?n para leer todos los 4 canales anal?gicos del chip PCF8591 de un solo golpe
uint8_t PCF8591_ReadAllChannels(uint8_t* channels) {
    I2C_Start();
    I2C_WriteByte(PCF8591_ADDR_WRITE); // Llama al chip
    if(!I2C_WaitAck()) { I2C_Stop(); return 0; }
    I2C_WriteByte(0x04); // Comando: Activar auto-incremento (lee canal 0, luego 1, 2, 3 autom?tico)
    I2C_WaitAck();
    I2C_Stop();
    HAL_Delay(2);
    I2C_Start();
    I2C_WriteByte(PCF8591_ADDR_READ); // Pide leer
    if(!I2C_WaitAck()) { I2C_Stop(); return 0; }
    I2C_ReadByte(1); // Lectura basura inicial (Dummy read) t?pica de este chip
    channels[0] = I2C_ReadByte(1); // Lee Canal 0 (ACK)
    channels[1] = I2C_ReadByte(1); // Lee Canal 1 (ACK)
    channels[2] = I2C_ReadByte(1); // Lee Canal 2 (ACK)
    channels[3] = I2C_ReadByte(0); // Lee Canal 3 (NACK - ?ltimo byte)
    I2C_Stop();
    return 1;
}

// Funci?n alternativa para leer solo un canal (LDR)
uint8_t PCF8591_ReadLDR(void) {
    uint8_t valor = 0;
    I2C_Start();
    I2C_WriteByte(PCF8591_ADDR_WRITE);
    if(!I2C_WaitAck()) { I2C_Stop(); return 0; }
    I2C_WriteByte(PCF8591_CHANNEL_0); // Pide canal 0
    I2C_WaitAck();
    I2C_Stop();
    HAL_Delay(2);
    I2C_Start();
    I2C_WriteByte(PCF8591_ADDR_READ);
    if(!I2C_WaitAck()) { I2C_Stop(); return 0; }
    I2C_ReadByte(1); // Dummy read
    valor = I2C_ReadByte(0); // Valor real
    I2C_Stop();
    return (255 - valor);
}

/* ====================================================
 *  CONTROLADORES DE REL? (LAMPARA Y VENTILADOR)
 * ==================================================== */
// Enciende (1) o apaga (0) la l?mpara (Pin PA1)
void setLamp(uint8_t estado) {
    lampState = estado; // Actualiza variable global
    HAL_GPIO_WritePin(GPIOA, GPIO_PIN_1, estado ? GPIO_PIN_SET : GPIO_PIN_RESET);
}

// Enciende (1) o apaga (0) el ventilador (Pin PA0)
void setFan(uint8_t estado) {
    fanState = estado; // Actualiza variable global
    HAL_GPIO_WritePin(GPIOA, GPIO_PIN_0, estado ? GPIO_PIN_SET : GPIO_PIN_RESET);
}

void setLock(uint8_t estado) {
		lockState = estado;
		HAL_GPIO_WritePin(GPIOA, GPIO_PIN_2, estado ? GPIO_PIN_SET : GPIO_PIN_RESET);
}

// Controladores para los servos
static uint16_t Servo_AngleToPulse(uint16_t angle, uint16_t physical_max) {
    if(angle > physical_max) angle = physical_max;
    return SERVO_MIN_PULSE + (angle * (SERVO_MAX_PULSE - SERVO_MIN_PULSE) / physical_max);
}

void Servo1_SetAngle(int16_t angle) {
    if(angle > SERVO1_MAX_ANGLE) angle = SERVO1_MAX_ANGLE;
    if(angle < SERVO1_MIN_ANGLE) angle = SERVO1_MIN_ANGLE;
    servo1_angle = (uint16_t)angle;
    __HAL_TIM_SET_COMPARE(&htim3, TIM_CHANNEL_1, Servo_AngleToPulse(servo1_angle, SERVO1_PHYSICAL_MAX));
}

void Servo2_SetAngle(int16_t angle) {
    if(angle > SERVO2_MAX_ANGLE) angle = SERVO2_MAX_ANGLE;
    if(angle < SERVO2_MIN_ANGLE) angle = SERVO2_MIN_ANGLE;
    servo2_angle = (uint16_t)angle;
    __HAL_TIM_SET_COMPARE(&htim3, TIM_CHANNEL_2, Servo_AngleToPulse(servo2_angle, SERVO2_PHYSICAL_MAX));
}
void Servo_Home(void) {
    Servo1_SetAngle(90);
    Servo2_SetAngle(90);
}

uint8_t Servo1_CommandAngle(int16_t angle)
{
    if(angle < SERVO1_MIN_ANGLE || angle > SERVO1_MAX_ANGLE) {
        return 0; // reject invalid
    }
    Servo1_SetAngle((uint16_t)angle);  // hardware update

    /* Immediate cloud report */
    ESP8266_SendSensor((char*)SERVO1_STATE_TAG, (uint8_t)servo1_angle);
    return 1;
}

/**
 * @brief Sets Servo 2 angle and reports it to NLE Cloud.
 */
uint8_t Servo2_CommandAngle(int16_t angle)
{
    if(angle < SERVO2_MIN_ANGLE || angle > SERVO2_MAX_ANGLE) {
        return 0;
    }
    Servo2_SetAngle((uint16_t)angle);

    ESP8266_SendSensor((char*)SERVO2_STATE_TAG, (uint8_t)servo2_angle);
    return 1;
}

/**
 * @brief Homes both servos and reports both states.
 */
void Servo_CommandHome(void)
{
    Servo_Home(); // uses pure hardware functions internally

    ESP8266_SendSensor((char*)SERVO1_STATE_TAG, (uint8_t)servo1_angle);
    ESP8266_SendSensor((char*)SERVO2_STATE_TAG, (uint8_t)servo2_angle);
}

/* ====================================================
 *  CONEXION AL SERVIDOR (SECUENCIA ESTABLE NLE)
 * ==================================================== */
// Realiza la secuencia AT para conectar WiFi, TCP e Iniciar Sesi?n en la nube
int8_t ConnectToServer(void) {
    char cmd[128];
    char json[200];

    // Informa en la pantalla LCD
    LCD_Clr();
    LCD_WriteEnglishString(0, 0, (unsigned char*)"Cargando...");

    SendAtCmd("AT+RESTORE"); // Reinicia m?dulo WiFi
    HAL_Delay(3000);
    ClearRxBuffer();
    HAL_Delay(2000);

    // 1. Verificar comunicaci?n con el m?dulo
    uint8_t at_ok = 0;
    for(int i = 0; i < 5; i++) {
        SendAtCmd("AT");
        if(WaitFor("OK", 2000)) { at_ok = 1; break; }
        HAL_Delay(500);
    }
    if(!at_ok) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: AT Timeout"); return -1; }

    // 2. Modo Estaci?n (Cliente WiFi)
    SendAtCmd("AT+CWMODE_CUR=1");
    if(!WaitFor("OK", 3000)) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: CWMODE"); return -2; }

    // 3. Desactivar Echo para respuestas m?s limpias
    SendAtCmd("ATE0");
    if(!WaitFor("OK", 2000)) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: ATE0"); return -3; }

    // 4. Conectar a la red WiFi local
    sprintf(cmd, "AT+CWJAP_CUR=\"%s\",\"%s\"", WIFI_SSID, WIFI_PASSWORD);
    SendAtCmd(cmd);
    if(!WaitFor("OK", 20000)) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: WiFi"); return -4; }

    // 5. Conectar al socket TCP del Servidor
    sprintf(cmd, "AT+CIPSTART=\"TCP\",\"%s\",%d", SERVER_IP, SERVER_PORT);
    SendAtCmd(cmd);
    if(!WaitFor("CONNECT", 10000)) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: TCP Conn"); return -5; }

    // 6. Enviar mensaje de autenticaci?n (Login JSON)
    if (build_connection_request(json, sizeof(json), DEVICE_TAG, SECRET_KEY) < 0) {
        LCD_Clr();
        LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: JSON Build");
        return -6;
    }

    sprintf(cmd, "AT+CIPSEND=%d", strlen(json));
    SendAtCmd(cmd);
    if(!WaitFor(">", 3000)) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: Send Timeout"); return -7; }

    SendAtData(json);
    HAL_UART_Transmit(&huart4, (uint8_t*)"\r\n", 2, 100);

    if(!WaitFor("SEND OK", 5000)) { LCD_Clr(); LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: Send Failed"); return -8; }

    // 7. Esperar respuesta de servidor confirmando login ("status":0)
    if(!WaitFor("\"status\":0", 10000)) {
        LCD_Clr();
        LCD_WriteEnglishString(0, 0, (unsigned char*)"Err: NLE Login");
        // Rutina para imprimir el error exacto en pantalla si falla
        char dbg[32];
        memset(dbg, 0, sizeof(dbg));
        char* p = strstr((char*)AT_RX_BUF, "\"status\"");
        if(p != NULL) {
            strncpy(dbg, p, 16);
        } else {
            strncpy(dbg, (char*)AT_RX_BUF + (AT_RX_COUNT > 16 ? AT_RX_COUNT - 16 : 0), 16);
        }
        for(int k=0; k<16; k++) {
            if(dbg[k] < 32 || dbg[k] > 126) dbg[k] = ' '; // Quita saltos de linea
        }
        LCD_WriteEnglishString(2, 0, (unsigned char*)dbg);
        return -9;
    }

    cloud_connected = 1; // Todo fue exitoso!
    return 0;
}

static int32_t extract_cmdid(const char *json) {
    const char *p = strstr(json, "\"cmdid\"");
    if (!p) return -1;
    p += 7;
    while (*p && *p != ':') p++;
    if (!*p) return -1;
    p++;
    while (*p == ' ' || *p == '\t') p++;
    return atoi(p);
}

void ESP8266_DataAnalysisProcess(char *RxBuf) {
    if(RxBuf[0] == '\0') return;

    /* --- heartbeat ping (unchanged) --- */
    if(strstr(RxBuf, "$#AT#") != NULL) {
        HAL_UART_Transmit(&huart4, (uint8_t*)"$OK##\r", 7, 100);
        return;
    }

    /* --- only process type-5 command requests --- */
    if(strstr(RxBuf, "\"t\":5") == NULL) return;

    int32_t cmdid = extract_cmdid(RxBuf);
    if(cmdid < 0) return;                 // can't reply without a cmdid

    /* --- parse every command present in the payload --- */
    int8_t lamp_val = -1;   // -1 = not present, 0 = off, 1 = on, -2 = parse error
    int8_t fan_val  = -1;
    int8_t lock_val = -1;
    int16_t servo1_val = -1;
    int16_t servo2_val = -1;
    uint8_t has_error = 0;

    const char *pLamp = strstr(RxBuf, "\"nl_lamp\"");
    if(pLamp != NULL) {
        while(*pLamp && *pLamp != '1' && *pLamp != '0') pLamp++;
        if(*pLamp == '1')      { setLamp(1); lamp_val = 1; }
        else if(*pLamp == '0') { setLamp(0); lamp_val = 0; }
        else                   { lamp_val = -2; has_error = 1; }
    }

    const char *pFan = strstr(RxBuf, "\"nl_fan\"");
    if(pFan != NULL) {
        while(*pFan && *pFan != '1' && *pFan != '0') pFan++;
        if(*pFan == '1')      { setFan(1); fan_val = 1; }
        else if(*pFan == '0') { setFan(0); fan_val = 0; }
        else                  { fan_val = -2; has_error = 1; }
    }

    const char *pLock = strstr(RxBuf, "\"nl_lock\"");
    if(pLock != NULL) {
        while(*pLock && *pLock != '1' && *pLock != '0') pLock++;
        if(*pLock == '1')      { setLock(1); lock_val = 1; }
        else if(*pLock == '0') { setLock(0); lock_val = 0; }
        else                   { lock_val = -2; has_error = 1; }
    }

    /* --- SERVO 1 --- */
    const char *pServo1 = strstr(RxBuf, "\"" SERVO1_TAG "\"");
    if(pServo1 != NULL) {
        pServo1 = strchr(pServo1, ':');
        if(pServo1 != NULL) {
            while(*pServo1 && (*pServo1 < '0' || *pServo1 > '9')) pServo1++;
            int angle = atoi(pServo1);
            if(Servo1_CommandAngle(angle)) {
                servo1_val = angle;
            } else {
                servo1_val = -2;
                has_error = 1;
            }
        }
    }

    /* --- SERVO 2 --- */
    const char *pServo2 = strstr(RxBuf, "\"" SERVO2_TAG "\"");
    if(pServo2 != NULL) {
        pServo2 = strchr(pServo2, ':');
        if(pServo2 != NULL) {
            while(*pServo2 && (*pServo2 < '0' || *pServo2 > '9')) pServo2++;
            int angle = atoi(pServo2);
            if(Servo2_CommandAngle(angle)) {
                servo2_val = angle;
            } else {
                servo2_val = -2;
                has_error = 1;
            }
        }
    }

    /* --- HOME --- */
    if(strstr(RxBuf, "\"solar_home\"") != NULL) {
        Servo_CommandHome();
        servo1_val = servo1_angle;
        servo2_val = servo2_angle;
    }

    /* --- nothing recognized? bail out --- */
    if(lamp_val == -1 && fan_val == -1 && lock_val == -1 &&
       servo1_val == -1 && servo2_val == -1) return;

    /* --- build one combined "data" object --- */
    char data_obj[256];
    int pos = 0;
    pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "{");

    int first = 1;
    if(lamp_val != -1) {
        if(!first) pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, ",");
        first = 0;
        pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "\"nl_lampState\":%d", lamp_val);
    }
    if(fan_val != -1) {
        if(!first) pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, ",");
        first = 0;
        pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "\"nl_fanState\":%d", fan_val);
    }
    if(lock_val != -1) {
        if(!first) pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, ",");
        first = 0;
        pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "\"nl_lockState\":%d", lock_val);
    }

    /* === SERVO STATES: always reported, no matter what === */
    if(!first) pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, ",");
    first = 0;
    pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "\"nl_servoState1\":%d", (int)servo1_angle);

    if(!first) pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, ",");
    first = 0;
    pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "\"nl_servoState2\":%d", (int)servo2_angle);

    /* === BRIGHTNESS: always reported as integer === */
    if(!first) pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, ",");
    first = 0;
    pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "\"brightness\":%d", (int)valorLDR);

    pos += snprintf(data_obj + pos, sizeof(data_obj) - pos, "}");

    /* --- response --- */
    char resp[512];
    snprintf(resp, sizeof(resp),
        "{\"t\":3,\"datatype\":1,\"status\":%d,\"datas\":%s}",
         has_error ? 1 : 0, data_obj);

    ESP8266_IpSend(resp);
}
/* USER CODE END 0 */
/* ====================================================
 *  B?SQUEDA LOCAL PERI?DICA
 *  Desde la posici?n actual, explora un paso en cada
 *  direcci?n y se mueve solo si encuentra m?s luz.
 *  Actualiza las variables globales de best light.
 * ==================================================== */
void Servo_LocalSearch(void) {
    uint16_t base_s1 = servo1_angle;
    uint16_t base_s2 = servo2_angle;
    uint16_t best_s1 = base_s1;
    uint16_t best_s2 = base_s2;
    uint8_t  best_light, base_light;

    /* Luz en posici?n actual */
    uint32_t sum = 0;
    for(uint8_t i = 0; i < CALIB_LDR_SAMPLES; i++) {
        sum += PCF8591_ReadLDR();
        HAL_Delay(5);
    }
    base_light = (uint8_t)(sum / CALIB_LDR_SAMPLES);
    best_light = base_light;

    /* 4 direcciones cardinales */
    const int8_t dir[4][2] = {
        { LOCAL_SEARCH_STEP,  0 },
        { -LOCAL_SEARCH_STEP, 0 },
        { 0,  LOCAL_SEARCH_STEP },
        { 0, -LOCAL_SEARCH_STEP }
    };

    for(uint8_t d = 0; d < 4; d++) {
        int16_t try_s1 = (int16_t)base_s1 + dir[d][0];
        int16_t try_s2 = (int16_t)base_s2 + dir[d][1];

        if(try_s1 < SERVO1_MIN_ANGLE) try_s1 = SERVO1_MIN_ANGLE;
        if(try_s1 > SERVO1_MAX_ANGLE) try_s1 = SERVO1_MAX_ANGLE;
        if(try_s2 < SERVO2_MIN_ANGLE) try_s2 = SERVO2_MIN_ANGLE;
        if(try_s2 > SERVO2_MAX_ANGLE) try_s2 = SERVO2_MAX_ANGLE;

        if((uint16_t)try_s1 == base_s1 && (uint16_t)try_s2 == base_s2) continue;

        Servo1_SetAngle((uint16_t)try_s1);
        Servo2_SetAngle((uint16_t)try_s2);
        HAL_Delay(CALIB_SETTLE_MS);

        sum = 0;
        for(uint8_t i = 0; i < CALIB_LDR_SAMPLES; i++) {
            sum += PCF8591_ReadLDR();
            HAL_Delay(5);
        }
        uint8_t try_light = (uint8_t)(sum / CALIB_LDR_SAMPLES);

        if(try_light > best_light) {
            best_light = try_light;
            best_s1 = (uint16_t)try_s1;
            best_s2 = (uint16_t)try_s2;
        }
    }

    /* Vuelve a la mejor posici?n (original o vecino) */
    Servo1_SetAngle(best_s1);
    Servo2_SetAngle(best_s2);

    /* Actualiza el best global si esta b?squeda local mejor? el r?cord hist?rico */
    if(best_light > g_best_light) {
        g_best_light = best_light;
        g_best_s1    = best_s1;
        g_best_s2    = best_s2;
    }
}

/* ====================================================
 *  CALIBRACION SERVOS -> MAXIMA LUZ (LDR)
 *  Ejecuta al inicio. Barre ambos servos y guarda la
 *  combinacion de angulos con mayor lectura de luz.
 * ==================================================== */
void CalibrateServosToLight(void) {
        uint16_t best_s1 = SERVO1_MIN_ANGLE;
    uint16_t best_s2 = SERVO2_MIN_ANGLE;
    uint8_t  best_light = 0;

    LCD_Clr();
    LCD_WriteEnglishString(0, 0, (unsigned char*)"Buscando luz...");

    /* Barrido de Servo 1 */
    for(uint16_t s1 = SERVO1_MIN_ANGLE; ; s1 += CALIB_ANGLE_STEP) {
        uint16_t angle1 = (s1 > SERVO1_MAX_ANGLE) ? SERVO1_MAX_ANGLE : s1;
        Servo1_SetAngle(angle1);

        /* Barrido de Servo 2 */
        for(uint16_t s2 = SERVO2_MIN_ANGLE; ; s2 += CALIB_ANGLE_STEP) {
            uint16_t angle2 = (s2 > SERVO2_MAX_ANGLE) ? SERVO2_MAX_ANGLE : s2;
            Servo2_SetAngle(angle2);

            HAL_Delay(CALIB_SETTLE_MS);

            /* Promedia varias lecturas para estabilidad */
            uint32_t ldr_sum = 0;
            for(uint8_t i = 0; i < CALIB_LDR_SAMPLES; i++) {
                ldr_sum += PCF8591_ReadLDR();
                HAL_Delay(5);
            }
            uint8_t current_light = (uint8_t)(ldr_sum / CALIB_LDR_SAMPLES);

            /* Muestra progreso en LCD */
            char buf[32];
            sprintf(buf, "A1:%3d A2:%3d", angle1, angle2);
            LCD_WriteEnglishString(2, 0, (unsigned char*)buf);
            sprintf(buf, "LDR:%3d B:%3d", current_light, best_light);
            LCD_WriteEnglishString(4, 0, (unsigned char*)buf);

            /* Guarda el mejor valor encontrado */
            if(current_light > best_light) {
                best_light = current_light;
                best_s1   = angle1;
                best_s2   = angle2;
            }

            if(angle2 >= SERVO2_MAX_ANGLE) break;
        }

        if(angle1 >= SERVO1_MAX_ANGLE) break;
    }

    /* Mueve servos a la mejor posicion encontrada */
		Servo1_SetAngle(best_s1);
    Servo2_SetAngle(best_s2);

    /* Guarda en variables globales para el tracking continuo */
    g_best_light = best_light;
    g_best_s1    = best_s1;
    g_best_s2    = best_s2;

    LCD_Clr();
    LCD_WriteEnglishString(0, 0, (unsigned char*)"Mejor luz encontrada");
    char buf[32];
    sprintf(buf, "S1:%3d S2:%3d", best_s1, best_s2);
    LCD_WriteEnglishString(2, 0, (unsigned char*)buf);
    sprintf(buf, "LDR:%3d", best_light);
    LCD_WriteEnglishString(4, 0, (unsigned char*)buf);

    HAL_Delay(3000);
}
int main(void) {
    /* === INICIALIZACI?N DEL SISTEMA === */
    HAL_Init();             // Inicializa librer?a HAL
    SystemClock_Config();   // Configura relojes del microcontrolador
    MX_GPIO_Init();         // Configura pines entrada/salida
    MX_RTC_Init();          // Inicializa el reloj interno
    MX_UART4_Init();        // Configura el puerto serie 4
    MX_USART1_UART_Init();  // Nuevo - Inicializa USART1 (STM32CubeMX)
    MX_TIM3_Init();         // Nuevo - Inicializa TIM3 (STM32CubeMX)

    /* USER CODE BEGIN 2 */
    /* KEY1 eliminado — hardware no funciona */

    uint8_t last_key2 = GPIO_PIN_SET;
    uint32_t key2_debounce_ms = 0;

    /* NUEVO: Variables para doble acci?n de KEY2 */
    uint8_t  key2_is_pressed  = 0;
    uint8_t  key2_long_done   = 0;
    uint32_t key2_press_start = 0;

    /* Configuraci?n manual de pines para el I2C por software (PA4 y PA5) */
    GPIO_InitTypeDef GPIO_InitStruct = {0};
    GPIO_InitStruct.Pin   = I2C_SCL_PIN | I2C_SDA_PIN;
    GPIO_InitStruct.Mode  = GPIO_MODE_OUTPUT_OD; // Open Drain, necesario para I2C
    GPIO_InitStruct.Pull  = GPIO_PULLUP;         // Resistencias Pull-up activadas
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
    HAL_GPIO_Init(I2C_PORT, &GPIO_InitStruct);
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN | I2C_SDA_PIN, GPIO_PIN_SET); // Nivel ALTO de reposo
    HAL_Delay(10);

    /* Inicializar la pantalla LCD */
    LCD_Init(); // Enciende y configura la pantalla
    LCD_Clr();  // Limpia cualquier basura

    /* Arrancar la recepci?n por UART mediante Interrupciones (1 byte a la vez) */
    HAL_UART_Receive_IT(&huart4, &rx_byte, 1);
    ClearRxBuffer();

    // reset servos
    Servo_Home();
    HAL_Delay(300);
    CalibrateServosToLight();

    /* Intento de Conexi?n a la Nube (m?x 3 intentos) */
    int8_t result = -99;
    for(int attempt = 0; attempt < 3; attempt++) {
        result = ConnectToServer();
        if(result == 0) break; // Sali? bien, sale del bucle
        HAL_Delay(5000); // Espera 5 segundos antes de reintentar
    }

    // Si no conect? despu?s de los reintentos, bloquea el sistema mostrando el error
    if(result != 0) {
        char errBuf[16];
        sprintf(errBuf, "Codigo: %d", result);
        LCD_WriteEnglishString(2, 0, (unsigned char*)errBuf);
        while(1) { HAL_Delay(1000); } // Bucle infinito de error
    } else {
        // Conexi?n exitosa, mostrar en LCD
        LCD_Clr();
        LCD_WriteEnglishString(0, 0, (unsigned char*)"Conectado NLE!");
    }

    /* Inicializar timestamps para decay y b?squeda local */
    last_best_decay   = HAL_GetTick();
    last_local_search = HAL_GetTick();

    /* USER CODE END 2 */

    /* === BUCLE INFINITO PRINCIPAL (SUPERLOOP) === */
    while(1) {
        if(cloud_connected) { // Si estamos conectados al servidor
            uint32_t now = HAL_GetTick(); // Tiempo en milisegundos actual

            /* TAREA 1: Leer Sensor de Luz (LDR) cada medio segundo (500ms) */
            if((now - last_ldr_read) > 500) {
                uint8_t ch[4];
                if(PCF8591_ReadAllChannels(ch)) {
                    valorLDR = 255 - ch[0]; // Invertido para que + luz = n?mero m?s alto
                }
                last_ldr_read = now;

                /* Evaluar si la luz cambi? dr?sticamente para enviar un reporte inmediato */
                int16_t diff = (int16_t)valorLDR - (int16_t)valorLDRAnterior;
                if(diff < 0) diff = -diff; // Valor absoluto
                if(diff > 5) { // Si vari? m?s de 5 unidades
                    ESP8266_SendSensor((char*)LDR_TAG, valorLDR);
                    valorLDRAnterior = valorLDR;
                }
            }

            /* TAREA 2: Enviar se?al de vida (Heartbeat/Ping) cada 30 segundos */
            if((now - last_heartbeat) > 30000) {
                SendDataToServer((char*)"{\"t\":2}"); // Tipo 2 = Ping a NLE Cloud
                last_heartbeat = now;
            }

            /* TAREA 3: Reporte c?clico de estado a la nube cada 3 segundos */
            if((now - last_sensor_report) > 3000) {
                static uint8_t sensor_turn = 0; // Variable est?tica para recordar el turno

                // Manda un dato diferente en cada ciclo para no saturar la red
                if(sensor_turn == 0) {
                    ESP8266_SendSensor((char*)"brightness", valorLDR);
                } else if(sensor_turn == 1) {
                    ESP8266_SendSensor((char*)"nl_lampState", lampState);
                } else if(sensor_turn == 2) {
                    ESP8266_SendSensor((char*)"nl_fanState", fanState);
                } else if(sensor_turn == 3) {
                    ESP8266_SendSensor((char*)"nl_lockState", lockState);
                } else if(sensor_turn == 4) {
                    ESP8266_SendSensor((char*)"nl_servoState1", (uint8_t)servo1_angle);
                } else if(sensor_turn == 5) {
                    ESP8266_SendSensor((char*)"nl_servoState2", (uint8_t)servo2_angle);
                }

                sensor_turn++;
                if(sensor_turn > 5) sensor_turn = 0;

                last_sensor_report = now;
            }

            /* TAREA 4: Actualizar la pantalla LCD local cada 2 segundos */
            if((now - last_lcd_update) > 2000) {
                char lcdBuf[32];
                // Fila 2: Imprimir valor LDR (con 3 espacios reservados para alinear %3d)
                sprintf(lcdBuf, "LDR:%3d ", valorLDR);
                LCD_WriteEnglishString(2, 0, (unsigned char*)lcdBuf);

                // Fila 4: Imprimir estado Lampara
                sprintf(lcdBuf, "LMP:%s ", lampState ? "ON " : "OFF");
                LCD_WriteEnglishString(4, 0, (unsigned char*)lcdBuf);

                // Fila 6: Imprimir estado Ventilador
                sprintf(lcdBuf, "VEN:%s ", fanState ? "ON " : "OFF");
                LCD_WriteEnglishString(6, 0, (unsigned char*)lcdBuf);

                // Fila 7: Imprimir estado Cerradura
                sprintf(lcdBuf, "LOK:%s ", lockState ? "ON " : "OFF");
                LCD_WriteEnglishString(2, 64, (unsigned char*)lcdBuf);

                sprintf(lcdBuf, "S1:%3d", servo1_angle);
                LCD_WriteEnglishString(4, 64, (unsigned char*)lcdBuf);

                sprintf(lcdBuf, "S2:%3d", servo2_angle);
                LCD_WriteEnglishString(6, 64, (unsigned char*)lcdBuf);

                last_lcd_update = now;
            }

            /* TAREA 5: Decay del best_light global */
            if((now - last_best_decay) > BEST_LIGHT_DECAY_MS) {
                if(g_best_light >= BEST_LIGHT_DECAY_AMOUNT) {
                    g_best_light -= BEST_LIGHT_DECAY_AMOUNT;
                } else {
                    g_best_light = 0;
                }
                last_best_decay = now;
            }

            /* TAREA 6: B?squeda local cada 20 segundos */
            if((now - last_local_search) > LOCAL_SEARCH_INTERVAL_MS) {
                Servo_LocalSearch();
                last_local_search = now;
            }
        }

        /* =========================================================
         *  CONTROL LOCAL POR BOTON KEY2 (DOBLE ACCION)
         *  Corto (< 800 ms) = Ventilador  |  Largo (>= 800 ms) = Cerradura
         * ========================================================= */
        uint8_t key2 = HAL_GPIO_ReadPin(KEY2_GPIO_Port, KEY2_Pin);
        uint32_t now_btn = HAL_GetTick();

        /* --- Detectar PRESION (flanco de bajada) con debounce --- */
        if(key2 == GPIO_PIN_RESET && last_key2 == GPIO_PIN_SET) {
            if((now_btn - key2_debounce_ms) > 200) {
                key2_is_pressed  = 1;
                key2_long_done   = 0;
                key2_press_start = now_btn;
            }
        }

        /* --- Mientras se MANTIENE presionado: revisar umbral largo --- */
        if(key2 == GPIO_PIN_RESET && key2_is_pressed && !key2_long_done) {
            if((now_btn - key2_press_start) >= KEY2_LONG_THRESHOLD_MS) {
                setLock(lockState ? 0 : 1);          // Toggle Cerradura
                if(cloud_connected) {
                    ESP8266_SendSensor((char*)"nl_lockState", lockState);
                }
                key2_long_done = 1;                  // Marcar que ya se ejecuto largo
            }
        }

        /* --- Detectar SUELTA (flanco de subida) --- */
        if(key2 == GPIO_PIN_SET && last_key2 == GPIO_PIN_RESET) {
            /* Si se solto ANTES del umbral largo => fue un toque corto */
            if(key2_is_pressed && !key2_long_done) {
                setFan(fanState ? 0 : 1);            // Toggle Ventilador
                if(cloud_connected) {
                    ESP8266_SendSensor((char*)"nl_fanState", fanState);
                }
            }
            key2_is_pressed = 0;
            key2_long_done  = 0;
            key2_debounce_ms = now_btn;
        }

        last_key2 = key2;

        /* TAREA 7: Procesar comandos as?ncronos recibidos desde Internet */
        if(AT_RX_COUNT > 0) {
            // Si recibe un PING directo
            if(strstr((const char*)AT_RX_BUF, "$#AT#") != NULL) {
                HAL_UART_Transmit(&huart4, (uint8_t*)"$OK##\r", 7, 100); // Contesta OK
                ClearRxBuffer();
            }
            // Si recibe un JSON completo (asume por la llave '}')
            else if(strstr((const char*)AT_RX_BUF, "}") != NULL) {
                HAL_Delay(5); // Peque?o margen para recibir caracteres finales
                ESP8266_GetIpData((uint8_t*)AT_RX_BUF, IpData); // Extrae el JSON puro
                ESP8266_DataAnalysisProcess(IpData); // Se lo pasa al int?rprete para encender/apagar cosas
                memset(IpData, 0x00, sizeof(IpData));
                ClearRxBuffer(); // Limpia para recibir el pr?ximo mensaje
            }
            // Mecanismo de seguridad: Si el buffer casi se llena, limpiarlo para evitar desbordamiento
            else if(AT_RX_COUNT >= MAX_AT_RX_LEN - 10) {
                ClearRxBuffer();
            }
        }

        HAL_Delay(10); // Peque?o retraso global del bucle para estabilidad
    }
}

/* ====================================================
 *  INTERRUPCI?N UART (Receptor Serial)
 * ==================================================== */
// Esta funci?n se ejecuta autom?ticamente (Hardware) cada vez que el ESP8266 env?a 1 byte
void HAL_UART_RxCpltCallback(UART_HandleTypeDef *huart) {
    if(huart->Instance == UART4) {
        // Evita escribir fuera de los l?mites del arreglo
        if(AT_RX_COUNT < MAX_AT_RX_LEN - 1) {
            AT_RX_BUF[AT_RX_COUNT] = rx_byte; // Guarda el byte le?do
            AT_RX_COUNT++;                    // Avanza un espacio
            AT_RX_BUF[AT_RX_COUNT] = '\0';    // Cierra la cadena (string de C)
        } else {
            // Si se llen? por alg?n error, limpia.
            AT_RX_COUNT  = 0;
            AT_RX_BUF[0] = '\0';
        }
        // Vuelve a armar la interrupci?n para escuchar el siguiente byte
        HAL_UART_Receive_IT(&huart4, &rx_byte, 1);
    }
}

/* ====================================================
 *  FUNCIONES DE CONFIGURACI?N DE HARDWARE B?SICO
 *  (Generadas autom?ticamente por STM32CubeMX)
 * ==================================================== */

// Configuraci?n del reloj principal del procesador a su frecuencia de trabajo
// ACTUALIZADO: Ahora usa HSE (cuarzo externo) + PLL para 72 MHz (m?s r?pido que HSI 8MHz)
void SystemClock_Config(void)
{
  RCC_OscInitTypeDef RCC_OscInitStruct = {0};
  RCC_ClkInitTypeDef RCC_ClkInitStruct = {0};
  RCC_PeriphCLKInitTypeDef PeriphClkInit = {0};

  /** Initializes the RCC Oscillators according to the specified parameters
  * in the RCC_OscInitTypeDef structure.
  */
  RCC_OscInitStruct.OscillatorType = RCC_OSCILLATORTYPE_LSI|RCC_OSCILLATORTYPE_HSE;
  RCC_OscInitStruct.HSEState = RCC_HSE_ON;
  RCC_OscInitStruct.HSEPredivValue = RCC_HSE_PREDIV_DIV1;
  RCC_OscInitStruct.HSIState = RCC_HSI_ON;
  RCC_OscInitStruct.LSIState = RCC_LSI_ON;
  RCC_OscInitStruct.PLL.PLLState = RCC_PLL_ON;
  RCC_OscInitStruct.PLL.PLLSource = RCC_PLLSOURCE_HSE;
  RCC_OscInitStruct.PLL.PLLMUL = RCC_PLL_MUL9;
  if (HAL_RCC_OscConfig(&RCC_OscInitStruct) != HAL_OK)
  {
    Error_Handler();
  }

  /** Initializes the CPU, AHB and APB buses clocks
  */
  RCC_ClkInitStruct.ClockType = RCC_CLOCKTYPE_HCLK|RCC_CLOCKTYPE_SYSCLK
                              |RCC_CLOCKTYPE_PCLK1|RCC_CLOCKTYPE_PCLK2;
  RCC_ClkInitStruct.SYSCLKSource = RCC_SYSCLKSOURCE_PLLCLK;
  RCC_ClkInitStruct.AHBCLKDivider = RCC_SYSCLK_DIV1;
  RCC_ClkInitStruct.APB1CLKDivider = RCC_HCLK_DIV2;
  RCC_ClkInitStruct.APB2CLKDivider = RCC_HCLK_DIV1;

  if (HAL_RCC_ClockConfig(&RCC_ClkInitStruct, FLASH_LATENCY_2) != HAL_OK)
  {
    Error_Handler();
  }
  PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_RTC;
  PeriphClkInit.RTCClockSelection = RCC_RTCCLKSOURCE_LSI;
  if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK)
  {
    Error_Handler();
  }
}

// Configuraci?n del Reloj de Tiempo Real interno (por si el proyecto usa fechas/horas)
// ACTUALIZADO: Ahora usa formato BCD como genera STM32CubeMX
static void MX_RTC_Init(void)
{
  /* USER CODE BEGIN RTC_Init 0 */
  /* USER CODE END RTC_Init 0 */

  RTC_TimeTypeDef sTime = {0};
  RTC_DateTypeDef DateToUpdate = {0};

  /* USER CODE BEGIN RTC_Init 1 */
  /* USER CODE END RTC_Init 1 */

  /** Initialize RTC Only
  */
  hrtc.Instance = RTC;
  hrtc.Init.AsynchPrediv = RTC_AUTO_1_SECOND;
  hrtc.Init.OutPut = RTC_OUTPUTSOURCE_ALARM;
  if (HAL_RTC_Init(&hrtc) != HAL_OK)
  {
    Error_Handler();
  }

  /* USER CODE BEGIN Check_RTC_BKUP */
  /* USER CODE END Check_RTC_BKUP */

  /** Initialize RTC and set the Time and Date
  */
  sTime.Hours = 0x0;
  sTime.Minutes = 0x0;
  sTime.Seconds = 0x0;

  if (HAL_RTC_SetTime(&hrtc, &sTime, RTC_FORMAT_BCD) != HAL_OK)
  {
    Error_Handler();
  }
  DateToUpdate.WeekDay = RTC_WEEKDAY_MONDAY;
  DateToUpdate.Month = RTC_MONTH_JANUARY;
  DateToUpdate.Date = 0x1;
  DateToUpdate.Year = 0x0;

  if (HAL_RTC_SetDate(&hrtc, &DateToUpdate, RTC_FORMAT_BCD) != HAL_OK)
  {
    Error_Handler();
  }
  /* USER CODE BEGIN RTC_Init 2 */
  /* USER CODE END RTC_Init 2 */
}

// Configuraci?n del puerto Serie 4 (Conectado al m?dulo ESP8266) a 115200 baudios
// ACTUALIZADO: Con secciones USER CODE para personalizaci?n
static void MX_UART4_Init(void)
{
  /* USER CODE BEGIN UART4_Init 0 */
  /* USER CODE END UART4_Init 0 */

  /* USER CODE BEGIN UART4_Init 1 */
  /* USER CODE END UART4_Init 1 */
  huart4.Instance = UART4;
  huart4.Init.BaudRate = 115200;
  huart4.Init.WordLength = UART_WORDLENGTH_8B;
  huart4.Init.StopBits = UART_STOPBITS_1;
  huart4.Init.Parity = UART_PARITY_NONE;
  huart4.Init.Mode = UART_MODE_TX_RX;
  huart4.Init.HwFlowCtl = UART_HWCONTROL_NONE;
  huart4.Init.OverSampling = UART_OVERSAMPLING_16;
  if (HAL_UART_Init(&huart4) != HAL_OK)
  {
    Error_Handler();
  }
  /* USER CODE BEGIN UART4_Init 2 */
  /* USER CODE END UART4_Init 2 */
}

// NUEVO: Configuraci?n del puerto Serie USART1 a 115200 baudios (generado por STM32CubeMX)
static void MX_USART1_UART_Init(void)
{
  /* USER CODE BEGIN USART1_Init 0 */
  /* USER CODE END USART1_Init 0 */

  /* USER CODE BEGIN USART1_Init 1 */
  /* USER CODE END USART1_Init 1 */
  huart1.Instance = USART1;
  huart1.Init.BaudRate = 115200;
  huart1.Init.WordLength = UART_WORDLENGTH_8B;
  huart1.Init.StopBits = UART_STOPBITS_1;
  huart1.Init.Parity = UART_PARITY_NONE;
  huart1.Init.Mode = UART_MODE_TX_RX;
  huart1.Init.HwFlowCtl = UART_HWCONTROL_NONE;
  huart1.Init.OverSampling = UART_OVERSAMPLING_16;
  if (HAL_UART_Init(&huart1) != HAL_OK)
  {
    Error_Handler();
  }
  /* USER CODE BEGIN USART1_Init 2 */
  /* USER CODE END USART1_Init 2 */
}

// NUEVO: Configuraci?n del temporizador TIM3 en modo PWM (generado por STM32CubeMX)
static void MX_TIM3_Init(void)
{
  /* USER CODE BEGIN TIM3_Init 0 */
  /* USER CODE END TIM3_Init 0 */

  TIM_MasterConfigTypeDef sMasterConfig = {0};
  TIM_OC_InitTypeDef sConfigOC = {0};

  /* USER CODE BEGIN TIM3_Init 1 */
  /* USER CODE END TIM3_Init 1 */
  htim3.Instance = TIM3;
  htim3.Init.Prescaler = 72 - 1;
  htim3.Init.CounterMode = TIM_COUNTERMODE_UP;
  htim3.Init.Period = 20000 - 1;
  htim3.Init.ClockDivision = TIM_CLOCKDIVISION_DIV1;
  htim3.Init.AutoReloadPreload = TIM_AUTORELOAD_PRELOAD_ENABLE;
  if (HAL_TIM_PWM_Init(&htim3) != HAL_OK)
  {
    Error_Handler();
  }
  sMasterConfig.MasterOutputTrigger = TIM_TRGO_RESET;
  sMasterConfig.MasterSlaveMode = TIM_MASTERSLAVEMODE_DISABLE;
  if (HAL_TIMEx_MasterConfigSynchronization(&htim3, &sMasterConfig) != HAL_OK)
  {
    Error_Handler();
  }
  sConfigOC.OCMode = TIM_OCMODE_PWM1;
  sConfigOC.Pulse = 1500;
  sConfigOC.OCPolarity = TIM_OCPOLARITY_HIGH;
  sConfigOC.OCFastMode = TIM_OCFAST_DISABLE;
  if (HAL_TIM_PWM_ConfigChannel(&htim3, &sConfigOC, TIM_CHANNEL_1) != HAL_OK)
  {
    Error_Handler();
  }
  sConfigOC.Pulse = 1500;
  if (HAL_TIM_PWM_ConfigChannel(&htim3, &sConfigOC, TIM_CHANNEL_2) != HAL_OK)
  {
    Error_Handler();
  }
  /* USER CODE BEGIN TIM3_Init 2 */
  if (HAL_TIM_PWM_Start(&htim3, TIM_CHANNEL_1) != HAL_OK)
  {
    Error_Handler();
  }
  if (HAL_TIM_PWM_Start(&htim3, TIM_CHANNEL_2) != HAL_OK)
  {
    Error_Handler();
  }
  /* USER CODE END TIM3_Init 2 */
}

// Configuraci?n de los pines de entrada/salida (GPIOs)
// ACTUALIZADO: Ahora habilita relojes de todos los puertos usados por la placa
static void MX_GPIO_Init(void)
{
    GPIO_InitTypeDef GPIO_InitStruct = {0};

    // Encender el reloj de TODOS los puertos a usar (ACTUALIZADO por STM32CubeMX)
    __HAL_RCC_GPIOC_CLK_ENABLE();
    __HAL_RCC_GPIOD_CLK_ENABLE();
    __HAL_RCC_GPIOA_CLK_ENABLE();
    __HAL_RCC_GPIOB_CLK_ENABLE();
    __HAL_RCC_GPIOE_CLK_ENABLE();

    /* PIN PA0: CONTROL DEL VENTILADOR (Rel?) */
    GPIO_InitStruct.Pin   = GPIO_PIN_0;
    GPIO_InitStruct.Mode  = GPIO_MODE_OUTPUT_PP; // Push-Pull (Corriente normal de salida)
    GPIO_InitStruct.Pull  = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);
    HAL_GPIO_WritePin(GPIOA, GPIO_PIN_0, GPIO_PIN_RESET); // Iniciar Apagado

    /* PIN PA1: CONTROL DE LA L?MPARA (Rel?) */
    GPIO_InitStruct.Pin = GPIO_PIN_1;
    GPIO_InitStruct.Mode  = GPIO_MODE_OUTPUT_PP; // Push-Pull
    GPIO_InitStruct.Pull  = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);
    HAL_GPIO_WritePin(GPIOA, GPIO_PIN_1, GPIO_PIN_RESET); // Iniciar Apagado

    /* PA2 = LOCK */
    GPIO_InitStruct.Pin = GPIO_PIN_2;
    GPIO_InitStruct.Mode  = GPIO_MODE_OUTPUT_PP;
    GPIO_InitStruct.Pull  = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);
    HAL_GPIO_WritePin(GPIOA, GPIO_PIN_2, GPIO_PIN_RESET);

    /* PINES PARA BOTONES (Ej. PC13, PD13) configurados como entradas con resistencia pull-up */
    GPIO_InitStruct.Pin  = GPIO_PIN_13;
    GPIO_InitStruct.Mode = GPIO_MODE_INPUT;
    GPIO_InitStruct.Pull = GPIO_PULLUP;
    HAL_GPIO_Init(GPIOC, &GPIO_InitStruct);
    HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);

    /* PA6 - TIM3_CH1 (Servo 1) */
    GPIO_InitStruct.Pin = GPIO_PIN_6;
    GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
    GPIO_InitStruct.Pull = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

    /* PA7 - TIM3_CH2 (Servo 2) */
    GPIO_InitStruct.Pin = GPIO_PIN_7;
    GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
    GPIO_InitStruct.Pull = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

		    /* PC13: KEY1 (Entrada con pull-up interno) */
    GPIO_InitStruct.Pin   = KEY1_Pin;
    GPIO_InitStruct.Mode  = GPIO_MODE_INPUT;
    GPIO_InitStruct.Pull  = GPIO_PULLUP;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
    HAL_GPIO_Init(KEY1_GPIO_Port, &GPIO_InitStruct);

    /* PD13: KEY2 (Entrada con pull-up interno) */
    GPIO_InitStruct.Pin   = KEY2_Pin;
    GPIO_InitStruct.Mode  = GPIO_MODE_INPUT;
    GPIO_InitStruct.Pull  = GPIO_PULLUP;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
    HAL_GPIO_Init(KEY2_GPIO_Port, &GPIO_InitStruct);
}

// Funci?n que atrapa errores fatales de configuraci?n
// ACTUALIZADO: Con secciones USER CODE para depuraci?n
void Error_Handler(void)
{
  /* USER CODE BEGIN Error_Handler_Debug */
  /* User can add his own implementation to report the HAL error return state */
  __disable_irq(); // Apaga todas las interrupciones
  while (1)
  {
    HAL_Delay(1000); // Queda bloqueado para siempre
  }
  /* USER CODE END Error_Handler_Debug */
}

// Utilidad por si falla una aserci?n de memoria/librer?a
#ifdef USE_FULL_ASSERT
void assert_failed(uint8_t *file, uint32_t line)
{
  /* USER CODE BEGIN 6 */
  /* User can add his own implementation to report the file name and line number,
     ex: printf("Wrong parameters value: file %s on line %d\r\n", file, line) */
  /* USER CODE END 6 */
}
#endif
