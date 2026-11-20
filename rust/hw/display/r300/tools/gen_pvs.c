/* Golden vectors for the PVS interpreter, the GLSL translator and the
 * disassembler, produced by the C original (r300_pvs.c, r300_pvs_glsl.c).
 * The programs are random but built from valid fields; they are written out
 * so the Rust test does not need to repeat the generator.
 *
 *   K i0 i1 i2 i3            the constant memory, one vec4 (bits), 16 lines
 *   V n bits...              input vertices: 32 vec4 of bits per line (batch vertex i is input i % 8)
 *   C idx first last max_const fc_opc addrs[16] loops[16] words...   one program
 *   R idx unsup runhash      the interpreter over the 8 input vertices
 *   G idx unsup glslhash len   the GLSL translator (unsup 0 = text produced)
 *   B idx unsup hash n       r300_pvs_run_batch over n vertices with a shuffled oidx
 *   O idx oidx...            that oidx
 *   M idx text...            disassembly of every instruction of program idx
 * Then "@@ idx len" and the GLSL text for the first programs that translate. */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_pvs.h"
#include "r300_pvs_glsl.h"

static uint32_t rs = 12345;
static uint32_t rnd(void) { rs = rs * 1664525u + 1013904223u; return rs >> 8; }
static uint32_t pick(uint32_t n) { return rnd() % n; }
static uint64_t fnv(uint64_t h, const void *p, size_t n)
{
    const uint8_t *b = p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 0x100000001b3ull; }
    return h;
}
/* FNV-1a over float bits with -0.0 hashed as +0.0: the sign of a zero that
 * fmaxf/fminf return for a +-0 tie is unspecified and depends on how the C
 * compiler orders the operands (LLVM treats maxnum as commutative). */
static uint64_t fnv_f(uint64_t h, const float *f, size_t n)
{
    for (size_t i = 0; i < n; i++) {
        uint32_t b;
        memcpy(&b, &f[i], 4);
        if (b == 0x80000000u) b = 0;
        h = fnv(h, &b, 4);
    }
    return h;
}
static float fv(uint32_t b) { float f; memcpy(&f, &b, 4); return f; }
static uint32_t bits(float f) { uint32_t b; memcpy(&b, &f, 4); return b; }

static const float fpool[] = { 0, -0.0f, 1, -1, 0.5f, -0.25f, 3.5f, 100, 1e-3f, -7.75f, 2, 128.5f, -300,
                               0.0001f, 1.5f, -2.5f, 3.14159274f, -3.14159274f, 4.0f, 0.1f };
static float rf(void) { return fpool[pick(sizeof fpool / sizeof *fpool)]; }

static uint32_t gen_src(void)
{
    uint32_t s = pick(4);                           /* type */
    uint32_t mode = pick(10) == 0 ? 1 + pick(3) : 0;
    s |= (mode & 1) << 4 | (mode >> 1) << 31;
    s |= (pick(8)) << 5;                            /* offset */
    if (mode == 1) s |= pick(4) << 29;
    if (!(mode & 1) && (mode >> 1)) s |= 0;         /* mode 2 uses the loop index */
    if (pick(5) == 0) s |= 1u << 3;                 /* abs */
    for (int i = 0; i < 4; i++) {
        uint32_t sel = pick(10) < 7 ? (uint32_t)i : pick(8);
        s |= sel << (13 + 3 * i);
        if (pick(6) == 0) s |= 1u << (25 + i);
    }
    if (mode == 1) { s &= ~(7u << 29); s |= pick(4) << 29; }
    return s;
}

static uint32_t gen_d0(void)
{
    static const uint8_t ve[] = { 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,23,24,25,26,27,28 };
    uint32_t d0, kind = pick(10);
    if (kind < 3) {                                 /* math engine */
        d0 = (pick(40) == 0 ? 21 + pick(10) : pick(21)) | 1u << 6;
    } else if (kind == 3) {                         /* macro */
        d0 = pick(2) | 1u << 7;
    } else {                                        /* vector (maybe dual) */
        d0 = pick(40) == 0 ? 15 + pick(8) : ve[pick(sizeof ve)];
        if (pick(8) == 0) d0 |= 1u << 28;
    }
    uint32_t dt = pick(30) == 0 ? 6 + pick(2) : pick(6);   /* TEMP A0 OUT OUT_REPL_X ALT INPUT */
    d0 |= dt << 8;
    d0 |= pick(8) << 13;                            /* offset */
    d0 |= (1 + pick(15)) << 20;                     /* mask */
    uint32_t mode = pick(12) == 0 ? 1 + pick(3) : 0;
    d0 |= (mode & 1) << 31 | (mode >> 1) << 12;
    if (mode == 1) d0 |= pick(4) << 29;
    if (pick(5) == 0) d0 |= 1u << 24;               /* vector saturate */
    if (pick(5) == 0) d0 |= 1u << 25;               /* math saturate */
    if (pick(40) == 0) d0 |= 1u << 26;              /* predication */
    return d0;
}

static uint32_t gen_dual_src(void)
{
    uint32_t s = gen_src() & ~(0xFu << 21) & ~(7u << 19) & ~(3u << 27) & ~(1u << 2);
    s |= (pick(21) & 0xF) << 21 | (uint32_t)(pick(21) >> 4 & 1) << 2;
    s |= pick(4) << 19 | pick(4) << 27;
    return s;
}

int main(void)
{
    static float consts[16][4];
    static float ins[600][R300_PVS_NUM_INPUTS][4];
    static const unsigned NV = 8;
    for (int i = 0; i < 16; i++) {
        for (int k = 0; k < 4; k++) consts[i][k] = rf();
        printf("K %08x %08x %08x %08x\n", bits(consts[i][0]), bits(consts[i][1]), bits(consts[i][2]), bits(consts[i][3]));
    }
    for (unsigned v = 0; v < 600; v++) {
        for (int i = 0; i < R300_PVS_NUM_INPUTS; i++) for (int k = 0; k < 4; k++) ins[v][i][k] = rf();
    }
    for (unsigned v = 0; v < NV; v++) {
        printf("V %u", v);
        for (int i = 0; i < R300_PVS_NUM_INPUTS; i++) for (int k = 0; k < 4; k++) printf(" %08x", bits(ins[v][i][k]));
        printf("\n");
    }
    unsigned samples = 0;
    static R300PVSProgram prog;
    static uint32_t code[R300_PVS_MAX_INSTS * 4];
    for (int idx = 0; idx < 400; idx++) {
        memset(&prog, 0, sizeof prog);
        unsigned n = 1 + pick(12);
        for (unsigned i = 0; i < n; i++) {
            uint32_t d0 = gen_d0();
            code[i * 4] = d0;
            code[i * 4 + 1] = gen_src(); code[i * 4 + 2] = gen_src(); code[i * 4 + 3] = gen_src();
            if (!((d0 >> 6) & 1) && !((d0 >> 7) & 1) && ((d0 >> 28) & 1)) code[i * 4 + 3] = gen_dual_src();
        }
        prog.code = code; prog.first_inst = 0; prog.last_inst = n - 1;
        prog.consts = (const float (*)[4])consts;
        prog.max_const = pick(5) == 0 ? (int)pick(16) : 15;
        if (idx % 4 == 3) {                         /* flow control, for the interpreter only */
            prog.fc_opc = rnd();
            for (int k = 0; k < 16; k++) { prog.fc_addrs[k] = rnd(); prog.fc_loop[k] = rnd(); }
            for (int k = 0; k < 16; k++) {          /* keep addresses near the program */
                prog.fc_addrs[k] = (prog.fc_addrs[k] & 0xFF) % (n + 1) | (pick(4)) << 8 | ((pick(n + 1)) << 16) | ((pick(n + 1)) << 24);
                prog.fc_loop[k] = (prog.fc_loop[k] & 0xFFFF) % 0x0303;
            }
            prog.fc_opc &= 0x0FFFFFFF;
        }
        printf("C %d %u %u %d %u", idx, prog.first_inst, prog.last_inst, prog.max_const, prog.fc_opc);
        for (int k = 0; k < 16; k++) printf(" %u", prog.fc_addrs[k]);
        for (int k = 0; k < 16; k++) printf(" %u", prog.fc_loop[k]);
        for (unsigned i = 0; i < n * 4; i++) printf(" %u", code[i]);
        printf("\n");
        r300_pvs_prepare(&prog);

        uint64_t h = 0xcbf29ce484222325ull;
        uint32_t unsup = 0;
        for (unsigned v = 0; v < NV; v++) {
            float out[R300_PVS_NUM_OUTPUTS][4];
            for (int i = 0; i < R300_PVS_NUM_OUTPUTS; i++) for (int k = 0; k < 4; k++) out[i][k] = fv(0x3f800000u + i * 16 + k);
            unsup |= r300_pvs_run(&prog, ins[v], out);
            h = fnv_f(h, &out[0][0], sizeof out / 4);
        }
        printf("R %d %u %016llx\n", idx, unsup, (unsigned long long)h);

        uint32_t gu = 0;
        char *glsl = r300_pvs_to_glsl(&prog, &gu);
        printf("G %d %u %016llx %zu\n", idx, gu, glsl ? (unsigned long long)fnv(0xcbf29ce484222325ull, glsl, strlen(glsl)) : 0ull, glsl ? strlen(glsl) : 0);
        if (glsl && samples < 6 && idx % 4 != 3) {
            printf("@@ %d %zu\n%s\n", idx, strlen(glsl), glsl);
            samples++;
        }
        free(glsl);
        for (unsigned i = 0; i < n; i++) {
            char buf[160];
            r300_pvs_disasm_inst(&code[i * 4], buf, sizeof buf);
            printf("M %d %u %s\n", idx, i, buf);
        }
        if (idx == 5 || idx == 8 || idx == 12) {    /* batch: shuffled oidx over many vertices */
            enum { N = 600 };
            static float outs[N][R300_PVS_NUM_OUTPUTS][4];
            static float bins[N][R300_PVS_NUM_INPUTS][4];
            for (unsigned i = 0; i < N; i++) memcpy(bins[i], ins[i % NV], sizeof bins[i]);   /* vertex i is input i % 8 */
            static uint32_t oidx[N];
            for (unsigned i = 0; i < N; i++) oidx[i] = i;
            for (unsigned i = N - 1; i > 0; i--) { unsigned j = pick(i + 1); uint32_t t = oidx[i]; oidx[i] = oidx[j]; oidx[j] = t; }
            for (unsigned i = 0; i < N; i++) for (int a = 0; a < R300_PVS_NUM_OUTPUTS; a++) for (int k = 0; k < 4; k++) outs[i][a][k] = fv(0x3f800000u + a * 16 + k);
            uint32_t u = r300_pvs_run_batch(&prog, N, (const float (*)[R300_PVS_NUM_INPUTS][4])bins, outs, oidx);
            printf("B %d %u %016llx %u\n", idx, u, (unsigned long long)fnv_f(0xcbf29ce484222325ull, &outs[0][0][0], sizeof outs / 4), N);
            printf("O %d", idx);
            for (unsigned i = 0; i < N; i++) printf(" %u", oidx[i]);
            printf("\n");
        }
    }
    return 0;
}
