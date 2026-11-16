/*
 * ppc_mac_gpu scanout for little-endian (x86) guests.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "qemu/osdep.h"
#include "ppc_mac_gpu_byteorder.h"
#include "ppc_mac_gpu_scanout.h"

#if !PPC_MAC_GPU_BE
#include <string.h>

void ppc_mac_gpu_scan16_le(const uint8_t *sp, uint32_t *dp, uint32_t width, bool is565)
{
    for (uint32_t x = 0; x < width; x++) {
        uint32_t v = (uint32_t)(sp[2 * x] | (sp[2 * x + 1] << 8));
        dp[x] = ppc_mac_gpu_px16_argb(v, is565);
    }
}

void ppc_mac_gpu_line32_le(uint32_t *dst, const uint32_t *src, int width)
{
    /* A little-endian (x86) guest writes XRGB as [B, G, R, X] already. */
    memcpy(dst, src, (size_t)width * 4);
}

pixman_format_code_t ppc_mac_gpu_fmt32_le(void)
{
    return PIXMAN_x8r8g8b8;
}

pixman_format_code_t ppc_mac_gpu_fmt24_le(void)
{
    return PIXMAN_r8g8b8;
}

#endif
