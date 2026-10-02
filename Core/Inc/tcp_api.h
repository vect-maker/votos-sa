#ifndef TCP_API
#define TCP_API

int build_connection_request(char* out_buf, size_t buf_size,
                              const char* device_tag, 
                              const char* secret_key);
#endif