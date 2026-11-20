/* Golden vectors for the self-contained helpers of r300_draw.c.  Lines:
 *   Y dim w h d levels tile pitch_bytes bpp dxt pitch_en | size off[12] pitch[12] rows[12]
 *   D dim w h d l | w h d                     r300_tex_level_dims
 *   C fmt v | bits x4                         r300_border_color
 *   A prim n cls nidx hash                    r300_assemble_prov (indices, provenance)
 *   Q cfg0 cfg1 mspos0 mspos1 | samples pos[6][2] bits
 *   M x y ns pitch bpp sample off             r300_msaa_offset
 *   X ndw i32 | indices...                    r300_index_at over a stream; "X" then values for i < 2n
 *   H n | hash_bytes hash_tex                 content hashes of the LCG-filled buffer of n bytes
 *   L ndw words... | count (addr size stride)* r300_load_vbpntr */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_draw.h"

static uint32_t rs = 4242;
static uint32_t rnd(void) { rs = rs * 1664525u + 1013904223u; return rs; }
static uint32_t pick(uint32_t n) { return (rnd() >> 8) % n; }
static uint64_t fnv(uint64_t h, const void *p, size_t n)
{
    const uint8_t *b = p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 0x100000001b3ull; }
    return h;
}

int main(void)
{
    static R300State st;
    for (int i = 0; i < 300; i++) {
        R300TexDesc td; memset(&td, 0, sizeof td);
        td.dim = pick(3);
        td.width = 1 + pick(i % 5 == 0 ? 2048 : 70);
        td.height = 1 + pick(i % 7 == 0 ? 2048 : 70);
        td.depth = td.dim == R300_TEXDIM_3D ? 1 + pick(20) : 1;
        td.levels = 1 + pick(12);
        td.tile = pick(32);
        td.pitch_bytes = 32 * (1 + pick(200));
        uint32_t bpp = (uint32_t[]){ 1, 2, 4, 8, 16 }[pick(5)];
        bool dxt = pick(4) == 0; if (dxt) bpp = pick(2) ? 8 : 16;
        bool pe = pick(2);
        r300_tex_layout(&td, bpp, dxt, pe);
        printf("Y %u %u %u %u %u %u %u %u %d %d |", td.dim, td.width, td.height, td.depth, td.levels, td.tile, td.pitch_bytes, bpp, dxt, pe);
        printf(" %u", td.size_bytes);
        for (int l = 0; l < 12; l++) printf(" %u", td.lvl_off[l]);
        for (int l = 0; l < 12; l++) printf(" %u", td.lvl_pitch[l]);
        for (int l = 0; l < 12; l++) printf(" %u", td.lvl_rows[l]);
        printf("\n");
        for (uint32_t l = 0; l < 12; l++) {
            uint32_t w, h, d; r300_tex_level_dims(&td, l, &w, &h, &d);
            printf("D %u %u %u %u %u | %u %u %u\n", td.dim, td.width, td.height, td.depth, l, w, h, d);
        }
    }
    static const uint32_t vals[] = { 0, 1, 0xFFFFFFFF, 0x80808080, 0x12345678, 0xDEADBEEF, 0x3C003C00, 0x7C00FC00, 0x7E017E01, 0x00010203, 0x7FFF8000, 0x3F800000 };
    for (uint32_t fmt = 0; fmt < 32; fmt++)
        for (unsigned k = 0; k < sizeof vals / sizeof *vals; k++) {
            float c[4]; uint32_t b[4];
            r300_border_color(fmt, vals[k], c); memcpy(b, c, 16);
            printf("C %u %u | %08x %08x %08x %08x\n", fmt, vals[k], b[0], b[1], b[2], b[3]);
        }
    for (uint32_t prim = 0; prim < 18; prim++)
        for (uint32_t n = 0; n < 14; n++) {
            uint32_t list[3 * 14 + 6], cls = 99; R300Prov prov[2 * 14 + 2];
            memset(prov, 0xAA, sizeof prov); memset(list, 0xAA, sizeof list);
            uint32_t m = r300_assemble_prov(prim, n, list, &cls, prov);
            uint64_t h = fnv(0xcbf29ce484222325ull, list, m * 4);
            uint32_t np = prim == 1 ? m : (cls == 1 ? m / 2 : m / 3);
            if (m == 0) np = 0;
            h = fnv(h, prov, np * sizeof(R300Prov));
            uint32_t m2 = r300_assemble(prim, n, list, &cls);
            printf("A %u %u %u %u %u %016llx\n", prim, n, m == 0 ? 99 : cls, m, m2, (unsigned long long)h);
        }
    for (int i = 0; i < 40; i++) {
        r300_state_reset(&st);
        uint32_t c = rnd(), a = rnd(), b = rnd();
        r300_state_write(&st, 0x4020, c); r300_state_write(&st, 0x4010, a); r300_state_write(&st, 0x4014, b);
        float pos[6][2]; r300_aa_positions(&st, pos); uint32_t bits[12]; memcpy(bits, pos, 48);
        printf("Q %08x %08x %08x | %u", c, a, b, r300_aa_samples(&st));
        for (int k = 0; k < 12; k++) printf(" %08x", bits[k]);
        printf("\n");
    }
    for (uint32_t ns = 0; ns < 8; ns++)
        for (int i = 0; i < 40; i++) {
            uint32_t x = pick(300), y = pick(300), pitch = 4 * (1 + pick(100)), bpp = pick(2) ? 2 : 4, s = pick(ns + 1);
            printf("M %u %u %u %u %u %u %u\n", x, y, ns, pitch, bpp, s, r300_msaa_offset(x, y, ns, pitch, bpp, s));
        }
    {
        uint32_t dw[8]; for (int i = 0; i < 8; i++) dw[i] = rnd();
        printf("X");
        for (int i = 0; i < 8; i++) printf(" %08x", dw[i]);
        printf(" |");
        for (int i32 = 0; i32 < 2; i32++) for (uint32_t i = 0; i < 20; i++) printf(" %08x", r300_index_at(dw, 8, i32, i));
        printf("\n");
    }
    static const uint32_t sizes[] = { 0, 1, 7, 8, 9, 100, 4096, 32768, 32769, 40000, 65536, 70001, 200000 };
    for (unsigned k = 0; k < sizeof sizes / sizeof *sizes; k++) {
        uint32_t n = sizes[k]; uint8_t *buf = malloc(n + 1); uint32_t s = k + 1;
        for (uint32_t i = 0; i < n; i++) { s = s * 1664525u + 1013904223u; buf[i] = s >> 24; }
        printf("H %u | %016llx %016llx\n", n, (unsigned long long)r300_hash_bytes(buf, n), (unsigned long long)r300_hash_tex(buf, n));
        free(buf);
    }
    for (int i = 0; i < 60; i++) {
        uint32_t ndw = pick(14), d[16];
        for (uint32_t k = 0; k < 16; k++) d[k] = rnd();
        if (ndw) d[0] = (d[0] & ~0x1Fu) | pick(20);
        R300Arrays arr; r300_load_vbpntr(&arr, d, ndw);
        printf("L %u", ndw);
        for (uint32_t k = 0; k < ndw; k++) printf(" %08x", d[k]);
        printf(" | %u", arr.count);
        for (unsigned k = 0; k < 16; k++) printf(" %08x %u %u", arr.a[k].addr, arr.a[k].size_dw, arr.a[k].stride_dw);
        printf("\n");
    }
    return 0;
}
