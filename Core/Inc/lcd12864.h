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

void LCD_WriteEnglish(unsigned char x, unsigned char y, unsigned char c);
void LCD_WriteEnglishString(unsigned char x, unsigned char y, unsigned char *s);

void LCD_WriteChinese(unsigned char x, unsigned char y, unsigned char *img);
void LCD_WriteChineseString(unsigned char x, unsigned char y, unsigned char *img, unsigned char len);

/* Aliases for modern naming */
#define LCD_WriteChar(x, y, c)       LCD_WriteEnglish((x), (y), (unsigned char)(c))
#define LCD_WriteString(x, y, s)     LCD_WriteEnglishString((x), (y), (unsigned char *)(s))

#endif // _LCD12864_H_
