/*
 * ppc_mac_gpu scanout for big-endian (PowerPC) guests.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "qemu/osdep.h"
#include "ppc_mac_gpu_byteorder.h"
#include "ppc_mac_gpu_scanout.h"

#if PPC_MAC_GPU_BE
void ppc_mac_gpu_scan16_be(const uint8_t *sp, uint32_t *dp, uint32_t width, bool is565)
{
    for (uint32_t x = 0; x < width; x++) {
        uint32_t v = (uint32_t)((sp[2 * x] << 8) | sp[2 * x + 1]);
        dp[x] = ppc_mac_gpu_px16_argb(v, is565);
    }
}

void ppc_mac_gpu_line32_be(uint32_t *dst, const uint32_t *src, int width)
{
    /* A big-endian guest writes XRGB as [X, R, G, B]: swap to host order. */
    for (int i = 0; i < width; i++) {
        dst[i] = __builtin_bswap32(src[i]);
    }
}

pixman_format_code_t ppc_mac_gpu_fmt32_be(void)
{
    return PIXMAN_BE_x8r8g8b8;
}

pixman_format_code_t ppc_mac_gpu_fmt24_be(void)
{
    return PIXMAN_BE_r8g8b8;
}

#endif
