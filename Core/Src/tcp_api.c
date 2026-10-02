#include "connection.h"
#include <stdio.h>

int build_connection_request(char* out_buf, size_t buf_size,
                              const char* device_tag,
                              const char* secret_key) {
    if (out_buf == NULL || buf_size == 0) return -1;
    
    int n = snprintf(out_buf, buf_size,
        "{\"t\":1,\"device\":\"%s\",\"key\":\"%s\",\"ver\":\"v1.1\"}",
        device_tag, secret_key);
    
    return (n < 0 || (size_t)n >= buf_size) ? -1 : n;
}