/* USER CODE BEGIN Header /
/**
 ******************************************************************************
 * @file           : main.c
 * @brief          : NLE CLOUD - LAMPARA (PA1) + VENTILADOR (PA0) + LDR
 * (PCF8591)
 * @version        : 28.0 FINAL - LIGERO (MODO 100% MANUAL)
 ******************************************************************************
 */
/* USER CODE END Header */

#include "main.h"
#include "lcd12864.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

RTC_HandleTypeDef hrtc;
UART_HandleTypeDef huart4;
UART_HandleTypeDef huart1;
TIM_HandleTypeDef htim3;

/* USER CODE BEGIN PV */
/* === CONSTANTES DE CONFIGURACION === */
#define MAX_AT_RX_LEN 512
#define WIFI_SSID "IoT-Room10H"
#define WIFI_PASSWORD "IoT-Room10H"
#define SERVER_IP "121.37.241.174"
#define SERVER_PORT 8600
#define DEVICE_TAG "p1476967_dev1"
#define DEVICE_ID "1542363"
#define SECRET_KEY "726637791de84a41bf0a8c45c16afc36"

/* === SENSOR & ACTUATOR TAGS (Control and state report share the same tag) ===
 */
#define TAG_BRIGHTNESS "brightness"
#define TAG_LAMP "lamp"
#define TAG_FAN "fan"
#define TAG_LOCK "lock"
#define TAG_SERVO_X "servo_x"
#define TAG_SERVO_Y "servo_y"


#define KEY2_LONG_THRESHOLD_MS 800

/* === SERVO CONFIG === */
#define SERVO_MIN_PULSE 500
#define SERVO_MAX_PULSE 2500
#define SERVO1_MIN_ANGLE 0
#define SERVO1_MAX_ANGLE 180
#define SERVO1_PHYSICAL_MAX 180

#define SERVO2_MIN_ANGLE 0
#define SERVO2_MAX_ANGLE 90
#define SERVO2_PHYSICAL_MAX 180

/* === BEST LIGHT TRACKING === */
#define BEST_LIGHT_DECAY_AMOUNT 5
#define BEST_LIGHT_DECAY_MS 10000
#define LOCAL_SEARCH_STEP 10
#define LOCAL_SEARCH_INTERVAL_MS 5000

/* === I2C PINS (PCF8591 LDR) === */
#define PCF8591_ADDR_WRITE 0x90
#define PCF8591_ADDR_READ 0x91
#define PCF8591_CHANNEL_0 0x00
#define I2C_SCL_PIN GPIO_PIN_4
#define I2C_SDA_PIN GPIO_PIN_5
#define I2C_PORT GPIOA

/* === SERVO LIGHT CALIBRATION CONFIG === */
#define CALIB_ANGLE_STEP 10
#define CALIB_SETTLE_MS 250
#define CALIB_LDR_SAMPLES 3

/* === VARIABLES GLOBALES === */
volatile char AT_RX_BUF[MAX_AT_RX_LEN];
volatile uint16_t AT_RX_COUNT = 0;
uint8_t rx_byte = 0;
uint8_t cloud_connected = 0;
uint32_t last_heartbeat = 0;
uint32_t last_sensor_report = 0;
uint32_t last_ldr_read = 0;
uint8_t lampState = 0;
uint8_t fanState = 0;
uint8_t lockState = 0;
uint8_t valorLDR = 0;
uint8_t valorLDRAnterior = 0;
uint32_t last_lcd_update = 0;
uint16_t servo1_angle = 90;
uint16_t servo2_angle = 90;
char IpData[256];

/* === BEST LIGHT TRACKING GLOBALES === */
uint8_t g_best_light = 0;
uint16_t g_best_s1 = 90;
uint16_t g_best_s2 = 90;
uint32_t last_best_decay = 0;
uint32_t last_local_search = 0;
/* USER CODE END PV */

void SystemClock_Config(void);
static void MX_GPIO_Init(void);
static void MX_RTC_Init(void);
static void MX_UART4_Init(void);
static void MX_USART1_UART_Init(void);
static void MX_TIM3_Init(void);

/* USER CODE BEGIN 0 */

// Helpers para construir payloads
int build_connection_request(char *out_buf, size_t buf_size,
                             const char *device_tag, const char *secret_key) {
  if (out_buf == NULL || buf_size == 0)
    return -1;

  int n =
      snprintf(out_buf, buf_size,
               "{\"t\":1,\"device\":\"%s\",\"key\":\"%s\",\"ver\":\"v1.1\"}",
               device_tag, secret_key);

  return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}

static uint16_t g_msgid = 1;

int build_data_upload_request(char *out_buf, size_t buf_size,
                              const char *tag, int32_t value) {
  if (out_buf == NULL || buf_size == 0)
    return -1;
  int n = snprintf(out_buf, buf_size,
                   "{\"t\":3,\"datatype\":1,\"datas\":{\"%s\":%d},\"msgid\":%u}",
                   tag, (int)value, (unsigned int)g_msgid++);
  if (g_msgid > 65000)
    g_msgid = 1;

  return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}

int build_cmd_response(char *out_buf, size_t buf_size,
                       int32_t cmdid, uint8_t status, int32_t data) {
  if (out_buf == NULL || buf_size == 0)
    return -1;

  int n = snprintf(out_buf, buf_size,
                   "{\"t\":6,\"cmdid\":%ld,\"status\":%d,\"data\":%ld}",
                   (long)cmdid, (int)status, (long)data);

  return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}

/* ====================================================
 *  UART / ESP8266 (WiFi Communication)
 * ==================================================== */

void ClearRxBuffer(void) {
  __disable_irq();
  memset((char *)AT_RX_BUF, 0, MAX_AT_RX_LEN);
  AT_RX_COUNT = 0;
  __enable_irq();
}

void SendAtCmd(char *cmd) {
  ClearRxBuffer();
  HAL_UART_Transmit(&huart4, (uint8_t *)cmd, strlen(cmd), 1000);
  HAL_UART_Transmit(&huart4, (uint8_t *)"\r\n", 2, 100);
  HAL_Delay(100);
}

void SendAtData(char *data) {
  HAL_UART_Transmit(&huart4, (uint8_t *)data, strlen(data), 1000);
}

uint8_t WaitFor(char *keyword, uint32_t timeout_ms) {
  uint32_t start = HAL_GetTick();
  while ((HAL_GetTick() - start) < timeout_ms) {
    if (strstr((const char *)AT_RX_BUF, keyword) != NULL)
      return 1;
    HAL_Delay(10);
  }
  return 0;
}

void ESP8266_GetIpData(uint8_t *src, char *dest) {
  char *p = strchr((char *)src, '{');
  if (p != NULL) {
    strcpy(dest, p);
    return;
  }
  p = strchr((char *)src, ':');
  if (p != NULL) {
    strcpy(dest, p + 1);
    return;
  }
  dest[0] = '\0';
}

uint8_t SendDataToServer(char *data) {
  char cmd[64];
  uint8_t attempts = 0;
  while (attempts < 3) {
    ClearRxBuffer();
    sprintf(cmd, "AT+CIPSEND=%d", strlen(data));
    SendAtCmd(cmd);
    if (!WaitFor(">", 3000)) {
      attempts++;
      continue;
    }
    HAL_Delay(50);
    SendAtData(data);
    HAL_UART_Transmit(&huart4, (uint8_t *)"\r\n", 2, 100);
    if (WaitFor("SEND OK", 5000))
      return 1;
    attempts++;
    HAL_Delay(200);
  }
  return 0;
}

void ESP8266_IpSend(char *data) { SendDataToServer(data); }

uint8_t ESP8266_SendSensor(char *tag, int32_t value) {
  char json[128];
  if (build_data_upload_request(json, sizeof(json), tag, value) < 0) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: JSON Build");
    return -8;
  }
  return SendDataToServer(json);
}

/* ====================================================
 *  I2C (PCF8591 / LDR)
 * ==================================================== */
static inline void I2C_Delay(void) {
  for (volatile int i = 0; i < 10; i++)
    ;
}

void I2C_Start(void) {
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_RESET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
  I2C_Delay();
}

void I2C_Stop(void) {
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_RESET);
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
  I2C_Delay();
}

void I2C_WriteByte(uint8_t byte) {
  for (int i = 0; i < 8; i++) {
    HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN,
                      (byte & 0x80) ? GPIO_PIN_SET : GPIO_PIN_RESET);
    byte <<= 1;
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
    I2C_Delay();
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
    I2C_Delay();
  }
}

uint8_t I2C_ReadByte(uint8_t ack) {
  uint8_t byte = 0;
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
  I2C_Delay();
  for (int i = 0; i < 8; i++) {
    byte <<= 1;
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
    I2C_Delay();
    if (HAL_GPIO_ReadPin(I2C_PORT, I2C_SDA_PIN) == GPIO_PIN_SET)
      byte |= 0x01;
    HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
    I2C_Delay();
  }
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, ack ? GPIO_PIN_RESET : GPIO_PIN_SET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
  return byte;
}

uint8_t I2C_WaitAck(void) {
  uint8_t ack;
  HAL_GPIO_WritePin(I2C_PORT, I2C_SDA_PIN, GPIO_PIN_SET);
  I2C_Delay();
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_SET);
  I2C_Delay();
  ack = HAL_GPIO_ReadPin(I2C_PORT, I2C_SDA_PIN);
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN, GPIO_PIN_RESET);
  I2C_Delay();
  return (ack == GPIO_PIN_RESET) ? 1 : 0;
}

uint8_t PCF8591_ReadAllChannels(uint8_t *channels) {
  I2C_Start();
  I2C_WriteByte(PCF8591_ADDR_WRITE);
  if (!I2C_WaitAck()) {
    I2C_Stop();
    return 0;
  }
  I2C_WriteByte(0x04);
  I2C_WaitAck();
  I2C_Stop();
  HAL_Delay(2);
  I2C_Start();
  I2C_WriteByte(PCF8591_ADDR_READ);
  if (!I2C_WaitAck()) {
    I2C_Stop();
    return 0;
  }
  I2C_ReadByte(1);
  channels[0] = I2C_ReadByte(1);
  channels[1] = I2C_ReadByte(1);
  channels[2] = I2C_ReadByte(1);
  channels[3] = I2C_ReadByte(0);
  I2C_Stop();
  return 1;
}

uint8_t PCF8591_ReadLDR(void) {
  uint8_t valor = 0;
  I2C_Start();
  I2C_WriteByte(PCF8591_ADDR_WRITE);
  if (!I2C_WaitAck()) {
    I2C_Stop();
    return 0;
  }
  I2C_WriteByte(PCF8591_CHANNEL_0);
  I2C_WaitAck();
  I2C_Stop();
  HAL_Delay(2);
  I2C_Start();
  I2C_WriteByte(PCF8591_ADDR_READ);
  if (!I2C_WaitAck()) {
    I2C_Stop();
    return 0;
  }
  I2C_ReadByte(1);
  valor = I2C_ReadByte(0);
  I2C_Stop();
  return (255 - valor);
}

/* ====================================================
 *  ACTUATORS (LAMP, FAN, LOCK)
 * ==================================================== */
void setLamp(uint8_t estado) {
  lampState = estado;
  HAL_GPIO_WritePin(GPIOA, GPIO_PIN_1, estado ? GPIO_PIN_SET : GPIO_PIN_RESET);
}

void setFan(uint8_t estado) {
  fanState = estado;
  HAL_GPIO_WritePin(GPIOA, GPIO_PIN_0, estado ? GPIO_PIN_SET : GPIO_PIN_RESET);
}

void setLock(uint8_t estado) {
  lockState = estado;
  HAL_GPIO_WritePin(GPIOA, GPIO_PIN_2, estado ? GPIO_PIN_SET : GPIO_PIN_RESET);
}

// Controladores para los servos
static uint16_t Servo_AngleToPulse(uint16_t angle, uint16_t physical_max) {
  if (angle > physical_max)
    angle = physical_max;
  return SERVO_MIN_PULSE +
         (angle * (SERVO_MAX_PULSE - SERVO_MIN_PULSE) / physical_max);
}

void Servo1_SetAngle(int16_t angle) {
  if (angle > SERVO1_MAX_ANGLE)
    angle = SERVO1_MAX_ANGLE;
  if (angle < SERVO1_MIN_ANGLE)
    angle = SERVO1_MIN_ANGLE;
  servo1_angle = (uint16_t)angle;
  __HAL_TIM_SET_COMPARE(&htim3, TIM_CHANNEL_1,
                        Servo_AngleToPulse(servo1_angle, SERVO1_PHYSICAL_MAX));
}

void Servo2_SetAngle(int16_t angle) {
  if (angle > SERVO2_MAX_ANGLE)
    angle = SERVO2_MAX_ANGLE;
  if (angle < SERVO2_MIN_ANGLE)
    angle = SERVO2_MIN_ANGLE;
  servo2_angle = (uint16_t)angle;
  __HAL_TIM_SET_COMPARE(&htim3, TIM_CHANNEL_2,
                        Servo_AngleToPulse(servo2_angle, SERVO2_PHYSICAL_MAX));
}
void Servo_Home(void) {
  Servo1_SetAngle(90);
  Servo2_SetAngle(90);
}

uint8_t Servo1_CommandAngle(int16_t angle) {
  if (angle < SERVO1_MIN_ANGLE || angle > SERVO1_MAX_ANGLE) {
    return 0;
  }
  Servo1_SetAngle((uint16_t)angle);
  ESP8266_SendSensor((char *)TAG_SERVO_X, (uint8_t)servo1_angle);
  return 1;
}

uint8_t Servo2_CommandAngle(int16_t angle) {
  if (angle < SERVO2_MIN_ANGLE || angle > SERVO2_MAX_ANGLE) {
    return 0;
  }
  Servo2_SetAngle((uint16_t)angle);
  ESP8266_SendSensor((char *)TAG_SERVO_Y, (uint8_t)servo2_angle);
  return 1;
}

void Servo_CommandHome(void) {
  Servo_Home();
  ESP8266_SendSensor((char *)TAG_SERVO_X, (uint8_t)servo1_angle);
  ESP8266_SendSensor((char *)TAG_SERVO_Y, (uint8_t)servo2_angle);
}

/* ====================================================
 *  SERVER CONNECTION (NLE Cloud via AT commands)
 * ==================================================== */
int8_t ConnectToServer(void) {
  char cmd[128];
  char json[200];

  LCD_Clr();
  LCD_WriteString(0, 0, "Cargando...");

  SendAtCmd("AT+RESTORE");
  HAL_Delay(3000);
  ClearRxBuffer();
  HAL_Delay(2000);

  uint8_t at_ok = 0;
  for (int i = 0; i < 5; i++) {
    SendAtCmd("AT");
    if (WaitFor("OK", 2000)) {
      at_ok = 1;
      break;
    }
    HAL_Delay(500);
  }
  if (!at_ok) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: AT Timeout");
    return -1;
  }

  SendAtCmd("AT+CWMODE_CUR=1");
  if (!WaitFor("OK", 3000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: CWMODE");
    return -2;
  }

  SendAtCmd("ATE0");
  if (!WaitFor("OK", 2000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: ATE0");
    return -3;
  }

  sprintf(cmd, "AT+CWJAP_CUR=\"%s\",\"%s\"", WIFI_SSID, WIFI_PASSWORD);
  SendAtCmd(cmd);
  if (!WaitFor("OK", 20000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: WiFi");
    return -4;
  }

  sprintf(cmd, "AT+CIPSTART=\"TCP\",\"%s\",%d", SERVER_IP, SERVER_PORT);
  SendAtCmd(cmd);
  if (!WaitFor("CONNECT", 10000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: TCP Conn");
    return -5;
  }

  if (build_connection_request(json, sizeof(json), DEVICE_TAG, SECRET_KEY) <
      0) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: JSON Build");
    return -6;
  }

  sprintf(cmd, "AT+CIPSEND=%d", strlen(json));
  SendAtCmd(cmd);
  if (!WaitFor(">", 3000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: Send Timeout");
    return -7;
  }

  SendAtData(json);
  HAL_UART_Transmit(&huart4, (uint8_t *)"\r\n", 2, 100);

  if (!WaitFor("SEND OK", 5000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: Send Failed");
    return -8;
  }

  if (!WaitFor("\"status\":0", 10000)) {
    LCD_Clr();
    LCD_WriteString(0, 0, "Err: NLE Login");
    char dbg[32];
    memset(dbg, 0, sizeof(dbg));
    char *p = strstr((char *)AT_RX_BUF, "\"status\"");
    if (p != NULL) {
      strncpy(dbg, p, 16);
    } else {
      strncpy(dbg,
              (char *)AT_RX_BUF + (AT_RX_COUNT > 16 ? AT_RX_COUNT - 16 : 0),
              16);
    }
    for (int k = 0; k < 16; k++) {
      if (dbg[k] < 32 || dbg[k] > 126)
        dbg[k] = ' ';
    }
    LCD_WriteString(2, 0, dbg);
    return -9;
  }

  cloud_connected = 1;
  return 0;
}

static int32_t extract_cmdid(const char *json) {
  const char *p = strstr(json, "\"cmdid\"");
  if (!p)
    return -1;
  p += 7;
  while (*p && (*p == ' ' || *p == ':' || *p == '\t'))
    p++;
  return atoi(p);
}

static int32_t extract_data_int(const char *json) {
  const char *p = strstr(json, "\"data\"");
  if (!p)
    return -1;
  p += 6;
  while (*p && (*p == ' ' || *p == ':' || *p == '\t' || *p == '\"'))
    p++;
  if (*p == 't' || *p == 'T')
    return 1;
  if (*p == 'f' || *p == 'F')
    return 0;
  if (*p >= '0' && *p <= '9')
    return atoi(p);
  return -1;
}

static int32_t extract_tag_int(const char *json, const char *tag) {
  char pattern[64];
  snprintf(pattern, sizeof(pattern), "\"%s\"", tag);
  const char *p = strstr(json, pattern);
  if (!p)
    return -1;
  p += strlen(pattern);
  while (*p && (*p == ' ' || *p == ':' || *p == '\t' || *p == '\"'))
    p++;
  if (*p == 't' || *p == 'T')
    return 1;
  if (*p == 'f' || *p == 'F')
    return 0;
  if (*p >= '0' && *p <= '9')
    return atoi(p);
  return -1;
}

static int32_t get_actuator_val(const char *json, const char *tag) {
  char pattern[64];
  snprintf(pattern, sizeof(pattern), "\"%s\"", tag);
  if (!strstr(json, pattern))
    return -1;
  int32_t val = extract_data_int(json);
  if (val != -1)
    return val;
  return extract_tag_int(json, tag);
}

void ESP8266_DataAnalysisProcess(char *RxBuf) {
  if (RxBuf[0] == '\0')
    return;

  /* --- Heartbeat inquiry from server (protocol fixed string) --- */
  if (strstr(RxBuf, "$#AT#") != NULL) {
    HAL_UART_Transmit(&huart4, (uint8_t *)"$OK##\r", 7, 100);
    return;
  }

  /* --- Only process type-5 command requests (CMD_REQ) --- */
  if (strstr(RxBuf, "\"t\":5") == NULL)
    return;

  int32_t cmdid = extract_cmdid(RxBuf);
  if (cmdid < 0)
    return; // cannot respond without cmdid

  uint8_t executed = 0;
  uint8_t has_error = 0;

  /* 1. Lamp Actuator */
  if (strstr(RxBuf, "\"" TAG_LAMP "\"") != NULL) {
    int32_t val = get_actuator_val(RxBuf, TAG_LAMP);
    if (val == 1 || val == 0) {
      setLamp((uint8_t)val);
      executed = 1;
    } else {
      has_error = 1;
    }
  }

  /* 2. Fan Actuator */
  if (strstr(RxBuf, "\"" TAG_FAN "\"") != NULL) {
    int32_t val = get_actuator_val(RxBuf, TAG_FAN);
    if (val == 1 || val == 0) {
      setFan((uint8_t)val);
      executed = 1;
    } else {
      has_error = 1;
    }
  }

  /* 3. Lock Actuator */
  if (strstr(RxBuf, "\"" TAG_LOCK "\"") != NULL) {
    int32_t val = get_actuator_val(RxBuf, TAG_LOCK);
    if (val == 1 || val == 0) {
      setLock((uint8_t)val);
      executed = 1;
    } else {
      has_error = 1;
    }
  }

  /* 4. Servo X Actuator (0 to 180 degrees) */
  if (strstr(RxBuf, "\"" TAG_SERVO_X "\"") != NULL) {
    int32_t val = get_actuator_val(RxBuf, TAG_SERVO_X);
    if (val >= SERVO1_MIN_ANGLE && val <= SERVO1_MAX_ANGLE) {
      Servo1_SetAngle((uint16_t)val);
      executed = 1;
    } else {
      has_error = 1;
    }
  }

  /* 5. Servo Y Actuator (0 to 90 degrees) */
  if (strstr(RxBuf, "\"" TAG_SERVO_Y "\"") != NULL) {
    int32_t val = get_actuator_val(RxBuf, TAG_SERVO_Y);
    if (val >= SERVO2_MIN_ANGLE && val <= SERVO2_MAX_ANGLE) {
      Servo2_SetAngle((uint16_t)val);
      executed = 1;
    } else {
      has_error = 1;
    }
  }

  /* 6. Home command */
  if (strstr(RxBuf, "\"solar_home\"") != NULL) {
    Servo_Home();
    executed = 1;
  }

  if (!executed && !has_error)
    return;

  /* --- Step A: Send Command Response (t: 6) as required by NLECloud protocol --- */
  char cmd_resp[64];
  if (build_cmd_response(cmd_resp, sizeof(cmd_resp), cmdid, has_error ? 1 : 0, 0) > 0) {
    SendDataToServer(cmd_resp);
  }

  /* --- Step B: In NLECloud, the actuator automatically creates a state with the same name.
                 Report current state using t: 3 so cloud dashboard reflects changes immediately. --- */
  char state_upload[256];
  snprintf(state_upload, sizeof(state_upload),
           "{\"t\":3,\"datatype\":1,\"datas\":{\"%s\":%d,\"%s\":%d,\"%s\":%d,\"%s\":%d,\"%s\":%d,\"%s\":%d},\"msgid\":%u}",
           TAG_LAMP, (int)lampState,
           TAG_FAN, (int)fanState,
           TAG_LOCK, (int)lockState,
           TAG_SERVO_X, (int)servo1_angle,
           TAG_SERVO_Y, (int)servo2_angle,
           TAG_BRIGHTNESS, (int)valorLDR,
           (unsigned int)g_msgid++);
  if (g_msgid > 65000)
    g_msgid = 1;

  SendDataToServer(state_upload);
}
/* USER CODE END 0 */

/* ====================================================
 *  LOCAL SEARCH (Light tracking)
 * ==================================================== */
void Servo_LocalSearch(void) {
  uint16_t base_s1 = servo1_angle;
  uint16_t base_s2 = servo2_angle;
  uint16_t best_s1 = base_s1;
  uint16_t best_s2 = base_s2;
  uint8_t best_light, base_light;

  uint32_t sum = 0;
  for (uint8_t i = 0; i < CALIB_LDR_SAMPLES; i++) {
    sum += PCF8591_ReadLDR();
    HAL_Delay(5);
  }
  base_light = (uint8_t)(sum / CALIB_LDR_SAMPLES);
  best_light = base_light;

  const int8_t dir[4][2] = {{LOCAL_SEARCH_STEP, 0},
                            {-LOCAL_SEARCH_STEP, 0},
                            {0, LOCAL_SEARCH_STEP},
                            {0, -LOCAL_SEARCH_STEP}};

  for (uint8_t d = 0; d < 4; d++) {
    int16_t try_s1 = (int16_t)base_s1 + dir[d][0];
    int16_t try_s2 = (int16_t)base_s2 + dir[d][1];

    if (try_s1 < SERVO1_MIN_ANGLE)
      try_s1 = SERVO1_MIN_ANGLE;
    if (try_s1 > SERVO1_MAX_ANGLE)
      try_s1 = SERVO1_MAX_ANGLE;
    if (try_s2 < SERVO2_MIN_ANGLE)
      try_s2 = SERVO2_MIN_ANGLE;
    if (try_s2 > SERVO2_MAX_ANGLE)
      try_s2 = SERVO2_MAX_ANGLE;

    if ((uint16_t)try_s1 == base_s1 && (uint16_t)try_s2 == base_s2)
      continue;

    Servo1_SetAngle((uint16_t)try_s1);
    Servo2_SetAngle((uint16_t)try_s2);
    HAL_Delay(CALIB_SETTLE_MS);

    sum = 0;
    for (uint8_t i = 0; i < CALIB_LDR_SAMPLES; i++) {
      sum += PCF8591_ReadLDR();
      HAL_Delay(5);
    }
    uint8_t try_light = (uint8_t)(sum / CALIB_LDR_SAMPLES);

    if (try_light > best_light) {
      best_light = try_light;
      best_s1 = (uint16_t)try_s1;
      best_s2 = (uint16_t)try_s2;
    }
  }

  Servo1_SetAngle(best_s1);
  Servo2_SetAngle(best_s2);

  if (best_light > g_best_light) {
    g_best_light = best_light;
    g_best_s1 = best_s1;
    g_best_s2 = best_s2;
  }
}

/* ====================================================
 *  CALIBRACION SERVOS -> MAXIMA LUZ (LDR)
 * ==================================================== */
void CalibrateServosToLight(void) {
  uint16_t best_s1 = SERVO1_MIN_ANGLE;
  uint16_t best_s2 = SERVO2_MIN_ANGLE;
  uint8_t best_light = 0;

  LCD_Clr();
  LCD_WriteString(0, 0, "Buscando luz...");

  for (uint16_t s1 = SERVO1_MIN_ANGLE;; s1 += CALIB_ANGLE_STEP) {
    uint16_t angle1 = (s1 > SERVO1_MAX_ANGLE) ? SERVO1_MAX_ANGLE : s1;
    Servo1_SetAngle(angle1);

    for (uint16_t s2 = SERVO2_MIN_ANGLE;; s2 += CALIB_ANGLE_STEP) {
      uint16_t angle2 = (s2 > SERVO2_MAX_ANGLE) ? SERVO2_MAX_ANGLE : s2;
      Servo2_SetAngle(angle2);

      HAL_Delay(CALIB_SETTLE_MS);

      uint32_t ldr_sum = 0;
      for (uint8_t i = 0; i < CALIB_LDR_SAMPLES; i++) {
        ldr_sum += PCF8591_ReadLDR();
        HAL_Delay(5);
      }
      uint8_t current_light = (uint8_t)(ldr_sum / CALIB_LDR_SAMPLES);

      char buf[32];
      sprintf(buf, "A1:%3d A2:%3d", angle1, angle2);
      LCD_WriteString(2, 0, buf);
      sprintf(buf, "LDR:%3d B:%3d", current_light, best_light);
      LCD_WriteString(4, 0, buf);

      if (current_light > best_light) {
        best_light = current_light;
        best_s1 = angle1;
        best_s2 = angle2;
      }

      if (angle2 >= SERVO2_MAX_ANGLE)
        break;
    }

    if (angle1 >= SERVO1_MAX_ANGLE)
      break;
  }

  Servo1_SetAngle(best_s1);
  Servo2_SetAngle(best_s2);

  g_best_light = best_light;
  g_best_s1 = best_s1;
  g_best_s2 = best_s2;

  LCD_Clr();
  LCD_WriteString(0, 0, "Mejor luz encontrada");
  char buf[32];
  sprintf(buf, "S1:%3d S2:%3d", best_s1, best_s2);
  LCD_WriteString(2, 0, buf);
  sprintf(buf, "LDR:%3d", best_light);
  LCD_WriteString(4, 0, buf);

  HAL_Delay(3000);
}
int main(void) {
  HAL_Init();
  SystemClock_Config();
  MX_GPIO_Init();
  MX_RTC_Init();
  MX_UART4_Init();
  MX_USART1_UART_Init();
  MX_TIM3_Init();

  /* USER CODE BEGIN 2 */
  uint8_t last_key2 = GPIO_PIN_SET;
  uint32_t key2_debounce_ms = 0;

  uint8_t key2_is_pressed = 0;
  uint8_t key2_long_done = 0;
  uint32_t key2_press_start = 0;

  GPIO_InitTypeDef GPIO_InitStruct = {0};
  GPIO_InitStruct.Pin = I2C_SCL_PIN | I2C_SDA_PIN;
  GPIO_InitStruct.Mode = GPIO_MODE_OUTPUT_OD;
  GPIO_InitStruct.Pull = GPIO_PULLUP;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
  HAL_GPIO_Init(I2C_PORT, &GPIO_InitStruct);
  HAL_GPIO_WritePin(I2C_PORT, I2C_SCL_PIN | I2C_SDA_PIN, GPIO_PIN_SET);
  HAL_Delay(10);

  LCD_Init();
  LCD_Clr();

  HAL_UART_Receive_IT(&huart4, &rx_byte, 1);
  ClearRxBuffer();

  Servo_Home();
  HAL_Delay(300);
  CalibrateServosToLight();

  int8_t result = -99;
  for (int attempt = 0; attempt < 3; attempt++) {
    result = ConnectToServer();
    if (result == 0)
      break;
    HAL_Delay(5000);
  }

  if (result != 0) {
    char errBuf[16];
    sprintf(errBuf, "Codigo: %d", result);
    LCD_WriteString(2, 0, errBuf);
    while (1) {
      HAL_Delay(1000);
    }
  } else {
    LCD_Clr();
    LCD_WriteString(0, 0, "Conectado NLE!");
  }

  last_best_decay = HAL_GetTick();
  last_local_search = HAL_GetTick();

  /* USER CODE END 2 */

  /* === BUCLE INFINITO PRINCIPAL (SUPERLOOP) === */
  while (1) {
    if (cloud_connected) {
      uint32_t now = HAL_GetTick();

      /* TAREA 1: Leer Sensor de Luz (LDR) cada 500ms */
      if ((now - last_ldr_read) > 500) {
        uint8_t ch[4];
        if (PCF8591_ReadAllChannels(ch)) {
          valorLDR = 255 - ch[0];
        }
        last_ldr_read = now;

        int16_t diff = (int16_t)valorLDR - (int16_t)valorLDRAnterior;
        if (diff < 0)
          diff = -diff;
        if (diff > 5) {
          ESP8266_SendSensor((char *)TAG_BRIGHTNESS, valorLDR);
          valorLDRAnterior = valorLDR;
        }
      }

      /* TAREA 2: Ping a NLE Cloud cada 30s (Heartbeat) */
      if ((now - last_heartbeat) > 30000) {
        SendDataToServer((char *)"$#AT#\r");
        last_heartbeat = now;
      }

      /* TAREA 3: Reporte ciclico de estado a la nube cada 3s */
      if ((now - last_sensor_report) > 3000) {
        static uint8_t sensor_turn = 0;

        if (sensor_turn == 0) {
          ESP8266_SendSensor((char *)TAG_BRIGHTNESS, valorLDR);
        } else if (sensor_turn == 1) {
          ESP8266_SendSensor((char *)TAG_LAMP, lampState);
        } else if (sensor_turn == 2) {
          ESP8266_SendSensor((char *)TAG_FAN, fanState);
        } else if (sensor_turn == 3) {
          ESP8266_SendSensor((char *)TAG_LOCK, lockState);
        } else if (sensor_turn == 4) {
          ESP8266_SendSensor((char *)TAG_SERVO_X, (uint8_t)servo1_angle);
        } else if (sensor_turn == 5) {
          ESP8266_SendSensor((char *)TAG_SERVO_Y, (uint8_t)servo2_angle);
        }

        sensor_turn++;
        if (sensor_turn > 5)
          sensor_turn = 0;

        last_sensor_report = now;
      }

      /* TAREA 4: Actualizar pantalla LCD cada 2s */
      if ((now - last_lcd_update) > 2000) {
        char lcdBuf[32];
        sprintf(lcdBuf, "LDR:%3d ", valorLDR);
        LCD_WriteString(2, 0, lcdBuf);

        sprintf(lcdBuf, "LMP:%s ", lampState ? "ON " : "OFF");
        LCD_WriteString(4, 0, lcdBuf);

        sprintf(lcdBuf, "VEN:%s ", fanState ? "ON " : "OFF");
        LCD_WriteString(6, 0, lcdBuf);

        sprintf(lcdBuf, "LOK:%s ", lockState ? "ON " : "OFF");
        LCD_WriteString(2, 64, lcdBuf);

        sprintf(lcdBuf, "S1:%3d", servo1_angle);
        LCD_WriteString(4, 64, lcdBuf);

        sprintf(lcdBuf, "S2:%3d", servo2_angle);
        LCD_WriteString(6, 64, lcdBuf);

        last_lcd_update = now;
      }

      /* TAREA 5: Decay del best_light global */
      if ((now - last_best_decay) > BEST_LIGHT_DECAY_MS) {
        if (g_best_light >= BEST_LIGHT_DECAY_AMOUNT) {
          g_best_light -= BEST_LIGHT_DECAY_AMOUNT;
        } else {
          g_best_light = 0;
        }
        last_best_decay = now;
      }

      /* TAREA 6: Busqueda local cada 5 segundos */
      if ((now - last_local_search) > LOCAL_SEARCH_INTERVAL_MS) {
        Servo_LocalSearch();
        last_local_search = now;
      }
    }

    /* Control local por boton KEY2 */
    uint8_t key2 = HAL_GPIO_ReadPin(KEY2_GPIO_Port, KEY2_Pin);
    uint32_t now_btn = HAL_GetTick();

    if (key2 == GPIO_PIN_RESET && last_key2 == GPIO_PIN_SET) {
      if ((now_btn - key2_debounce_ms) > 200) {
        key2_is_pressed = 1;
        key2_long_done = 0;
        key2_press_start = now_btn;
      }
    }

    if (key2 == GPIO_PIN_RESET && key2_is_pressed && !key2_long_done) {
      if ((now_btn - key2_press_start) >= KEY2_LONG_THRESHOLD_MS) {
        setLock(lockState ? 0 : 1);
        if (cloud_connected) {
          ESP8266_SendSensor((char *)TAG_LOCK, lockState);
        }
        key2_long_done = 1;
      }
    }

    if (key2 == GPIO_PIN_SET && last_key2 == GPIO_PIN_RESET) {
      if (key2_is_pressed && !key2_long_done) {
        setFan(fanState ? 0 : 1);
        if (cloud_connected) {
          ESP8266_SendSensor((char *)TAG_FAN, fanState);
        }
      }
      key2_is_pressed = 0;
      key2_long_done = 0;
      key2_debounce_ms = now_btn;
    }

    last_key2 = key2;

    /* TAREA 7: Procesar comandos recibidos desde Internet */
    if (AT_RX_COUNT > 0) {
      if (strstr((const char *)AT_RX_BUF, "$#AT#") != NULL) {
        HAL_UART_Transmit(&huart4, (uint8_t *)"$OK##\r", 7, 100);
        ClearRxBuffer();
      } else if (strstr((const char *)AT_RX_BUF, "}") != NULL) {
        HAL_Delay(5);
        ESP8266_GetIpData((uint8_t *)AT_RX_BUF, IpData);
        ESP8266_DataAnalysisProcess(IpData);
        memset(IpData, 0x00, sizeof(IpData));
        ClearRxBuffer();
      } else if (AT_RX_COUNT >= MAX_AT_RX_LEN - 10) {
        ClearRxBuffer();
      }
    }

    HAL_Delay(10);
  }
}

/* ====================================================
 *  INTERRUPCION UART (Receptor Serial)
 * ==================================================== */
void HAL_UART_RxCpltCallback(UART_HandleTypeDef *huart) {
  if (huart->Instance == UART4) {
    if (AT_RX_COUNT < MAX_AT_RX_LEN - 1) {
      AT_RX_BUF[AT_RX_COUNT] = rx_byte;
      AT_RX_COUNT++;
      AT_RX_BUF[AT_RX_COUNT] = '\0';
    } else {
      AT_RX_COUNT = 0;
      AT_RX_BUF[0] = '\0';
    }
    HAL_UART_Receive_IT(&huart4, &rx_byte, 1);
  }
}

/* ====================================================
 *  HARDWARE INITIALIZATION (STM32 HAL / CubeMX)
 * ==================================================== */

void SystemClock_Config(void) {
  RCC_OscInitTypeDef RCC_OscInitStruct = {0};
  RCC_ClkInitTypeDef RCC_ClkInitStruct = {0};
  PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_RTC;
  RCC_PeriphCLKInitTypeDef PeriphClkInit = {0};

  RCC_OscInitStruct.OscillatorType =
      RCC_OSCILLATORTYPE_LSI | RCC_OSCILLATORTYPE_HSE;
  RCC_OscInitStruct.HSEState = RCC_HSE_ON;
  RCC_OscInitStruct.HSEPredivValue = RCC_HSE_PREDIV_DIV1;
  RCC_OscInitStruct.HSIState = RCC_HSI_ON;
  RCC_OscInitStruct.LSIState = RCC_LSI_ON;
  RCC_OscInitStruct.PLL.PLLState = RCC_PLL_ON;
  RCC_OscInitStruct.PLL.PLLSource = RCC_PLLSOURCE_HSE;
  RCC_OscInitStruct.PLL.PLLMUL = RCC_PLL_MUL9;
  if (HAL_RCC_OscConfig(&RCC_OscInitStruct) != HAL_OK) {
    Error_Handler();
  }

  RCC_ClkInitStruct.ClockType = RCC_CLOCKTYPE_HCLK | RCC_CLOCKTYPE_SYSCLK |
                                RCC_CLOCKTYPE_PCLK1 | RCC_CLOCKTYPE_PCLK2;
  RCC_ClkInitStruct.SYSCLKSource = RCC_SYSCLKSOURCE_PLLCLK;
  RCC_ClkInitStruct.AHBCLKDivider = RCC_SYSCLK_DIV1;
  RCC_ClkInitStruct.APB1CLKDivider = RCC_HCLK_DIV2;
  RCC_ClkInitStruct.APB2CLKDivider = RCC_HCLK_DIV1;

  if (HAL_RCC_ClockConfig(&RCC_ClkInitStruct, FLASH_LATENCY_2) != HAL_OK) {
    Error_Handler();
  }
  PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_RTC;
  PeriphClkInit.RTCClockSelection = RCC_RTCCLKSOURCE_LSI;
  if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) {
    Error_Handler();
  }
}

static void MX_RTC_Init(void) {
  RTC_TimeTypeDef sTime = {0};
  RTC_DateTypeDef DateToUpdate = {0};

  hrtc.Instance = RTC;
  hrtc.Init.AsynchPrediv = RTC_AUTO_1_SECOND;
  hrtc.Init.OutPut = RTC_OUTPUTSOURCE_ALARM;
  if (HAL_RTC_Init(&hrtc) != HAL_OK) {
    Error_Handler();
  }

  sTime.Hours = 0x0;
  sTime.Minutes = 0x0;
  sTime.Seconds = 0x0;

  if (HAL_RTC_SetTime(&hrtc, &sTime, RTC_FORMAT_BCD) != HAL_OK) {
    Error_Handler();
  }
  DateToUpdate.WeekDay = RTC_WEEKDAY_MONDAY;
  DateToUpdate.Month = RTC_MONTH_JANUARY;
  DateToUpdate.Date = 0x1;
  DateToUpdate.Year = 0x0;

  if (HAL_RTC_SetDate(&hrtc, &DateToUpdate, RTC_FORMAT_BCD) != HAL_OK) {
    Error_Handler();
  }
}

static void MX_UART4_Init(void) {
  huart4.Instance = UART4;
  huart4.Init.BaudRate = 115200;
  huart4.Init.WordLength = UART_WORDLENGTH_8B;
  huart4.Init.StopBits = UART_STOPBITS_1;
  huart4.Init.Parity = UART_PARITY_NONE;
  huart4.Init.Mode = UART_MODE_TX_RX;
  huart4.Init.HwFlowCtl = UART_HWCONTROL_NONE;
  huart4.Init.OverSampling = UART_OVERSAMPLING_16;
  if (HAL_UART_Init(&huart4) != HAL_OK) {
    Error_Handler();
  }
}

static void MX_USART1_UART_Init(void) {
  huart1.Instance = USART1;
  huart1.Init.BaudRate = 115200;
  huart1.Init.WordLength = UART_WORDLENGTH_8B;
  huart1.Init.StopBits = UART_STOPBITS_1;
  huart1.Init.Parity = UART_PARITY_NONE;
  huart1.Init.Mode = UART_MODE_TX_RX;
  huart1.Init.HwFlowCtl = UART_HWCONTROL_NONE;
  huart1.Init.OverSampling = UART_OVERSAMPLING_16;
  if (HAL_UART_Init(&huart1) != HAL_OK) {
    Error_Handler();
  }
}

static void MX_TIM3_Init(void) {
  TIM_MasterConfigTypeDef sMasterConfig = {0};
  TIM_OC_InitTypeDef sConfigOC = {0};

  htim3.Instance = TIM3;
  htim3.Init.Prescaler = 72 - 1;
  htim3.Init.CounterMode = TIM_COUNTERMODE_UP;
  htim3.Init.Period = 20000 - 1;
  htim3.Init.ClockDivision = TIM_CLOCKDIVISION_DIV1;
  htim3.Init.AutoReloadPreload = TIM_AUTORELOAD_PRELOAD_ENABLE;
  if (HAL_TIM_PWM_Init(&htim3) != HAL_OK) {
    Error_Handler();
  }
  sMasterConfig.MasterOutputTrigger = TIM_TRGO_RESET;
  sMasterConfig.MasterSlaveMode = TIM_MASTERSLAVEMODE_DISABLE;
  if (HAL_TIMEx_MasterConfigSynchronization(&htim3, &sMasterConfig) != HAL_OK) {
    Error_Handler();
  }
  sConfigOC.OCMode = TIM_OCMODE_PWM1;
  sConfigOC.Pulse = 1500;
  sConfigOC.OCPolarity = TIM_OCPOLARITY_HIGH;
  sConfigOC.OCFastMode = TIM_OCFAST_DISABLE;
  if (HAL_TIM_PWM_ConfigChannel(&htim3, &sConfigOC, TIM_CHANNEL_1) != HAL_OK) {
    Error_Handler();
  }
  sConfigOC.Pulse = 1500;
  if (HAL_TIM_PWM_ConfigChannel(&htim3, &sConfigOC, TIM_CHANNEL_2) != HAL_OK) {
    Error_Handler();
  }
  if (HAL_TIM_PWM_Start(&htim3, TIM_CHANNEL_1) != HAL_OK) {
    Error_Handler();
  }
  if (HAL_TIM_PWM_Start(&htim3, TIM_CHANNEL_2) != HAL_OK) {
    Error_Handler();
  }
}

static void MX_GPIO_Init(void) {
  GPIO_InitTypeDef GPIO_InitStruct = {0};

  __HAL_RCC_GPIOC_CLK_ENABLE();
  __HAL_RCC_GPIOD_CLK_ENABLE();
  __HAL_RCC_GPIOA_CLK_ENABLE();
  __HAL_RCC_GPIOB_CLK_ENABLE();
  __HAL_RCC_GPIOE_CLK_ENABLE();

  /* PA0: Fan */
  GPIO_InitStruct.Pin = GPIO_PIN_0;
  GPIO_InitStruct.Mode = GPIO_MODE_OUTPUT_PP;
  GPIO_InitStruct.Pull = GPIO_NOPULL;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
  HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);
  HAL_GPIO_WritePin(GPIOA, GPIO_PIN_0, GPIO_PIN_RESET);

  /* PA1: Lamp */
  GPIO_InitStruct.Pin = GPIO_PIN_1;
  GPIO_InitStruct.Mode = GPIO_MODE_OUTPUT_PP;
  GPIO_InitStruct.Pull = GPIO_NOPULL;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
  HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);
  HAL_GPIO_WritePin(GPIOA, GPIO_PIN_1, GPIO_PIN_RESET);

  /* PA2: Lock */
  GPIO_InitStruct.Pin = GPIO_PIN_2;
  GPIO_InitStruct.Mode = GPIO_MODE_OUTPUT_PP;
  GPIO_InitStruct.Pull = GPIO_NOPULL;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
  HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);
  HAL_GPIO_WritePin(GPIOA, GPIO_PIN_2, GPIO_PIN_RESET);

  /* Buttons */
  GPIO_InitStruct.Pin = GPIO_PIN_13;
  GPIO_InitStruct.Mode = GPIO_MODE_INPUT;
  GPIO_InitStruct.Pull = GPIO_PULLUP;
  HAL_GPIO_Init(GPIOC, &GPIO_InitStruct);
  HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);

  /* PA6: TIM3_CH1 (Servo 1 / Servo X) */
  GPIO_InitStruct.Pin = GPIO_PIN_6;
  GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
  GPIO_InitStruct.Pull = GPIO_NOPULL;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
  HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

  /* PA7: TIM3_CH2 (Servo 2 / Servo Y) */
  GPIO_InitStruct.Pin = GPIO_PIN_7;
  GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
  GPIO_InitStruct.Pull = GPIO_NOPULL;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
  HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

  /* KEY1 */
  GPIO_InitStruct.Pin = KEY1_Pin;
  GPIO_InitStruct.Mode = GPIO_MODE_INPUT;
  GPIO_InitStruct.Pull = GPIO_PULLUP;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
  HAL_GPIO_Init(KEY1_GPIO_Port, &GPIO_InitStruct);

  /* KEY2 */
  GPIO_InitStruct.Pin = KEY2_Pin;
  GPIO_InitStruct.Mode = GPIO_MODE_INPUT;
  GPIO_InitStruct.Pull = GPIO_PULLUP;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_LOW;
  HAL_GPIO_Init(KEY2_GPIO_Port, &GPIO_InitStruct);
}

void Error_Handler(void) {
  __disable_irq();
  while (1) {
    HAL_Delay(1000);
  }
}

#ifdef USE_FULL_ASSERT
void assert_failed(uint8_t *file, uint32_t line) {}
#endif
