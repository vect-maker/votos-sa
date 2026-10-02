/* USER CODE BEGIN Header */
/**
  ******************************************************************************
  * @file           : main.h
  * @brief          : Header for main.c file.
  *                   This file contains the common defines of the application.
  ******************************************************************************
  * @attention
  *
  * Copyright (c) 2026 STMicroelectronics.
  * All rights reserved.
  *
  * This software is licensed under terms that can be found in the LICENSE file
  * in the root directory of this software component.
  * If no LICENSE file comes with this software, it is provided AS-IS.
  *
  ******************************************************************************
  */
/* USER CODE END Header */

/* Define to prevent recursive inclusion -------------------------------------*/
#ifndef __MAIN_H
#define __MAIN_H

#ifdef __cplusplus
extern "C" {
#endif

/* Includes ------------------------------------------------------------------*/
#include "stm32f1xx_hal.h"

/* Private includes ----------------------------------------------------------*/
/* USER CODE BEGIN Includes */

/* USER CODE END Includes */

/* Exported types ------------------------------------------------------------*/
/* USER CODE BEGIN ET */

/* USER CODE END ET */

/* Exported constants --------------------------------------------------------*/
/* USER CODE BEGIN EC */

/* USER CODE END EC */

/* Exported macro ------------------------------------------------------------*/
/* USER CODE BEGIN EM */

/* USER CODE END EM */

void HAL_TIM_MspPostInit(TIM_HandleTypeDef *htim);

/* Exported functions prototypes ---------------------------------------------*/
void Error_Handler(void);

/* USER CODE BEGIN EFP */

/* USER CODE END EFP */

/* Private defines -----------------------------------------------------------*/
#define RL1_Pin GPIO_PIN_0
#define RL1_GPIO_Port GPIOA
#define RL2_Pin GPIO_PIN_1
#define RL2_GPIO_Port GPIOA
#define RL3_Pin GPIO_PIN_2
#define RL3_GPIO_Port GPIOA
#define PCF8591_SCL_Pin GPIO_PIN_4
#define PCF8591_SCL_GPIO_Port GPIOA
#define PCF8591_SDA_Pin GPIO_PIN_5
#define PCF8591_SDA_GPIO_Port GPIOA
#define HC595_RCK_Pin GPIO_PIN_4
#define HC595_RCK_GPIO_Port GPIOC
#define HC595_SCK_Pin GPIO_PIN_5
#define HC595_SCK_GPIO_Port GPIOC
#define HC595_DAT_Pin GPIO_PIN_0
#define HC595_DAT_GPIO_Port GPIOB
#define BUZZER_Pin GPIO_PIN_8
#define BUZZER_GPIO_Port GPIOA

#define KEY1_Pin GPIO_PIN_13
#define KEY1_GPIO_Port GPIOC
#define KEY2_Pin GPIO_PIN_13
#define KEY2_GPIO_Port GPIOD
/* USER CODE BEGIN Private defines */

/* USER CODE END Private defines */

#ifdef __cplusplus
}
#endif

#endif /* __MAIN_H */
