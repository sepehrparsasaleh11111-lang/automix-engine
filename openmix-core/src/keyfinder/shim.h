#ifndef OPENMIX_KF_SHIM_H
#define OPENMIX_KF_SHIM_H
#include <cstddef>
#ifdef __cplusplus
extern "C" {
#endif
int openmix_kf_detect(const float* samples, size_t n, unsigned rate,
                      int* key_out, float* conf_out);
#ifdef __cplusplus
}
#endif
#endif