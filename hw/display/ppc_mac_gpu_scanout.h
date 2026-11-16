/*
 * ppc_mac_gpu scanout: the byte-order-specific pieces of the display path.
 * The little-endian (x86 guest) and big-endian (PowerPC guest) versions are
 * in ppc_mac_gpu_scanout_le.c and ppc_mac_gpu_scanout_be.c; the shared code
 * in ppc_mac_gpu.c picks one from s->swap.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#ifndef PPC_MAC_GPU_SCANOUT_H
#define PPC_MAC_GPU_SCANOUT_H

#include <stdbool.h>
#include <stdint.h>
#include "ui/qemu-pixman.h"

/* Byte-order independent: one 16bpp pixel (already read as a host value)
 * to 32bpp ARGB with alpha 0xFF. */
static inline uint32_t ppc_mac_gpu_px16_argb(uint32_t v, bool is565)
{
    uint32_t r, g, b;

    if (is565) {
        r = (v >> 11) & 31; g = (v >> 5) & 63; b = v & 31;
        r = (r << 3) | (r >> 2); g = (g << 2) | (g >> 4);
    } else {
        r = (v >> 10) & 31; g = (v >> 5) & 31; b = v & 31;
        r = (r << 3) | (r >> 2); g = (g << 3) | (g >> 2);
    }
    b = (b << 3) | (b >> 2);
    return 0xFF000000u | (r << 16) | (g << 8) | b;
}

/* Little-endian guest (x86): VRAM rows are little-endian halfwords. */
void ppc_mac_gpu_scan16_le(const uint8_t *sp, uint32_t *dp, uint32_t width, bool is565);
/* Big-endian guest (PowerPC): VRAM rows are big-endian halfwords. */
void ppc_mac_gpu_scan16_be(const uint8_t *sp, uint32_t *dp, uint32_t width, bool is565);

/* Copy one 32bpp scanline into the shadow buffer's native order. */
void ppc_mac_gpu_line32_le(uint32_t *dst, const uint32_t *src, int width);
void ppc_mac_gpu_line32_be(uint32_t *dst, const uint32_t *src, int width);

/* Pixman formats that describe the VRAM byte order to the display code. */
pixman_format_code_t ppc_mac_gpu_fmt32_le(void);
pixman_format_code_t ppc_mac_gpu_fmt32_be(void);
pixman_format_code_t ppc_mac_gpu_fmt24_le(void);
pixman_format_code_t ppc_mac_gpu_fmt24_be(void);

#endif
