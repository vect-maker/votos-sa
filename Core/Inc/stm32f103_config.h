#ifndef STM32F103_CONFIG_H
#define STM32F103_CONFIG_H

#include "main.h"

/* Bit-banding macros for STM32F1 */
#define BITBAND(addr, bitnum) ((addr & 0xF0000000)+0x02000000+((addr & 0xFFFFF)<<5)+(bitnum<<2)) 
#define MEM_ADDR(addr)  *((volatile unsigned long  *)(addr)) 
#define BIT_ADDR(addr, bitnum)   MEM_ADDR(BITBAND(addr, bitnum)) 

#define GPIOD_ODR_Addr    (GPIOD_BASE+12)
#define GPIOE_ODR_Addr    (GPIOE_BASE+12)
#define GPIOD_IDR_Addr    (GPIOD_BASE+8)
#define GPIOE_IDR_Addr    (GPIOE_BASE+8)

#define PDout(n)   BIT_ADDR(GPIOD_ODR_Addr,n)
#define PDin(n)    BIT_ADDR(GPIOD_IDR_Addr,n)
#define PEout(n)   BIT_ADDR(GPIOE_ODR_Addr,n)
#define PEin(n)    BIT_ADDR(GPIOE_IDR_Addr,n)

/* LCD Pin Mapping */
#define LCD12864_BL      PDout(14)
#define LCD12864_RST     PDout(0)
#define LCD12864_CS2     PEout(7)
#define LCD12864_CS1     PEout(9)
#define LCD12864_EN      PEout(12)
#define LCD12864_RW      PEout(14)
#define LCD12864_RS      PDout(8)

#define LCD12864_DB7     PEout(11)
#define LCD12864_DB6     PEout(13)
#define LCD12864_DB5     PEout(15)
#define LCD12864_DB4     PDout(9)
#define LCD12864_DB3     PDout(15)
#define LCD12864_DB2     PDout(1)
#define LCD12864_DB1     PEout(8)
#define LCD12864_DB0     PEout(10)

#define LCD12864_DB7_IN  PEin(11)
#define LCD12864_DB6_IN  PEin(13)
#define LCD12864_DB5_IN  PEin(15)
#define LCD12864_DB4_IN  PDin(9)
#define LCD12864_DB3_IN  PDin(15)
#define LCD12864_DB2_IN  PDin(1)
#define LCD12864_DB1_IN  PEin(8)
#define LCD12864_DB0_IN  PEin(10)

/* GPIO Abstraction */
static inline void LCD_SetInputs(void) {
    GPIO_InitTypeDef GPIO_InitStruct = {0};
    GPIO_InitStruct.Mode = GPIO_MODE_INPUT;
    GPIO_InitStruct.Pull = GPIO_NOPULL;
    GPIO_InitStruct.Pin = GPIO_PIN_1 | GPIO_PIN_9 | GPIO_PIN_15;
    HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);
    GPIO_InitStruct.Pin = GPIO_PIN_8 | GPIO_PIN_10 | GPIO_PIN_11 | GPIO_PIN_13 | GPIO_PIN_15;
    HAL_GPIO_Init(GPIOE, &GPIO_InitStruct);
}

static inline void LCD_SetOutputs(void) {
    GPIO_InitTypeDef GPIO_InitStruct = {0};
    GPIO_InitStruct.Mode = GPIO_MODE_OUTPUT_PP;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
    GPIO_InitStruct.Pin = GPIO_PIN_1 | GPIO_PIN_9 | GPIO_PIN_15;
    HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);
    GPIO_InitStruct.Pin = GPIO_PIN_8 | GPIO_PIN_10 | GPIO_PIN_11 | GPIO_PIN_13 | GPIO_PIN_15;
    HAL_GPIO_Init(GPIOE, &GPIO_InitStruct);
}

static inline void M3_LCD_GpioInit(void) {
    __HAL_RCC_GPIOD_CLK_ENABLE();
    __HAL_RCC_GPIOE_CLK_ENABLE();
    GPIO_InitTypeDef GPIO_InitStruct = {0};
    GPIO_InitStruct.Mode = GPIO_MODE_OUTPUT_PP;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_HIGH;
    /* Control pins */
    GPIO_InitStruct.Pin = GPIO_PIN_0 | GPIO_PIN_8 | GPIO_PIN_14; /* RST, RS, BL */
    HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);
    GPIO_InitStruct.Pin = GPIO_PIN_7 | GPIO_PIN_9 | GPIO_PIN_12 | GPIO_PIN_14; /* CS2, CS1, EN, RW */
    HAL_GPIO_Init(GPIOE, &GPIO_InitStruct);
    /* Set Data pins as outputs initially */
    LCD_SetOutputs();
}

#define STM32_GPIOx_Init(x) x

#define LCD12864_DB7_IN_Init LCD_SetInputs()
#define LCD12864_DB6_IN_Init 
#define LCD12864_DB5_IN_Init 
#define LCD12864_DB4_IN_Init 
#define LCD12864_DB3_IN_Init 
#define LCD12864_DB2_IN_Init 
#define LCD12864_DB1_IN_Init 
#define LCD12864_DB0_IN_Init 

#define LCD12864_DB7_Init LCD_SetOutputs()
#define LCD12864_DB6_Init 
#define LCD12864_DB5_Init 
#define LCD12864_DB4_Init 
#define LCD12864_DB3_Init 
#define LCD12864_DB2_Init 
#define LCD12864_DB1_Init 
#define LCD12864_DB0_Init 

#endif
