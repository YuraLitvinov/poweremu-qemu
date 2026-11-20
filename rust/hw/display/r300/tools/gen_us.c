/* Golden vectors for the fragment-shader translator, from the C original
 * (r300_us.c).  Output lines:
 *   B fmt bpp                      r300_tex_raw_bpp
 *   V cf of view bpp               r300_cb_view
 *   F bits float24-bits            r300_float24
 *   W idx addr val ...             the register writes that build case idx
 *   D idx vsflag route[32]         the R300FSDesc of case idx
 *   N idx ntargets of0..of3 dlen dhash   num_targets, out_fmt, disassembly
 *   U idx flags len hash           r300_us_to_glsl output (flags 0..3)
 *   E idx flags message            ... or the reason it refused
 *   @@ idx flags len + text        a few full texts */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_us.h"

static uint32_t rs = 777;
static uint32_t rnd(void) { rs = rs * 1664525u + 1013904223u; return rs; }
static uint32_t pick(uint32_t n) { return (rnd() >> 8) % n; }
static uint32_t rnd32(void) { return (rnd() & 0xFFFF0000u) | (rnd() >> 16); }
static uint64_t fnv(const char *p, size_t n)
{
    uint64_t h = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < n; i++) { h ^= (uint8_t)p[i]; h *= 0x100000001b3ull; }
    return h;
}

static R300State st;
static uint32_t wr[1024][2];
static int nwr;
static void W(uint32_t a, uint32_t v) { r300_state_write(&st, a, v); wr[nwr][0] = a; wr[nwr][1] = v; nwr++; }

int main(void)
{
    for (uint32_t f = 0; f < 32; f++) printf("B %u %u\n", f, r300_tex_raw_bpp(f));
    for (uint32_t cf = 0; cf < 16; cf++)
        for (uint32_t of = 0; of < 32; of++) { uint32_t bpp; uint32_t v = r300_cb_view(cf, of, &bpp); printf("V %u %u %u %u\n", cf, of, v, bpp); }
    for (int i = 0; i < 400; i++) {
        uint32_t v = i < 8 ? (uint32_t[]){0, 1, 0xFFFF, 0x10000, 0x7F0000, 0x800000, 0xFFFFFF, 0x3F0000}[i] : rnd() & 0xFFFFFF;
        float f = r300_float24(v); uint32_t b; memcpy(&b, &f, 4);
        printf("F %06x %08x\n", v, b);
    }
    unsigned samples = 0;
    for (int idx = 0; idx < 160; idx++) {
        r300_state_reset(&st); nwr = 0;
        unsigned nn = 1 + pick(4);
        int bad = pick(12) == 0;       /* a program that runs off instruction memory */
        uint32_t config = (nn - 1) | (pick(2) << 3);
        W(0x4600, config);
        W(0x4608, pick(4) == 0 ? (pick(8) | (pick(6) << 13)) : 0);
        unsigned alu = 0, tex = 0;
        for (unsigned i = 0; i < nn; i++) {
            unsigned ac = 1 + pick(5), tc = 1 + pick(4);
            if (bad && i == 0) ac = 70;
            uint32_t a = (alu & 0x3F) | ((ac - 1) << 6) | ((tex & 0x1F) << 12) | ((tc - 1) << 17);
            if (bad && i == 0) a = (alu & 0x3F) | (63u << 6) | ((tex & 0x1F) << 12) | ((tc - 1) << 17);
            W(0x4610 + 4 * (4 - nn + i), a);
            alu += ac; tex += tc;
        }
        for (unsigned t = 0; t < 32; t++) {
            uint32_t op = pick(24) == 0 ? 5 + pick(3) : pick(5);
            W(0x4620 + 4 * t, pick(32) | (pick(32) << 6) | (pick(16) << 11) | (op << 15));
        }
        for (unsigned a = 0; a < 64; a++) {
            W(0x46C0 + 4 * a, rnd32());
            W(0x47C0 + 4 * a, rnd32());
            W(0x48C0 + 4 * a, rnd32());
            W(0x49C0 + 4 * a, rnd32());
        }
        W(0x4104, rnd32() & 0xFFFF);
        for (unsigned u = 0; u < 16; u++) {
            uint32_t fmt = pick(32);
            uint32_t f1 = fmt | (pick(2) << 22) | (pick(4) << 25) | (pick(16) << 5);
            W(0x44C0 + 4 * u, f1);
            W(0x4400 + 4 * u, rnd32());
            W(0x4540 + 4 * u, rnd32());
        }
        static const uint32_t cfs[] = { 3, 4, 6, 6, 6, 7, 9, 10, 13, 15, 0, 5, 12 };
        for (unsigned k = 0; k < 4; k++) {
            W(0x4E38 + 4 * k, (cfs[pick(13)] << 21) | (pick(4) << 19) | pick(1 << 10));
            uint32_t of = pick(24) == 0 ? 15 : pick(24);
            W(0x46A4 + 4 * k, of | (pick(1 << 12) << 8) | (pick(16) << 16));
        }
        W(0x4E00, pick(4) << 5 | pick(4));
        W(0x221C, rnd32() & 0xFF);
        uint32_t flagsets[] = { 0, 1, 2, 3 };
        printf("W %d %d", idx, nwr);
        for (int i = 0; i < nwr; i++) printf(" %x %x", wr[i][0], wr[i][1]);
        printf("\n");
        R300FSDesc d; memset(&d, 0, sizeof d);
        int vsflag = pick(3) == 0;
        for (int k = 0; k < 32; k++) d.route[k] = pick(5) == 0 ? -1 : (int8_t)pick(10);
        if (pick(4) == 0) d.route[0] = -1;
        static const char vs[] = "void main() { gl_Position = vec4(0.0); }\n";
        d.vs_gpu = vsflag ? vs : NULL;
        printf("D %d %d", idx, vsflag);
        for (int k = 0; k < 32; k++) printf(" %d", d.route[k]);
        printf("\n");
        char *dis = r300_us_disasm(&st);
        printf("N %d %u %08x %08x %08x %08x %zu %016llx\n", idx, r300_us_num_targets(&st),
               r300_us_out_fmt(&st, 0), r300_us_out_fmt(&st, 1), r300_us_out_fmt(&st, 2), r300_us_out_fmt(&st, 3),
               strlen(dis), (unsigned long long)fnv(dis, strlen(dis)));
        free(dis);
        for (int fi = 0; fi < 4; fi++) {
            const char *err = NULL;
            char *g = r300_us_to_glsl(&st, &d, flagsets[fi], &err);
            if (g) {
                printf("U %d %u %zu %016llx\n", idx, flagsets[fi], strlen(g), (unsigned long long)fnv(g, strlen(g)));
                if (samples < 5 && fi == samples % 4) { printf("@@ %d %u %zu\n%s\n", idx, flagsets[fi], strlen(g), g); samples++; }
            } else {
                printf("E %d %u %s\n", idx, flagsets[fi], err);
            }
            free(g);
        }
    }
    return 0;
}
