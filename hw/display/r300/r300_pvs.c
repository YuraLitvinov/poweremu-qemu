/*
 * R300-family Programmable Vertex Shader (PVS) interpreter.
 *
 * Semantics: AMD "R5xx Acceleration" v1.5, 7.5.7 (vector ops) and 7.5.8
 * (math ops), and the PVS instruction tables that follow.  Math ops read
 * the W channel of their operands (after swizzle): A.w is the argument or
 * base, B.w a clamp, C.w an exponent.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "r300_pvs.h"

#include <float.h>
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef struct PVSRegs {
    float temp[R300_PVS_NUM_TEMPS][4];
    float alt[R300_PVS_NUM_ALT_TEMPS][4];
    int a0[4];
} PVSRegs;

typedef struct PVSCtx {
    const R300PVSProgram *prog;
    const float (*in)[4];
    float (*out)[4];
    PVSRegs r;
    int al;                 /* fixed-point loop index of the innermost loop */
    uint32_t unsup;
} PVSCtx;

static const float zero4[4];

static const float *pvs_mem(PVSCtx *c, unsigned type, int index)
{
    switch (type) {
    case SRC_TEMP:
        return index >= 0 && index < R300_PVS_NUM_TEMPS ? c->r.temp[index] : zero4;
    case SRC_INPUT:
        return index >= 0 && index < R300_PVS_NUM_INPUTS ? c->in[index] : zero4;
    case SRC_CONST:
        return index >= 0 && index <= c->prog->max_const ?
               c->prog->consts[index] : zero4;
    default:
        return index >= 0 && index < R300_PVS_NUM_ALT_TEMPS ? c->r.alt[index] : zero4;
    }
}

/* ADDR_MODE: 0 absolute, 1 relative to A0.ADDR_SEL, 2 relative to the
 * loop index (R5xx guide, PVS source and destination operands). */
static int pvs_rel(PVSCtx *c, unsigned mode, unsigned sel, unsigned offset)
{
    if (mode == 1) {
        return (int)offset + c->r.a0[sel & 3];
    }
    if (mode == 2) {
        return (int)offset + c->al;
    }
    return offset;
}

static int pvs_addr(PVSCtx *c, uint32_t s, unsigned offset)
{
    return pvs_rel(c, ((s >> 4) & 1) | ((s >> 31) << 1), s >> 29, offset);
}

static float pvs_select(const float *v, unsigned sel)
{
    return sel < 4 ? v[sel] : sel == 5 ? 1.0f : 0.0f;
}

/*
 * A source operand as decoded once per draw (r300_pvs_prepare): swizzled, abs and negate applied.  A plain
 * operand is the register itself (no copy); otherwise the result goes to o.
 */
static inline const float *pvs_src(PVSCtx *c, const R300PVSSrc *s, float o[4])
{
    const float *v = pvs_mem(c, s->type, s->mode ? pvs_rel(c, s->mode, s->sel, s->offset) : s->offset);
    float t[8];

    if (s->plain) {
        return v;
    }
    t[0] = v[0]; t[1] = v[1]; t[2] = v[2]; t[3] = v[3];
    t[4] = 0.0f; t[5] = 1.0f; t[6] = 0.0f; t[7] = 0.0f;
    for (int i = 0; i < 4; i++) {
        uint32_t b;

        memcpy(&b, &t[s->swz[i]], sizeof(b));
        b = (b & s->absmask) ^ s->neg[i];
        memcpy(&o[i], &b, sizeof(b));
    }
    return o;
}

static void pvs_decode_src(R300PVSSrc *d, uint32_t s)
{
    d->type = s & 3;
    d->mode = ((s >> 4) & 1) | ((s >> 31) << 1);
    d->sel = (s >> 29) & 7;
    d->offset = (s >> 5) & 0xFF;
    d->absmask = (s & (1u << 3)) ? 0x7fffffffu : 0xffffffffu;
    d->plain = d->absmask == 0xffffffffu;
    for (int i = 0; i < 4; i++) {
        d->swz[i] = (s >> (13 + 3 * i)) & 7;
        d->neg[i] = (s & (1u << (25 + i))) ? 0x80000000u : 0;
        d->plain = d->plain && d->swz[i] == i && !d->neg[i];
    }
}

static float pvs_pow_ff(float base, float e)
{
    /* Special cases, in the documented order of detection. */
    if (base == 0.0f) {
        return e < 0.0f ? INFINITY : 0.0f;
    }
    if (e == 0.0f) {
        return 1.0f;
    }
    if (base < 0.0f) {
        return -powf(-base, e);
    }
    return powf(base, e);
}

/* Math engine: scalar operands A, B, C (the .w channels). */
static void pvs_math(PVSCtx *c, unsigned op, float a, float b, float cc,
                     float r[4])
{
    float x;

    switch (op) {
    case ME_EXP_BASE2_DX:
        r[0] = exp2f(floorf(a));
        r[1] = a > 128.0f ? 0.0f : a - floorf(a);
        r[2] = exp2f(a);
        r[3] = 1.0f;
        return;
    case ME_LOG_BASE2_DX:
        if (a == 0.0f) {
            r[0] = -FLT_MAX; r[1] = 1.0f; r[2] = -FLT_MAX; r[3] = 1.0f;
        } else {
            int e;
            float m = frexpf(fabsf(a), &e);    /* m in [0.5, 1) */
            r[0] = (float)(e - 1);
            r[1] = m * 2.0f;
            r[2] = log2f(fabsf(a));
            r[3] = 1.0f;
        }
        return;
    case ME_LIGHT_COEFF_DX:
        r[0] = 1.0f;
        r[1] = fmaxf(b, 0.0f);
        r[2] = b > 0.0f ? pvs_pow_ff(fmaxf(a, 0.0f),
                                     fminf(fmaxf(cc, -128.0f), 128.0f)) : 0.0f;
        r[3] = 1.0f;
        return;
    case ME_EXP_BASEE_FF:       x = expf(a); break;
    case ME_POWER_FUNC_FF:      x = pvs_pow_ff(a, cc); break;
    case ME_RECIP_DX:           x = a == 0.0f ? FLT_MAX : 1.0f / a; break;
    case ME_RECIP_FF:           x = a == 0.0f ? 0.0f : 1.0f / a; break;
    case ME_RECIP_SQRT_DX:      x = a == 0.0f ? FLT_MAX : 1.0f / sqrtf(fabsf(a)); break;
    case ME_RECIP_SQRT_FF:      x = a == 0.0f ? 0.0f : 1.0f / sqrtf(fabsf(a)); break;
    case ME_MULTIPLY:           x = a * b; break;
    case ME_EXP_BASE2_FULL_DX:  x = exp2f(a); break;
    case ME_LOG_BASE2_FULL_DX:  x = a == 0.0f ? -FLT_MAX : log2f(fabsf(a)); break;
    case ME_POWER_FUNC_FF_CLAMP_B:
        x = a < b ? 0.0f : pvs_pow_ff(a, cc);
        break;
    case ME_POWER_FUNC_FF_CLAMP_B1:
        x = a < b ? 0.0f : a > 1.0f ? 1.0f : pvs_pow_ff(a, cc);
        break;
    case ME_POWER_FUNC_FF_CLAMP_01:
        x = a <= 0.0f ? 0.0f : a > 1.0f ? 1.0f : pvs_pow_ff(a, cc);
        break;
    case ME_SIN:
    case ME_COS: {
        /* Inputs outside [-pi, pi] clamp: sin 0, cos -1. */
        float t = fminf(fmaxf(a, (float)-M_PI), (float)M_PI);
        x = op == ME_SIN ? sinf(t) : cosf(t);
        break;
    }
    case ME_LOG_BASE2_IEEE:     x = log2f(fabsf(a)); break;
    case ME_RECIP_IEEE:         x = 1.0f / a; break;
    case ME_RECIP_SQRT_IEEE:    x = 1.0f / sqrtf(fabsf(a)); break;
    case ME_NOP:                x = 0.0f; break;
    default:
        c->unsup |= R300_PVS_UNSUP_OPCODE;
        x = 0.0f;
        break;
    }
    r[0] = r[1] = r[2] = r[3] = x;
}

static void pvs_vector(PVSCtx *c, unsigned op, const float *a, const float *b,
                       const float *cc, float r[4])
{
    switch (op) {
    case VE_DOT_PRODUCT: {
        float d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
        r[0] = r[1] = r[2] = r[3] = d;
        return;
    }
    case VE_DISTANCE_VECTOR:
        r[0] = 1.0f; r[1] = a[1] * b[1]; r[2] = a[2]; r[3] = b[3];
        return;
    case VE_MULTIPLY_CLAMP: {
        float x;
        if (cc[3] < a[3] * b[3]) {
            x = cc[3];
        } else if (cc[0] >= a[0] * b[0]) {
            x = cc[0];
        } else {
            x = a[0] * b[0];
        }
        r[0] = r[1] = r[2] = r[3] = x;
        return;
    }
    }
    /* one loop per opcode, so the compiler can vectorise the common ones */
#define VLOOP(expr) for (int i = 0; i < 4; i++) { r[i] = (expr); } return
    switch (op) {
    case VE_NOP:                    VLOOP(0.0f);
    case VE_MULTIPLY:               VLOOP(a[i] * b[i]);
    case VE_ADD:                    VLOOP(a[i] + b[i]);
    case VE_MULTIPLY_ADD:           VLOOP(a[i] * b[i] + cc[i]);
    case VE_FRACTION:               VLOOP(a[i] - floorf(a[i]));
    case VE_MAXIMUM:                VLOOP(fmaxf(a[i], b[i]));
    case VE_MINIMUM:                VLOOP(fminf(a[i], b[i]));
    case VE_SET_GREATER_THAN_EQUAL: VLOOP(a[i] >= b[i]);
    case VE_SET_LESS_THAN:          VLOOP(a[i] < b[i]);
    case VE_MULTIPLYX2_ADD:         VLOOP(2.0f * (a[i] * b[i]) + cc[i]);
    case VE_FLT2FIX_DX:             VLOOP(floorf(a[i]));
    case VE_FLT2FIX_DX_RND:         VLOOP(floorf(a[i] + 0.5f));
    case VE_COND_MUX_EQ:            VLOOP(a[i] == 0.0f ? b[i] : cc[i]);
    case VE_COND_MUX_GT:            VLOOP(a[i] > 0.0f ? b[i] : cc[i]);
    case VE_COND_MUX_GTE:           VLOOP(a[i] >= 0.0f ? b[i] : cc[i]);
    case VE_SET_GREATER_THAN:       VLOOP(a[i] > b[i]);
    case VE_SET_EQUAL:              VLOOP(a[i] == b[i]);
    case VE_SET_NOT_EQUAL:          VLOOP(a[i] != b[i]);
    default:
        c->unsup |= R300_PVS_UNSUP_OPCODE;
        r[0] = r[1] = r[2] = r[3] = 0.0f;
        return;
    }
#undef VLOOP
}

static void pvs_sat(float r[4])
{
    for (int i = 0; i < 4; i++) {
        r[i] = fminf(fmaxf(r[i], 0.0f), 1.0f);
    }
}

static void pvs_write(PVSCtx *c, uint32_t d0, const float r[4])
{
    unsigned type = (d0 >> 8) & 0xF;
    unsigned mask = (d0 >> 20) & 0xF;
    /* ADDR_MODE_0 is bit 31, ADDR_MODE_1 bit 12 */
    unsigned mode = ((d0 >> 31) & 1) | (((d0 >> 12) & 1) << 1);
    int rel = pvs_rel(c, mode, d0 >> 29, (d0 >> 13) & 0x7F);
    unsigned index = rel < 0 ? ~0u : (unsigned)rel;
    float *dst;

    switch (type) {
    case DST_TEMP:
        dst = index < R300_PVS_NUM_TEMPS ? c->r.temp[index] : NULL;
        break;
    case DST_A0:
        for (int i = 0; i < 4; i++) {
            if (mask & (1u << i)) {
                float f = floorf(r[i]);
                c->r.a0[i] = f < -256.0f ? -256 : f > 255.0f ? 255 : (int)f;
            }
        }
        return;
    case DST_OUT:
    case DST_OUT_REPL_X:
        dst = index < R300_PVS_NUM_OUTPUTS ? c->out[index] : NULL;
        break;
    case DST_ALT:
        dst = index < R300_PVS_NUM_ALT_TEMPS ? c->r.alt[index] : NULL;
        break;
    default:
        return;
    }
    if (!dst) {
        return;
    }
    for (int i = 0; i < 4; i++) {
        if (mask & (1u << i)) {
            dst[i] = type == DST_OUT_REPL_X ? r[0] : r[i];
        }
    }
}

/*
 * The instruction after pc, following the flow control instructions
 * (AMD R5xx guide 7.5.5; VAP_PVS_FLOW_CNTL_*), once pc has run.  A
 * loop's end and a subroutine's
 * return are live only while that loop or call is.
 *
 *   ADDRS: 7:0 activation instruction (the last before the redirect),
 *          15:8 JUMP/JSR target or LOOP count, 23:16 last instruction of
 *          the loop or subroutine, 31:24 loop start or return address.
 *   LOOP_INDEX: 7:0 initial loop index, 15:8 signed step.
 */
typedef struct PVSLoop {
    int fc;                 /* its FLOW_CNTL instruction */
    unsigned count;         /* iterations left */
    int saved_al;           /* the enclosing loop's index */
} PVSLoop;

static unsigned pvs_flow(const R300PVSProgram *prog, PVSCtx *c, unsigned pc,
                         PVSLoop *loop, unsigned *nloop, int *jsr, unsigned *njsr)
{
    if (*nloop) {
        int k = loop[*nloop - 1].fc;
        if (((prog->fc_addrs[k] >> 16) & 0xFF) == pc) {
            c->al += (int8_t)(prog->fc_loop[k] >> 8);
            if (--loop[*nloop - 1].count) {
                return (prog->fc_addrs[k] >> 24) & 0xFF;
            }
            c->al = loop[--*nloop].saved_al;
            return pc + 1;
        }
    }
    if (*njsr) {
        int k = jsr[*njsr - 1];
        if (((prog->fc_addrs[k] >> 16) & 0xFF) == pc) {
            --*njsr;
            return (prog->fc_addrs[k] >> 24) & 0xFF;
        }
    }
    for (int k = 0; k < 16; k++) {
        unsigned op = (prog->fc_opc >> (2 * k)) & 3;
        uint32_t a = prog->fc_addrs[k];

        if (!op || (a & 0xFF) != pc) {
            continue;
        }
        switch (op) {
        case 1:                                 /* JUMP */
            return (a >> 8) & 0xFF;
        case 2:                                 /* LOOP */
            if (!((a >> 8) & 0xFF) || *nloop == 16) {
                return pc + 1;
            }
            loop[*nloop].fc = k;
            loop[*nloop].count = (a >> 8) & 0xFF;
            loop[*nloop].saved_al = c->al;
            ++*nloop;
            c->al = prog->fc_loop[k] & 0xFF;
            return (a >> 24) & 0xFF;
        default:                                /* JSR */
            if (*njsr == 16) {
                return pc + 1;
            }
            jsr[(*njsr)++] = k;
            return (a >> 8) & 0xFF;
        }
    }
    return pc + 1;
}

/* One instruction. */
static void pvs_exec(PVSCtx *c, const R300PVSInst *ins)
{
    const uint32_t *d = ins->d;
    uint32_t d0 = d[0];
    unsigned op = d0 & 0x3F;
    bool math = (d0 >> 6) & 1;
    bool macro = (d0 >> 7) & 1;
    bool dual = !math && !macro && ((d0 >> 28) & 1);
    float sa[4], sb[4], sc[4], r[4];
    static const float zero[4];
    const float *a, *b, *cc;     /* dual: no third operand */

    if ((d0 >> 26) & 1) {
        c->unsup |= R300_PVS_UNSUP_PRED;
    }
    a = pvs_src(c, &ins->src[0], sa);
    b = pvs_src(c, &ins->src[1], sb);
    cc = dual ? zero : pvs_src(c, &ins->src[2], sc);

    if (math) {
        pvs_math(c, op, a[3], b[3], cc[3], r);
        if ((d0 >> 25) & 1) {
            pvs_sat(r);
        }
        pvs_write(c, d0, r);
        return;
    }

    if (macro) {
        /* Two-clock MAD/M2X_ADD with three distinct temporaries. */
        pvs_vector(c, (op & 1) ? VE_MULTIPLYX2_ADD : VE_MULTIPLY_ADD,
                   a, b, cc, r);
    } else {
        pvs_vector(c, op, a, b, cc, r);
    }
    if ((d0 >> 24) & 1) {
        pvs_sat(r);
    }

    if (dual) {
        /* The third source dword describes a math op writing ATRM 0-3. */
        uint32_t s = d[3];
        unsigned mop = ((s >> 21) & 0xF) | (((s >> 2) & 1) << 4);
        const float *v = pvs_mem(c, s & 3, pvs_addr(c, s, (s >> 5) & 0xFF));
        float x = pvs_select(v, (s >> 13) & 7);
        float y = pvs_select(v, (s >> 16) & 7);
        float mr[4];

        if (s & (1u << 3)) {
            x = fabsf(x);
            y = fabsf(y);
        }
        if (s & (1u << 25)) {
            x = -x;
        }
        if (s & (1u << 26)) {
            y = -y;
        }
        pvs_math(c, mop, x, y, y, mr);
        if ((d0 >> 25) & 1) {
            pvs_sat(mr);
        }
        pvs_write(c, d0, r);   /* vector result first... */
        {
            unsigned comp = (s >> 27) & 3;
            c->r.alt[(s >> 19) & 3][comp] = mr[comp];  /* ...then math */
        }
        return;
    }
    pvs_write(c, d0, r);
}

void r300_pvs_prepare(R300PVSProgram *prog)
{
    unsigned nt = 0;
    bool all = false, alt = false, inrel = false;
    uint32_t imask = 0;

    for (unsigned pc = prog->first_inst; pc <= prog->last_inst && pc < R300_PVS_MAX_INSTS; pc++) {
        R300PVSInst *ins = &prog->dec[pc];
        uint32_t d0;

        memcpy(ins->d, &prog->code[pc * 4], sizeof(ins->d));
        d0 = ins->d[0];
        for (int k = 0; k < 3; k++) {
            pvs_decode_src(&ins->src[k], ins->d[k + 1]);
        }
        /* which temporaries can the program touch?  A relative address could reach any of them. */
        for (int k = 0; k < 3; k++) {
            const R300PVSSrc *sr = &ins->src[k];

            if (sr->type == SRC_ALT) {
                alt = true;
            }
            if (sr->type == SRC_INPUT) {
                if (sr->mode) {
                    inrel = true;
                } else if (sr->offset < R300_PVS_NUM_INPUTS) {
                    imask |= 1u << sr->offset;
                }
            }
            if (sr->type == SRC_TEMP) {
                if (sr->mode) {
                    all = true;
                } else if (sr->offset + 1u > nt) {
                    nt = sr->offset + 1u;
                }
            }
        }
        if (((d0 >> 8) & 0xF) == DST_ALT || (!((d0 >> 6) & 1) && !((d0 >> 7) & 1) && ((d0 >> 28) & 1))) {
            alt = true;                 /* an ATRM destination, or a dual-issue math half that writes one */
        }
        if (((d0 >> 8) & 0xF) == DST_TEMP) {
            unsigned mode = ((d0 >> 31) & 1) | (((d0 >> 12) & 1) << 1);

            if (mode) {
                all = true;
            } else if (((d0 >> 13) & 0x7F) + 1u > nt) {
                nt = ((d0 >> 13) & 0x7F) + 1u;
            }
        }
    }
    prog->ntemps = all || nt > R300_PVS_NUM_TEMPS ? R300_PVS_NUM_TEMPS : nt;
    prog->uses_alt = alt;
    prog->in_mask = inrel ? 0xffffffffu : imask;
}

uint32_t r300_pvs_run(const R300PVSProgram *prog,
                      const float in[R300_PVS_NUM_INPUTS][4],
                      float out[R300_PVS_NUM_OUTPUTS][4])
{
    PVSCtx c;

    /* only the registers the program can read need to start at zero (see r300_pvs_prepare) */
    memset(c.r.temp, 0, prog->ntemps * sizeof(c.r.temp[0]));
    if (prog->uses_alt) {
        memset(c.r.alt, 0, sizeof(c.r.alt));
    }
    memset(c.r.a0, 0, sizeof(c.r.a0));
    c.prog = prog;
    c.in = in;
    c.out = out;
    c.unsup = 0;

    /* Flow control state: active loops (innermost last) and subroutine
     * returns, as indices of their FLOW_CNTL instruction. */
    PVSLoop loop[16];
    int jsr[16];
    unsigned nloop = 0, njsr = 0, steps = 0;
    c.al = 0;

    for (unsigned pc = prog->first_inst, next;
         pc <= prog->last_inst && pc < R300_PVS_MAX_INSTS; pc = next) {
        if (++steps > 65536) {
            c.unsup |= R300_PVS_UNSUP_FLOW;     /* runaway loop */
            break;
        }
        pvs_exec(&c, &prog->dec[pc]);
        next = prog->fc_opc ? pvs_flow(prog, &c, pc, loop, &nloop, jsr, &njsr)
                            : pc + 1;
    }
    return c.unsup;
}

/* ---- batches over a small worker pool ---------------------------------- */

#define PVS_MAX_WORKERS     7           /* plus the calling thread */
#define PVS_MIN_BATCH       256         /* smaller batches cost more to hand out than to run */
#define PVS_CHUNK           64

typedef struct PVSJob {
    const R300PVSProgram *prog;
    unsigned n;
    const float (*in)[R300_PVS_NUM_INPUTS][4];
    float (*out)[R300_PVS_NUM_OUTPUTS][4];
    const uint32_t *oidx;
    unsigned next;                      /* next unclaimed vertex (atomic) */
    uint32_t unsup;                     /* OR of the flags (atomic) */
} PVSJob;

static struct {
    pthread_mutex_t call;               /* one batch at a time */
    pthread_mutex_t m;
    pthread_cond_t wake, done;
    unsigned gen;                       /* bumps per batch */
    unsigned pending;                   /* workers that have not finished the current batch */
    int nworkers;
    PVSJob job;
} pool = { PTHREAD_MUTEX_INITIALIZER, PTHREAD_MUTEX_INITIALIZER,
           PTHREAD_COND_INITIALIZER, PTHREAD_COND_INITIALIZER, 0, 0, -1 };

static void pvs_job_work(PVSJob *j)
{
    uint32_t u = 0;

    for (;;) {
        unsigned i0 = __atomic_fetch_add(&j->next, PVS_CHUNK, __ATOMIC_RELAXED);
        unsigned i1;

        if (i0 >= j->n) {
            break;
        }
        i1 = i0 + PVS_CHUNK < j->n ? i0 + PVS_CHUNK : j->n;
        for (unsigned i = i0; i < i1; i++) {
            u |= r300_pvs_run(j->prog, j->in[i], j->out[j->oidx[i]]);
        }
    }
    if (u) {
        __atomic_fetch_or(&j->unsup, u, __ATOMIC_RELAXED);
    }
}

static void *pvs_worker(void *arg)
{
    unsigned seen = 0;

    (void)arg;
    pthread_mutex_lock(&pool.m);
    for (;;) {
        while (pool.gen == seen) {
            pthread_cond_wait(&pool.wake, &pool.m);
        }
        seen = pool.gen;
        pthread_mutex_unlock(&pool.m);
        pvs_job_work(&pool.job);
        pthread_mutex_lock(&pool.m);
        if (--pool.pending == 0) {
            pthread_cond_signal(&pool.done);
        }
    }
    return NULL;
}

static int pvs_pool_init(void)
{
    if (pool.nworkers < 0) {
        const char *e = getenv("R300_PVS_THREADS");   /* total threads, 1 = serial */
        long cpus = sysconf(_SC_NPROCESSORS_ONLN);
        int want = e ? atoi(e) : (int)(cpus < 8 ? cpus : 8);
        int made = 0;

        want = want < 1 ? 1 : want > PVS_MAX_WORKERS + 1 ? PVS_MAX_WORKERS + 1 : want;
        for (int i = 0; i < want - 1; i++) {
            pthread_t t;

            if (pthread_create(&t, NULL, pvs_worker, NULL)) {
                break;
            }
            pthread_detach(t);
            made++;
        }
        pool.nworkers = made;
    }
    return pool.nworkers;
}

uint32_t r300_pvs_run_batch(const R300PVSProgram *prog, unsigned n,
                            const float (*in)[R300_PVS_NUM_INPUTS][4],
                            float (*out)[R300_PVS_NUM_OUTPUTS][4],
                            const uint32_t *oidx)
{
    PVSJob serial = { prog, n, in, out, oidx, 0, 0 };

    if (n < PVS_MIN_BATCH || pvs_pool_init() == 0 || pthread_mutex_trylock(&pool.call)) {
        pvs_job_work(&serial);
        return serial.unsup;
    }
    pthread_mutex_lock(&pool.m);
    pool.job = serial;
    pool.pending = pool.nworkers;
    pool.gen++;
    pthread_cond_broadcast(&pool.wake);
    pthread_mutex_unlock(&pool.m);

    pvs_job_work(&pool.job);

    pthread_mutex_lock(&pool.m);
    while (pool.pending) {
        pthread_cond_wait(&pool.done, &pool.m);
    }
    serial.unsup = pool.job.unsup;
    pthread_mutex_unlock(&pool.m);
    pthread_mutex_unlock(&pool.call);
    return serial.unsup;
}

void r300_pvs_disasm_inst(const uint32_t d[4], char *buf, unsigned len)
{
    static const char *dst_type[] = { "t", "a0", "o", "ox", "at", "i", "?", "?" };
    static const char *src_type[] = { "t", "i", "c", "at" };
    static const char swz[] = "xyzw01??";
    uint32_t d0 = d[0];
    int n;

    n = snprintf(buf, len, "%s%s op%-2u %s%u.%s%s%s%s <-",
                 (d0 >> 6) & 1 ? "ME" : (d0 >> 7) & 1 ? "MACRO" : "VE",
                 !((d0 >> 6) & 1) && ((d0 >> 28) & 1) ? "+dual" : "",
                 d0 & 0x3F, dst_type[(d0 >> 8) & 7], (d0 >> 13) & 0x7F,
                 (d0 >> 20) & 1 ? "x" : "", (d0 >> 21) & 1 ? "y" : "",
                 (d0 >> 22) & 1 ? "z" : "", (d0 >> 23) & 1 ? "w" : "");
    for (int k = 1; k < 4 && n > 0 && (unsigned)n < len; k++) {
        uint32_t s = d[k];
        char sw[5];
        for (int i = 0; i < 4; i++) {
            sw[i] = swz[(s >> (13 + 3 * i)) & 7];
            if (s & (1u << (25 + i))) {
                sw[i] = sw[i] >= 'a' ? sw[i] - 32 : sw[i];   /* upper = negated */
            }
        }
        sw[4] = 0;
        n += snprintf(buf + n, len - n, " %s%s[%u%s].%s",
                      s & 8 ? "|" : "", src_type[s & 3], (s >> 5) & 0xFF,
                      s & 16 ? "+a0" : "", sw);
    }
}
