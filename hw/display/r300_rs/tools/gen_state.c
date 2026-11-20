/* Golden vectors for the R300 state: %g formatting, the register-name table
 * and a full state dump, from the C original (r300_state.c).  Output to
 * stdout in three sections, split by gen_golden.sh:
 *   G bits text          printf("%g") of the float with these bits
 *   N hash               FNV-1a of "addr=name\n" over the register window
 *   DUMP ...             the text r300_state_dump writes */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "r300_state.h"

static uint64_t fnv_add(uint64_t h, const char *s)
{
    for (; *s; s++) { h ^= (uint8_t)*s; h *= 0x100000001b3ull; }
    return h;
}
static uint32_t lcg(uint32_t *s) { *s = *s * 1664525u + 1013904223u; return *s; }
static void g(uint32_t bits)
{
    float f; char buf[64];
    memcpy(&f, &bits, 4);
    snprintf(buf, sizeof buf, "%g", (double)f);
    printf("G %08x %s\n", bits, buf);
}

int main(int argc, char **argv)
{
    if (argc > 1 && !strcmp(argv[1], "dump")) {
        static R300State st;
        uint32_t s = 7;
        r300_state_reset(&st);
        r300_state_write(&st, 0x1000, 0x1234);            /* outside the window: ignored */
        r300_state_write(&st, 0x1D98, 0x3f800000);
        r300_state_write(&st, 0x2080, 0x5);
        r300_state_write(&st, 0x2138, 0xdeadbeef);
        r300_state_write(&st, 0x2FFC, 0x77);              /* a register without a name */
        r300_state_write(&st, 0x4F60, 0x10);
        r300_state_write(&st, 0x22D0, 2 | (4 << 10) | (5u << 20));    /* PVS code 2..5 */
        r300_state_write(&st, R300_VAP_PVS_VECTOR_INDX_REG, 0);
        for (int i = 0; i < 6 * 4; i++) r300_state_write(&st, R300_VAP_PVS_VECTOR_DATA_REG, lcg(&s));
        r300_state_write(&st, R300_VAP_PVS_VECTOR_INDX_REG, R300_PVS_CONST_START);
        static const uint32_t fl[] = { 0x3f800000, 0xbf000000, 0x41200000, 0x3dcccccd, 0x7f800000, 0, 0x358637bd, 0x49742400 };
        for (unsigned i = 0; i < 8 * 4; i++) r300_state_write(&st, R300_VAP_PVS_UPLOAD_DATA, fl[i % 8]);
        r300_state_write(&st, R300_VAP_PVS_VECTOR_INDX_REG, R300_PVS_MEM_VECS - 1);   /* run off the end */
        for (int i = 0; i < 9; i++) r300_state_write(&st, R300_VAP_PVS_VECTOR_DATA_REG, 0x11111111 * (i + 1));
        st.draws = 12345678901ull;
        printf("gen=%llu dw=%u\n", (unsigned long long)st.pvs_gen, st.pvs_upload_dw);
        r300_state_dump(&st, stdout);
        return 0;
    }
    uint32_t s = 99;
    static const uint32_t fixed[] = {
        0, 0x80000000, 0x3f800000, 0xbf800000, 0x3dcccccd, 0x3727c5ac, 0x38d1b717, 0x47f12000,
        0x4b961e40, 0x60ad78ec, 0x7f7fffff, 0x00800000, 0x00000001, 0x7f800000, 0xff800000,
        0x7fc00000, 0xffc00000, 0x3f000000, 0x47c35000, 0x497423f0, 0x38d1b717, 0x37d1b717,
        0x49742400, 0x461c4000, 0x3f7fffff, 0x41200000, 0x42c80000, 0x447a0000, 0x4c189680,
    };
    for (unsigned i = 0; i < sizeof fixed / sizeof *fixed; i++) g(fixed[i]);
    for (int i = 0; i < 300; i++) g(lcg(&s));
    for (int i = 0; i < 200; i++) g((lcg(&s) & 0x807fffff) | ((100 + lcg(&s) % 60) << 23));
    uint64_t h = 0xcbf29ce484222325ull;
    for (uint32_t a = R300_REG_BASE - 4; a <= R300_REG_END; a += 4) {
        const char *n = r300_reg_name(a);
        char line[96];
        snprintf(line, sizeof line, "%04x=%s\n", a, n ? n : "(null)");
        h = fnv_add(h, line);
    }
    printf("N %016llx\n", (unsigned long long)h);
    return 0;
}
