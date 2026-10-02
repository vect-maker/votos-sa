#ifndef TCP_API_H
#define TCP_API_H

#include <stddef.h>
#include <stdint.h>

int build_connection_request(char *out_buf, size_t buf_size,
                             const char *device_tag, const char *secret_key);

int build_data_upload_request(char *out_buf, size_t buf_size,
                              const char *tag, int32_t value);

int build_cmd_response(char *out_buf, size_t buf_size,
                       int32_t cmdid, uint8_t status, int32_t data);

#endif /* TCP_API_H */