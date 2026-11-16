/*
 * R300 texel conversion for little-endian (x86) guests.  The texel words are
 * the card's own, so no byte swap is applied.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "r300_texel.h"

#include <stdlib.h>
#include <string.h>

uint8_t *r300_level_le(const R300TexDesc *td, const uint8_t *src,
                       uint32_t l, uint32_t w, uint32_t h, uint32_t *bpr)
{
    uint32_t pitch = td->lvl_pitch[l];
    uint8_t *out;

    switch (td->kind) {
    case R300_TEXK_CONVERT16:
        *bpr = w * 4;
        out = malloc((size_t)w * h * 4);
        for (uint32_t y = 0; y < h; y++) {
            const uint8_t *row = src + (uint64_t)y * pitch;
            for (uint32_t x = 0; x < w; x++) {
                uint16_t v = (uint16_t)(row[2 * x] | row[2 * x + 1] << 8);
                r300_decode16(td->format, v, out + ((size_t)y * w + x) * 4);
            }
        }
        return out;
    default: {                          /* DXT1/3/5: the copy is the result */
        uint32_t bs = td->kind == R300_TEXK_DXT1 ? 8 : 16;
        uint32_t bw = (w + 3) / 4, bh = (h + 3) / 4;
        *bpr = bw * bs;
        out = malloc((size_t)bw * bh * bs);
        for (uint32_t y = 0; y < bh; y++) {
            memcpy(out + (size_t)y * bw * bs, src + (uint64_t)y * pitch, (size_t)bw * bs);
        }
        return out;
    }
    }
}
