#ifndef _LCD12864_H_
#define _LCD12864_H_

void LCD_WrCmd(unsigned char cmd);
void LCD_Init(void);
void LCD_Clr(void);
void LCD_Full(void);
void LCD_DispOneRow(unsigned char x, unsigned char y, unsigned char *buf, unsigned char len);
void DrawDot_12864(unsigned char y, unsigned char x, unsigned char type);

void LCD_DispFullImg(unsigned char *img);
void LCD_DispImg(unsigned char x, unsigned char y, unsigned char wid, unsigned char lon, unsigned char *img);

void LCD_WriteChar(unsigned char x, unsigned char y, char c);
void LCD_WriteString(unsigned char x, unsigned char y, const char *s);

/* Aliases for backwards compatibility */
#define LCD_WriteEnglish(x, y, c)       LCD_WriteChar(x, y, c)
#define LCD_WriteEnglishString(x, y, s) LCD_WriteString(x, y, (const char *)(s))

#endif // _LCD12864_H_
