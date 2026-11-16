/*
 * R300 texel conversion for big-endian (PowerPC) guests.  The texel words
 * are the guest CPU's big-endian view, so the TX_OFFSET dword swap is applied.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "r300_texel.h"

#include <stdlib.h>
#include <string.h>

/*
 * A big-endian 16bpp texel at halfword x of a row, with the TX_OFFSET dword
 * swap (mode 1..3) applied to its 4-byte group.  Mode 0 gives the plain
 * big-endian halfword.
 */
static uint16_t r300_tex16_endian(const uint8_t *row, uint32_t x, unsigned mode)
{
    uint32_t g = 4 * (x / 2);
    uint32_t v = (uint32_t)row[g] << 24 | (uint32_t)row[g + 1] << 16 |
                 (uint32_t)row[g + 2] << 8 | row[g + 3];

    switch (mode) {
    case 1: v = ((v & 0x00ff00ffu) << 8) | ((v >> 8) & 0x00ff00ffu); break;
    case 2: v = __builtin_bswap32(v); break;
    case 3: v = (v << 16) | (v >> 16); break;
    default: break;
    }
    return (x & 1) ? (uint16_t)v : (uint16_t)(v >> 16);
}

uint8_t *r300_level_be(const R300TexDesc *td, const uint8_t *src,
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
                uint16_t v = (uint16_t)(row[2 * x] << 8 | row[2 * x + 1]);
                /* Only when the whole 4-byte group lies inside the row. */
                if ((td->endian & 3) && 4 * (x / 2) + 4 <= 2 * w) {
                    v = r300_tex16_endian(row, x, td->endian & 3);
                }
                r300_decode16(td->format, v, out + ((size_t)y * w + x) * 4);
            }
        }
        return out;
    default: {                          /* DXT1/3/5 */
        uint32_t bs = td->kind == R300_TEXK_DXT1 ? 8 : 16;
        uint32_t bw = (w + 3) / 4, bh = (h + 3) / 4;
        *bpr = bw * bs;
        out = malloc((size_t)bw * bh * bs);
        for (uint32_t y = 0; y < bh; y++) {
            memcpy(out + (size_t)y * bw * bs, src + (uint64_t)y * pitch, (size_t)bw * bs);
        }
        /*
         * Blocks are little-endian words to the card.  VRAM holds the
         * big-endian CPU's view (GART copies the other side of the
         * aperture), and the card's dword is the TX_OFFSET endian swap of
         * that: with no swap, every word is byte-reversed.
         */
        for (size_t i = 0; i < (size_t)bw * bh * bs; i += 4) {
            uint32_t lw, v;
            memcpy(&lw, out + i, 4);
            if (td->host_data) {
                lw = __builtin_bswap32(lw);
            }
            v = __builtin_bswap32(lw);                      /* the BE value */
            switch (td->endian & 3) {
            case 1: v = ((v & 0x00ff00ffu) << 8) | ((v >> 8) & 0x00ff00ffu); break;
            case 2: v = __builtin_bswap32(v); break;
            case 3: v = (v << 16) | (v >> 16); break;
            default: break;
            }
            memcpy(out + i, &v, 4);
        }
        return out;
    }
    }
}
