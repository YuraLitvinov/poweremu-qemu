/* Golden vectors for the draw assembly (r300_draw_build, r300_draw_build_indexed),
 * from the C original.  Per scenario:
 *   S idx win_immd le_screen_h glsl_flags
 *   W idx n addr val ...          register writes (PVS memory goes through its ports)
 *   B idx kind opcode ndw dw...   the draw: kind 0 = r300_draw_build, 1 = indexed (opcode = vf, dw = index dwords)
 *   V idx n dw...                 the 3D_LOAD_VBPNTR body
 *   P idx ok|err ...              the packet's digest or the error message
 * Guest memory is a function of the address (see mem_dword), so the Rust test
 * needs no memory image. */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_draw.h"

static uint32_t rs = 91;
static uint32_t rnd(void) { rs = rs * 1664525u + 1013904223u; return rs; }
static uint32_t pick(uint32_t n) { return (rnd() >> 8) % n; }
static uint32_t rnd32(void) { return (rnd() & 0xFFFF0000u) | (rnd() >> 16); }

static uint32_t mem_dword(uint32_t i)
{
    float f = (float)((i * 2654435761u >> 16) & 0xFF) / 64.0f - 2.0f;
    uint32_t b; memcpy(&b, &f, 4); return b;
}
static bool rd(void *opaque, uint32_t addr, void *dst, uint32_t len)
{
    (void)opaque;
    if ((uint64_t)addr + len > 0x10000000u) return false;
    for (uint32_t k = 0; k < len; k++) ((uint8_t *)dst)[k] = (mem_dword((addr + k) >> 2) >> (8 * ((addr + k) & 3))) & 0xFF;
    return true;
}

static uint64_t fnv(uint64_t h, const void *p, size_t n)
{
    const uint8_t *b = p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 0x100000001b3ull; }
    return h;
}
/* float words with -0.0 as +0.0: the sign of a zero out of fmaxf/fminf ties is unspecified */
static uint64_t fnv_f(uint64_t h, const uint32_t *w, size_t n)
{
    for (size_t i = 0; i < n; i++) { uint32_t b = w[i] == 0x80000000u ? 0 : w[i]; h = fnv(h, &b, 4); }
    return h;
}

static R300State st;
static uint32_t wr[4096][2];
static int nwr;
static void W(uint32_t a, uint32_t v) { r300_state_write(&st, a, v); wr[nwr][0] = a; wr[nwr][1] = v; nwr++; }
static void pvs_at(uint32_t vec) { W(0x2200, vec); }
static void pvs_dw(uint32_t v) { W(0x2204, v); }
static uint32_t fbits(float f) { uint32_t b; memcpy(&b, &f, 4); return b; }

static uint32_t plain_swz(void) { return 0u << 13 | 1u << 16 | 2u << 19 | 3u << 22; }

static void digest(int idx, bool ok, const char *err, const R300DrawPacket *p)
{
    if (!ok) { printf("P %d err %s\n", idx, err); return; }
    uint64_t h = 0xcbf29ce484222325ull;
    printf("P %d ok rt %x %u %u %u %u %u %d %u %u %u | sc %u %u %u %u | ncb %u dep %d %x %u %u %u cull %u aa %u warn %x cls %u nv %u nl %u gpu %d gw %u dmm %08x %08x %d %d\n",
           idx, p->rt_gpu_addr, p->rt_pitch, p->rt_width, p->rt_height, p->rt_format, p->rt_bpp, p->rt_dummy, p->rt_view,
           0, 0, p->scissor[0], p->scissor[1], p->scissor[2], p->scissor[3], p->num_cb, p->depth.attach, p->depth.gpu_addr,
           p->depth.pitch, p->depth.bpp, p->depth.format, p->cull, p->aa_samples, p->warn, p->prim_class, p->num_verts,
           p->num_line_verts, p->gpu_vs, p->gpu_words, fbits(p->depth_min), fbits(p->depth_max), p->depth_clip, p->depth_dx);
    for (unsigned k = 1; k < 4; k++) printf("C %u %x %u %u %u %u\n", k, p->cb[k].gpu_addr, p->cb[k].pitch, p->cb[k].format, p->cb[k].bpp, p->cb[k].view);
    h = fnv(h, p->aa_pos, sizeof p->aa_pos);
    printf("H aa %016llx\n", (unsigned long long)h);
    printf("H uni %016llx\n", (unsigned long long)fnv(0xcbf29ce484222325ull, &p->uniforms, sizeof p->uniforms));
    uint64_t t = 0xcbf29ce484222325ull;
    for (int k = 0; k < R300_NUM_TEX_UNITS; k++) {
        const R300TexDesc *d = &p->tex[k];
        uint32_t w[8 + 3 * 12 + 4] = { d->bound, d->gpu_addr, d->width, d->height, d->endian, d->le, d->tile, d->pitch_bytes };
        uint32_t m[10] = { d->format, d->kind, d->view_bpp, d->filter0, d->filter1, d->dim, d->depth, d->levels, d->size_bytes, 0 };
        t = fnv(t, w, 8 * 4); t = fnv(t, m, 9 * 4);
        t = fnv(t, d->lvl_off, 48); t = fnv(t, d->lvl_pitch, 48); t = fnv(t, d->lvl_rows, 48);
    }
    printf("H tex %016llx\n", (unsigned long long)t);
    printf("H glsl %zu %016llx\n", p->glsl ? strlen(p->glsl) : 0, p->glsl ? (unsigned long long)fnv(0xcbf29ce484222325ull, p->glsl, strlen(p->glsl)) : 0ull);
    size_t nv = p->verts ? p->num_verts + p->num_line_verts : 0;
    printf("H verts %zu %016llx\n", nv, (unsigned long long)fnv_f(0xcbf29ce484222325ull, (const uint32_t *)p->verts, nv * sizeof(R300Vertex) / 4));
    printf("H gpu %016llx\n", p->gpu_data ? (unsigned long long)fnv(0xcbf29ce484222325ull, p->gpu_data, p->gpu_words * 4ull) : 0ull);
}

int main(void)
{
    static const uint32_t prims[] = { 1, 2, 3, 4, 4, 4, 5, 6, 6, 7, 8, 12, 13, 14, 15, 4, 4, 6, 0, 9, 10 };
    for (int idx = 0; idx < 150; idx++) {
        r300_state_reset(&st); nwr = 0;
        uint32_t w = 32 + pick(200), h = 32 + pick(200);
        st.win_immd = pick(6) == 0; st.le_screen_h = pick(4) == 0 ? h : 0; st.glsl_flags = pick(4);
        printf("S %d %d %u %u\n", idx, st.win_immd, st.le_screen_h, st.glsl_flags);
        /* colour buffer, scissor, clip rects */
        static const uint32_t cfs[] = { 6, 6, 6, 6, 6, 4, 3, 9, 10, 7, 13, 15, 2 };
        uint32_t cf = cfs[pick(13)];
        W(0x4E28, (pick(1 << 20) << 5));
        W(0x4E38, (cf << 21) | (pick(8) == 0 ? pick(4) << 19 : 0) | ((16 * (2 + pick(40)))));
        W(0x46A4, pick(6) == 0 ? pick(24) : 0);
        W(0x43E0, (1440u) | (1440u << 13));
        W(0x43E4, (1440u + w - 1) | ((1440u + h - 1) << 13));
        if (pick(5) == 0) W(0x43E4, (1440u + w - 1) | (0x1F80u << 13));
        W(0x43D0, pick(3) ? 0xFFFF : rnd() & 0xFFFF);
        for (int k = 0; k < 8; k++) W(0x43B0 + 4 * k, pick(5) == 0 ? (1100 + pick(300)) | ((1100 + pick(300)) << 13) : (1440 + pick(50)) | ((1440 + pick(50)) << 13));
        /* viewport */
        uint32_t vte = pick(4) == 0 ? rnd() & 0x1FF : 0x3F | (pick(3) == 0 ? 1u << 8 : 0);
        W(0x20B0, vte);
        W(0x1D98, fbits(w / 2.0f)); W(0x1D9C, fbits(w / 2.0f)); W(0x1DA0, fbits(-(float)h / 2.0f)); W(0x1DA4, fbits(h / 2.0f));
        W(0x1DA8, fbits(0.5f)); W(0x1DAC, fbits(0.5f));
        W(0x2080, pick(4) == 0 ? rnd32() : 0);
        uint32_t bypass = pick(10) == 0;
        W(0x2140, pick(3) == 0 ? pick(4) | (bypass << 8) : bypass << 8);
        W(0x221C, pick(3) == 0 ? rnd32() & 0x1003F : 0);
        /* vertex streams: element 0 position FLOAT_4, element 1.. random, last vector marked */
        uint32_t nel = 1 + pick(4), dsts[4];
        for (uint32_t k = 0; k < 4; k++) {
            uint32_t ty = k == 0 ? 3 : pick(14), dst = k == 0 ? 0 : 1 + pick(5); dsts[k] = dst;
            uint32_t e = ty | (pick(3) == 0 ? pick(3) << 4 : 0) | (dst << 8) | (k == nel - 1 ? 1u << 13 : 0) | (pick(4) == 0 ? 1u << 14 : 0) | (pick(3) == 0 ? 1u << 15 : 0);
            uint32_t x = plain_swz() | (0xFu << 12); if (k && pick(3) == 0) x = rnd32() & 0xFFFF;
            uint32_t reg = 0x2150 + 4 * (k / 2), ext = 0x21E0 + 4 * (k / 2);
            static uint32_t rv[8], ev[8];
            if (k == 0) { memset(rv, 0, sizeof rv); memset(ev, 0, sizeof ev); }
            rv[k / 2] |= e << (16 * (k % 2)); ev[k / 2] |= x << (16 * (k % 2));
            W(reg, rv[k / 2]); W(ext, ev[k / 2] & 0xFFFFFFFFu);
        }
        uint32_t vsize = pick(3) == 0 ? 4 + pick(20) : 16;
        W(0x20B4, vsize);
        /* PVS: o0 = i0 + t0, o1 = i1 + t0, then random instructions; constants at 512.. */
        uint32_t ninst = 2 + pick(5);
        W(0x22D0, 0 | (ninst * 0) | ((ninst - 1) << 20));
        pvs_at(0);
        for (uint32_t i = 0; i < ninst; i++) {
            uint32_t d0, s0, s1, s2;
            if (i < 2) { d0 = 3 | (2u << 8) | (i << 13) | (0xFu << 20); s0 = 1 | (i << 5) | (plain_swz()); s1 = plain_swz(); s2 = plain_swz(); }
            else {
                static const uint8_t ve[] = { 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14 };
                d0 = ve[pick(sizeof ve)] | (pick(2) ? 2u << 8 : 0u) | (pick(6) << 13) | ((1 + pick(15)) << 20);
                if (pick(5) == 0) d0 |= 1u << 6;                       /* math engine */
                if (pick(4) == 0) d0 |= (pick(2) ? 1u << 24 : 1u << 25);
                uint32_t sr[3]; for (int k = 0; k < 3; k++) { sr[k] = pick(3) | (pick(4) << 5) | plain_swz(); if (pick(5) == 0) sr[k] = pick(1 << 28) & ~(7u << 29); }
                s0 = sr[0]; s1 = sr[1]; s2 = sr[2];
            }
            pvs_dw(d0); pvs_dw(s0); pvs_dw(s1); pvs_dw(s2);
        }
        uint32_t cbase = pick(4) == 0 ? pick(100) : 0;
        pvs_at(512 + cbase);
        for (int i = 0; i < 16; i++) { float f = (float)((int)pick(40) - 20) / 8.0f; pvs_dw(fbits(f)); pvs_dw(fbits(f * 0.5f)); pvs_dw(fbits(1.0f)); pvs_dw(fbits(0.25f)); }
        W(0x22D4, cbase | (pick(4) == 0 ? 15u << 16 : 31u << 16));
        pvs_at(1024); for (int i = 0; i < 24; i++) pvs_dw(fbits((float)((int)pick(9) - 4) / 4.0f));
        W(0x22DC, pick(8) == 0 ? rnd() & 0xFF : 0);
        for (int k = 0; k < 16; k++) { W(0x2230 + 4 * k, pick(4) * 0 + (pick(ninst + 1)) | (pick(3) << 8) | (pick(ninst + 1) << 16) | (pick(ninst + 1) << 24)); }
        /* output format and routing */
        uint32_t fmt0 = 0; if (pick(8) == 0) fmt0 |= 1u << 16; for (int c = 0; c < 4; c++) if (pick(3) == 0) fmt0 |= 1u << (1 + c);
        W(0x2090, fmt0);
        uint32_t fmt1 = 0; for (int t = 0; t < 8; t++) if (pick(3) == 0) fmt1 |= (1 + pick(4)) << (3 * t);
        W(0x2094, fmt1);
        W(0x4008, pick(2) ? rnd32() : 0);
        W(0x401C, pick(8));
        uint32_t nri = pick(4); W(0x4304, nri);
        for (uint32_t i = 0; i <= nri; i++) W(0x4330 + 4 * i, rnd32() & ~(1u << 5));
        for (int i = 0; i < 8; i++) W(0x4310 + 4 * i, rnd32() & 0x7FFFFFFF);
        W(0x4278, pick(3) == 0 ? rnd32() : 0xAAAA);
        W(0x4288, pick(8) == 0 ? (1 | (pick(3) << 4) | (pick(3) << 7)) : 0);
        W(0x42B8, pick(3) == 0 ? pick(8) : 0);
        W(0x4234, pick(2) ? (12 * (1 + pick(3))) | (pick(4) << 16) : 0);
        W(0x4238, pick(3) == 0 ? (fbits(0.5f) & ~3u) | pick(3) : 0);
        W(0x421C, (6 * (8 + pick(24)) << 16) | (6 * (8 + pick(24))));
        W(0x4230, (12 * 4) | (12 * 200 << 16));
        W(0x4020, pick(4) == 0 ? 1 | (pick(4) << 1) : 0);
        W(0x4010, rnd32() & 0xFFFFFF); W(0x4014, rnd32() & 0xFFFFFF);
        W(0x4BC0, pick(4) == 0 ? 1 | pick(8) : 0); W(0x4294, fbits(0.01f)); W(0x4298, fbits(0.2f));
        W(0x4F00, pick(3) ? 3 : 0); W(0x4F10, pick(5) == 0 ? pick(5) : 2); W(0x4F20, pick(1 << 20) << 5); W(0x4F24, (32 * (1 + pick(20))));
        W(0x4E0C, pick(3) == 0 ? 0 : 0xF);
        W(0x4E00, pick(4) << 5);
        W(0x42B4, pick(8)); W(0x42A4, fbits(1.0f)); W(0x42A8, fbits(2.0f)); W(0x42AC, fbits(1.0f)); W(0x42B0, fbits(2.0f));
        /* fragment program and textures as in gen_us.c */
        W(0x4600, (pick(2)) | (pick(2) << 3)); W(0x4608, 0);
        W(0x4610 + 4 * 3, pick(3) | (pick(2) << 6) | (pick(2) << 12) | (pick(2) << 17));
        W(0x4620, pick(32) | (pick(32) << 6) | (pick(16) << 11) | (pick(5) << 15));
        for (int a = 0; a < 4; a++) { W(0x46C0 + 4 * a, rnd32()); W(0x47C0 + 4 * a, rnd32()); W(0x48C0 + 4 * a, rnd32()); W(0x49C0 + 4 * a, rnd32()); }
        W(0x4104, pick(2) ? rnd32() & 0xFFFF : 0);
        for (unsigned u = 0; u < 16; u++) {
            uint32_t f0 = (pick(64) + 0) | (pick(64) << 11) | (pick(12) << 26) | (pick(5) == 0 ? (pick(5) << 22) : 0) | (pick(5) == 0 ? 1u << 31 : 0);
            W(0x4480 + 4 * u, f0);
            W(0x44C0 + 4 * u, pick(32) | (pick(2) << 22) | (pick(3) << 25) | (pick(16) << 5) | (pick(2) << 21) | (pick(8) << 9));
            W(0x4500 + 4 * u, pick(100)); W(0x4400 + 4 * u, rnd32()); W(0x4440 + 4 * u, rnd32());
            W(0x4540 + 4 * u, rnd32()); W(0x45C0 + 4 * u, rnd32());
        }
        for (int i = 0; i < 32 * 4; i++) W(0x4C00 + 4 * i, rnd() & 0xFFFFFF);
        /* the draw */
        uint32_t prim = prims[pick(sizeof prims / sizeof *prims)];
        uint32_t n = 1 + pick(prim == 8 ? 9 : 24);
        int kind = pick(4) == 0, imm = pick(5) == 0;
        uint32_t d[600], ndw = 0, opcode;
        R300Arrays arr; memset(&arr, 0, sizeof arr);
        uint32_t body[16], nbody = 0;
        if (!imm) {
            uint32_t na = 1 + pick(2);
            body[0] = na; body[1] = (4 | (4 << 8)) | (pick(2) ? 0 : ((4 | (4 << 8)) << 16)); body[2] = pick(1 << 16) * 16; body[3] = pick(1 << 16) * 16;
            nbody = 4;
            if (na == 1) { body[1] &= 0xFFFF; }
            r300_load_vbpntr(&arr, body, nbody);
        }
        printf("V %d %u", idx, nbody); for (uint32_t k = 0; k < nbody; k++) printf(" %08x", body[k]); printf("\n");
        R300DrawPacket pkt; const char *err = NULL; bool ok;
        if (kind == 0) {
            opcode = imm ? 0x35 : (pick(2) ? 0x34 : 0x36);
            uint32_t vf = prim | (n << 16) | (pick(2) << 11);
            d[0] = vf; ndw = 1;
            if (opcode == 0x36) { uint32_t ni = (vf >> 11) & 1 ? n : (n + 1) / 2; for (uint32_t k = 0; k < ni; k++) d[ndw++] = (pick(n) | (pick(n) << 16)) & ((vf >> 11) & 1 ? 0xFF : 0xFFFFFFFF); }
            if (opcode == 0x35) { for (uint32_t k = 0; k < n * vsize + pick(3); k++) d[ndw++] = mem_dword(k + idx * 77); }
            printf("B %d 0 %u %u", idx, opcode, ndw); for (uint32_t k = 0; k < ndw; k++) printf(" %08x", d[k]); printf("\n");
            ok = r300_draw_build(&st, &arr, opcode, d, ndw, rd, NULL, &pkt, &err);
        } else {
            uint32_t vf = prim | (n << 16) | (pick(2) << 11);
            uint32_t ni = (vf >> 11) & 1 ? n : (n + 1) / 2; if (pick(8) == 0) ni = ni > 0 ? ni - 1 : 0;
            for (uint32_t k = 0; k < ni; k++) d[k] = pick(n) | (pick(n) << 16);
            R300Indices ix = { d, ni };
            printf("B %d 1 %u %u", idx, vf, ni); for (uint32_t k = 0; k < ni; k++) printf(" %08x", d[k]); printf("\n");
            ok = r300_draw_build_indexed(&st, &arr, vf, &ix, rd, NULL, &pkt, &err);
        }
        printf("W %d %d", idx, nwr); for (int k = 0; k < nwr; k++) printf(" %x %x", wr[k][0], wr[k][1]); printf("\n");
        digest(idx, ok, err, &pkt);
        if (ok) r300_draw_free(&pkt);
    }
    return 0;
}
