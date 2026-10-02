#include "tcp_api.h"
#include <stdio.h>

static uint16_t g_tcp_msgid = 1;

int build_connection_request(char *out_buf, size_t buf_size,
                             const char *device_tag, const char *secret_key) {
  if (out_buf == NULL || buf_size == 0)
    return -1;

  int n = snprintf(out_buf, buf_size,
                   "{\"t\":1,\"device\":\"%s\",\"key\":\"%s\",\"ver\":\"v1.1\"}",
                   device_tag, secret_key);

  return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}

int build_data_upload_request(char *out_buf, size_t buf_size,
                              const char *tag, int32_t value) {
  if (out_buf == NULL || buf_size == 0)
    return -1;

  int n = snprintf(out_buf, buf_size,
                   "{\"t\":3,\"datatype\":1,\"datas\":{\"%s\":%d},\"msgid\":%u}",
                   tag, (int)value, (unsigned int)g_tcp_msgid++);
  if (g_tcp_msgid > 65000)
    g_tcp_msgid = 1;

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