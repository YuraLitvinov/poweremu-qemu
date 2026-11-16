/*
 * R300 texel conversion for renderers: shared dispatch, the 16bpp decoder,
 * and the plain copy for RGBA8/R8/RG8.  The endian-specific conversions are
 * in r300_texel_le.c and r300_texel_be.c.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "r300_texel.h"

#include <stdlib.h>
#include <string.h>

/* A 16bpp texel as the card sees it, decoded to RGBA8 bytes. */
void r300_decode16(uint32_t fmt, uint16_t v, uint8_t out[4])
{
    uint32_t x, y, z, w;

    switch (fmt) {
    case 0x6:   /* Z5Y6X5 */
        x = (v & 31) * 255 / 31; y = ((v >> 5) & 63) * 255 / 63;
        z = (v >> 11) * 255 / 31; w = 255;
        break;
    case 0x7:   /* Z6Y5X5 */
        x = (v & 31) * 255 / 31; y = ((v >> 5) & 31) * 255 / 31;
        z = (v >> 10) * 255 / 63; w = 255;
        break;
    case 0xA:   /* W4Z4Y4X4 */
        x = (v & 15) * 17; y = ((v >> 4) & 15) * 17;
        z = ((v >> 8) & 15) * 17; w = (v >> 12) * 17;
        break;
    case 0xB:   /* W1Z5Y5X5 */
        x = (v & 31) * 255 / 31; y = ((v >> 5) & 31) * 255 / 31;
        z = ((v >> 10) & 31) * 255 / 31; w = (v >> 15) ? 255 : 0;
        break;
    default:    /* X16 */
        x = v >> 8; y = z = 0; w = 255;
        break;
    }
    out[0] = x; out[1] = y; out[2] = z; out[3] = w;
}

uint8_t *r300_tex_level_bytes(const R300TexDesc *td, const uint8_t *src,
                                 uint32_t l, uint32_t w, uint32_t h,
                                 uint32_t *bpr)
{
    uint32_t pitch = td->lvl_pitch[l];
    uint8_t *out;

    switch (td->kind) {
    case R300_TEXK_CONVERT16:
    case R300_TEXK_DXT1:
    case R300_TEXK_DXT3:
    case R300_TEXK_DXT5:
        return GUEST_LE ? r300_level_le(td, src, l, w, h, bpr)
                      : r300_level_be(td, src, l, w, h, bpr);
    default: {                          /* RGBA8, R8, RG8: bytes as they lie */
        uint32_t bpp = td->kind == R300_TEXK_RGBA8 ? 4 : td->kind == R300_TEXK_RG8 ? 2 : 1;
        *bpr = w * bpp;
        out = malloc((size_t)w * h * bpp);
        for (uint32_t y = 0; y < h; y++) {
            memcpy(out + (size_t)y * w * bpp, src + (uint64_t)y * pitch, (size_t)w * bpp);
        }
        return out;
    }
    }
}
