/* Golden vectors for the texel conversion, produced by the C original
 * (r300_texel.c + r300_texel_le.c).  Output lines:
 *   D fmt fnv            FNV-1a of r300_decode16 over all 65536 values
 *   S fmt v r g b a      one decoded sample
 *   L kind fmt w h pitch bpr outlen fnv   one converted mip level; the source
 *                        is rows*pitch bytes from the LCG below, seed idx+1. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_texel.h"

static uint64_t fnv(const uint8_t *p, size_t n)
{
    uint64_t h = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < n; i++) { h ^= p[i]; h *= 0x100000001b3ull; }
    return h;
}
static uint8_t lcg(uint32_t *s) { *s = *s * 1664525u + 1013904223u; return (uint8_t)(*s >> 24); }

int main(void)
{
    static const uint32_t fmts[] = { 0x6, 0x7, 0xA, 0xB, 0x0, 0x1, 0x1F };
    static const uint16_t samples[] = { 0, 1, 0x1f, 0x20, 0x7ff, 0x8000, 0x1234, 0xabcd, 0xfedc, 0xffff };
    for (unsigned f = 0; f < sizeof fmts / sizeof *fmts; f++) {
        static uint8_t all[65536 * 4];
        for (uint32_t v = 0; v < 65536; v++) r300_decode16(fmts[f], (uint16_t)v, all + v * 4);
        printf("D %u %016llx\n", fmts[f], (unsigned long long)fnv(all, sizeof all));
        for (unsigned s = 0; s < sizeof samples / sizeof *samples; s++) {
            uint8_t o[4];
            r300_decode16(fmts[f], samples[s], o);
            printf("S %u %u %u %u %u %u\n", fmts[f], samples[s], o[0], o[1], o[2], o[3]);
        }
    }
    static const struct { uint32_t kind, fmt, w, h, pitch; } cases[] = {
        { R300_TEXK_CONVERT16, 0x6, 7, 5, 16 },   { R300_TEXK_CONVERT16, 0x7, 16, 16, 32 },
        { R300_TEXK_CONVERT16, 0xA, 1, 1, 32 },   { R300_TEXK_CONVERT16, 0xB, 33, 3, 96 },
        { R300_TEXK_CONVERT16, 0x0, 9, 9, 18 },   { R300_TEXK_CONVERT16, 0x1, 64, 2, 128 },
        { R300_TEXK_DXT1, 0xF, 4, 4, 32 },        { R300_TEXK_DXT1, 0xF, 5, 7, 32 },
        { R300_TEXK_DXT1, 0xF, 1, 1, 32 },        { R300_TEXK_DXT3, 0x10, 8, 8, 64 },
        { R300_TEXK_DXT3, 0x10, 13, 6, 64 },      { R300_TEXK_DXT5, 0x11, 16, 4, 64 },
        { R300_TEXK_DXT5, 0x11, 3, 9, 32 },       { R300_TEXK_RGBA8, 0x0, 5, 4, 32 },
        { R300_TEXK_RGBA8, 0x0, 16, 16, 64 },     { R300_TEXK_R8, 0x0, 7, 3, 32 },
        { R300_TEXK_RG8, 0x0, 6, 6, 32 },         { R300_TEXK_RG8, 0x0, 1, 1, 32 },
        { R300_TEXK_RAW, 0x0, 10, 2, 32 },
    };
    for (unsigned i = 0; i < sizeof cases / sizeof *cases; i++) {
        R300TexDesc td;
        memset(&td, 0, sizeof td);
        td.kind = cases[i].kind; td.format = cases[i].fmt; td.lvl_pitch[0] = cases[i].pitch;
        uint32_t w = cases[i].w, h = cases[i].h;
        bool dxt = td.kind >= R300_TEXK_DXT1;
        size_t rows = dxt ? (h + 3) / 4 : h;
        size_t n = rows * cases[i].pitch;
        uint8_t *src = malloc(n);
        uint32_t seed = i + 1;
        for (size_t k = 0; k < n; k++) src[k] = lcg(&seed);
        uint32_t bpr = 0;
        uint8_t *out = r300_tex_level_bytes(&td, src, 0, w, h, &bpr);
        size_t outlen = (size_t)bpr * rows;
        printf("L %u %u %u %u %u %u %zu %016llx\n", cases[i].kind, cases[i].fmt, w, h,
               cases[i].pitch, bpr, outlen, (unsigned long long)fnv(out, outlen));
        free(out); free(src);
    }
    return 0;
}
