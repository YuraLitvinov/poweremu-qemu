/*
 * R300-family PVS programs as GLSL: see r300_pvs_glsl.h.  The semantics are those of r300_pvs.c, instruction
 * by instruction; where the two differ the interpreter is the reference.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "r300_pvs_glsl.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct Sb {
    char *p;
    size_t n, cap;
} Sb;

static void sb_printf(Sb *sb, const char *fmt, ...)
{
    va_list ap;
    int k;

    for (;;) {
        va_start(ap, fmt);
        k = vsnprintf(sb->p ? sb->p + sb->n : NULL, sb->cap - sb->n, fmt, ap);
        va_end(ap);
        if (k >= 0 && sb->n + (size_t)k + 1 <= sb->cap) {
            sb->n += k;
            return;
        }
        sb->cap = sb->cap ? sb->cap * 2 : 4096;
        sb->p = realloc(sb->p, sb->cap);
    }
}

static const char comp[] = "xyzw";

/* The register an operand names, before swizzle, abs and negate, as a vec4 expression. */
static void reg_expr(char *out, size_t len, const R300PVSProgram *prog, const R300PVSSrc *s)
{
    const char *zero = "vec4(0.0)";
    char ri[64];

    if (s->mode == 0) {
        unsigned i = s->offset;

        switch (s->type) {
        case SRC_TEMP:
            if (i < R300_PVS_NUM_TEMPS) {
                snprintf(out, len, "T[%u]", i);
            } else {
                snprintf(out, len, "%s", zero);
            }
            return;
        case SRC_INPUT:
            snprintf(out, len, i < R300_PVS_NUM_INPUTS ? "IN[%u]" : "vec4(0.0)", i);
            return;
        case SRC_CONST:
            if ((int)i <= prog->max_const) {
                snprintf(out, len, "pvs_consts[%u]", i);
            } else {
                snprintf(out, len, "%s", zero);
            }
            return;
        default:
            snprintf(out, len, i < R300_PVS_NUM_ALT_TEMPS ? "A[%u]" : "vec4(0.0)", i);
            return;
        }
    }
    /* relative to A0.sel (mode 1) or to the loop index, which is 0 outside the loops this translator leaves out */
    if (s->mode == 1) {
        snprintf(ri, sizeof(ri), "(%u + a0[%u])", s->offset, s->sel & 3u);
    } else {
        snprintf(ri, sizeof(ri), "%u", s->offset);
    }
    switch (s->type) {
    case SRC_TEMP:
        snprintf(out, len, "(%s >= 0 && %s < %d ? T[%s] : vec4(0.0))", ri, ri, R300_PVS_NUM_TEMPS, ri);
        break;
    case SRC_INPUT:
        snprintf(out, len, "(%s >= 0 && %s < %d ? IN[%s] : vec4(0.0))", ri, ri, R300_PVS_NUM_INPUTS, ri);
        break;
    case SRC_CONST:
        snprintf(out, len, "(%s >= 0 && %s <= %d ? pvs_consts[%s] : vec4(0.0))", ri, ri, prog->max_const, ri);
        break;
    default:
        snprintf(out, len, "(%s >= 0 && %s < %d ? A[%s] : vec4(0.0))", ri, ri, R300_PVS_NUM_ALT_TEMPS, ri);
        break;
    }
}

/* A swizzled source operand: declares vec4 `name` in the current block. */
static void emit_src(Sb *sb, const R300PVSProgram *prog, const R300PVSSrc *s, const char *name)
{
    char reg[160];

    reg_expr(reg, sizeof(reg), prog, s);
    if (s->plain) {
        sb_printf(sb, "    vec4 %s = %s;\n", name, reg);
        return;
    }
    sb_printf(sb, "    vec4 %s_r = %s;\n    vec4 %s = vec4(", name, reg, name);
    for (int i = 0; i < 4; i++) {
        char e[48];
        unsigned sel = s->swz[i];

        if (sel < 4) {
            snprintf(e, sizeof(e), "%s_r.%c", name, comp[sel]);
        } else {
            snprintf(e, sizeof(e), "%s", sel == 5 ? "1.0" : "0.0");
        }
        sb_printf(sb, "%s%s%s%s%s", i ? ", " : "", s->neg[i] ? "-" : "",
                  s->absmask != 0xffffffffu ? "abs(" : "(", e, ")");
    }
    sb_printf(sb, ");\n");
}

/* Math engine: statements that set vec4 `r` from the scalars a, b and c. */
static bool emit_math(Sb *sb, unsigned op, const char *a, const char *b, const char *c, const char *r)
{
    const char *fm = "3.4028235e38";

    switch (op) {
    case ME_EXP_BASE2_DX:
        sb_printf(sb, "    %s = vec4(exp2(floor(%s)), %s > 128.0 ? 0.0 : %s - floor(%s), exp2(%s), 1.0);\n",
                  r, a, a, a, a, a);
        return true;
    case ME_LOG_BASE2_DX:
        sb_printf(sb, "    if (%s == 0.0) { %s = vec4(-%s, 1.0, -%s, 1.0); }\n"
                      "    else { int e_; float m_ = frexp(abs(%s), e_); %s = vec4(float(e_ - 1), m_ * 2.0, log2(abs(%s)), 1.0); }\n",
                  a, r, fm, fm, a, r, a);
        return true;
    case ME_LIGHT_COEFF_DX:
        sb_printf(sb, "    %s = vec4(1.0, max(%s, 0.0), %s > 0.0 ? pvs_pow_ff(max(%s, 0.0), clamp(%s, -128.0, 128.0)) : 0.0, 1.0);\n",
                  r, b, b, a, c);
        return true;
    default:
        break;
    }
    const char *x;
    char buf[200];

    switch (op) {
    case ME_EXP_BASEE_FF:       snprintf(buf, sizeof(buf), "exp(%s)", a); break;
    case ME_POWER_FUNC_FF:      snprintf(buf, sizeof(buf), "pvs_pow_ff(%s, %s)", a, c); break;
    case ME_RECIP_DX:           snprintf(buf, sizeof(buf), "(%s == 0.0 ? %s : 1.0 / %s)", a, fm, a); break;
    case ME_RECIP_FF:           snprintf(buf, sizeof(buf), "(%s == 0.0 ? 0.0 : 1.0 / %s)", a, a); break;
    case ME_RECIP_SQRT_DX:      snprintf(buf, sizeof(buf), "(%s == 0.0 ? %s : 1.0 / sqrt(abs(%s)))", a, fm, a); break;
    case ME_RECIP_SQRT_FF:      snprintf(buf, sizeof(buf), "(%s == 0.0 ? 0.0 : 1.0 / sqrt(abs(%s)))", a, a); break;
    case ME_MULTIPLY:           snprintf(buf, sizeof(buf), "(%s * %s)", a, b); break;
    case ME_EXP_BASE2_FULL_DX:  snprintf(buf, sizeof(buf), "exp2(%s)", a); break;
    case ME_LOG_BASE2_FULL_DX:  snprintf(buf, sizeof(buf), "(%s == 0.0 ? -%s : log2(abs(%s)))", a, fm, a); break;
    case ME_POWER_FUNC_FF_CLAMP_B:
        snprintf(buf, sizeof(buf), "(%s < %s ? 0.0 : pvs_pow_ff(%s, %s))", a, b, a, c);
        break;
    case ME_POWER_FUNC_FF_CLAMP_B1:
        snprintf(buf, sizeof(buf), "(%s < %s ? 0.0 : %s > 1.0 ? 1.0 : pvs_pow_ff(%s, %s))", a, b, a, a, c);
        break;
    case ME_POWER_FUNC_FF_CLAMP_01:
        snprintf(buf, sizeof(buf), "(%s <= 0.0 ? 0.0 : %s > 1.0 ? 1.0 : pvs_pow_ff(%s, %s))", a, a, a, c);
        break;
    case ME_SIN:                snprintf(buf, sizeof(buf), "sin(clamp(%s, -3.14159274, 3.14159274))", a); break;
    case ME_COS:                snprintf(buf, sizeof(buf), "cos(clamp(%s, -3.14159274, 3.14159274))", a); break;
    case ME_LOG_BASE2_IEEE:     snprintf(buf, sizeof(buf), "log2(abs(%s))", a); break;
    case ME_RECIP_IEEE:         snprintf(buf, sizeof(buf), "(1.0 / %s)", a); break;
    case ME_RECIP_SQRT_IEEE:    snprintf(buf, sizeof(buf), "(1.0 / sqrt(abs(%s)))", a); break;
    case ME_NOP:                snprintf(buf, sizeof(buf), "0.0"); break;
    default:
        return false;
    }
    x = buf;
    sb_printf(sb, "    %s = vec4(%s);\n", r, x);
    return true;
}

/* Vector engine: statements that set vec4 `r` from vec4s a, b and cc. */
static bool emit_vector(Sb *sb, unsigned op, const char *r)
{
#define PER(expr) sb_printf(sb, "    %s = vec4(" expr ");\n", r)
#define E4(f) f("x"), f("y"), f("z"), f("w")
    switch (op) {
    case VE_DOT_PRODUCT:
        sb_printf(sb, "    %s = vec4(a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w);\n", r);
        return true;
    case VE_DISTANCE_VECTOR:
        sb_printf(sb, "    %s = vec4(1.0, a.y * b.y, a.z, b.w);\n", r);
        return true;
    case VE_MULTIPLY_CLAMP:
        sb_printf(sb, "    %s = vec4(cc.w < a.w * b.w ? cc.w : cc.x >= a.x * b.x ? cc.x : a.x * b.x);\n", r);
        return true;
    case VE_NOP:                    PER("0.0"); return true;
    case VE_MULTIPLY:               sb_printf(sb, "    %s = a * b;\n", r); return true;
    case VE_ADD:                    sb_printf(sb, "    %s = a + b;\n", r); return true;
    case VE_MULTIPLY_ADD:           sb_printf(sb, "    %s = a * b + cc;\n", r); return true;
    case VE_FRACTION:               sb_printf(sb, "    %s = a - floor(a);\n", r); return true;
    case VE_MAXIMUM:                sb_printf(sb, "    %s = max(a, b);\n", r); return true;
    case VE_MINIMUM:                sb_printf(sb, "    %s = min(a, b);\n", r); return true;
    case VE_SET_GREATER_THAN_EQUAL: sb_printf(sb, "    %s = vec4(greaterThanEqual(a, b));\n", r); return true;
    case VE_SET_LESS_THAN:          sb_printf(sb, "    %s = vec4(lessThan(a, b));\n", r); return true;
    case VE_MULTIPLYX2_ADD:         sb_printf(sb, "    %s = 2.0 * (a * b) + cc;\n", r); return true;
    case VE_FLT2FIX_DX:             sb_printf(sb, "    %s = floor(a);\n", r); return true;
    case VE_FLT2FIX_DX_RND:         sb_printf(sb, "    %s = floor(a + 0.5);\n", r); return true;
    case VE_COND_MUX_EQ:
        sb_printf(sb, "    %s = vec4(a.x == 0.0 ? b.x : cc.x, a.y == 0.0 ? b.y : cc.y, a.z == 0.0 ? b.z : cc.z, a.w == 0.0 ? b.w : cc.w);\n", r);
        return true;
    case VE_COND_MUX_GT:
        sb_printf(sb, "    %s = vec4(a.x > 0.0 ? b.x : cc.x, a.y > 0.0 ? b.y : cc.y, a.z > 0.0 ? b.z : cc.z, a.w > 0.0 ? b.w : cc.w);\n", r);
        return true;
    case VE_COND_MUX_GTE:
        sb_printf(sb, "    %s = vec4(a.x >= 0.0 ? b.x : cc.x, a.y >= 0.0 ? b.y : cc.y, a.z >= 0.0 ? b.z : cc.z, a.w >= 0.0 ? b.w : cc.w);\n", r);
        return true;
    case VE_SET_GREATER_THAN:       sb_printf(sb, "    %s = vec4(greaterThan(a, b));\n", r); return true;
    case VE_SET_EQUAL:              sb_printf(sb, "    %s = vec4(equal(a, b));\n", r); return true;
    case VE_SET_NOT_EQUAL:          sb_printf(sb, "    %s = vec4(notEqual(a, b));\n", r); return true;
    default:
        return false;
    }
#undef E4
#undef PER
}

/* A write of vec4 `r` under the destination fields of d0. */
static void emit_write(Sb *sb, uint32_t d0, const char *r)
{
    unsigned type = (d0 >> 8) & 0xF;
    unsigned mask = (d0 >> 20) & 0xF;
    unsigned mode = ((d0 >> 31) & 1) | (((d0 >> 12) & 1) << 1);
    unsigned offset = (d0 >> 13) & 0x7F;
    const char *arr;
    unsigned lim;
    bool repl = type == DST_OUT_REPL_X;

    switch (type) {
    case DST_A0:
        for (int i = 0; i < 4; i++) {
            if (mask & (1u << i)) {
                sb_printf(sb, "    a0.%c = int(clamp(floor(%s.%c), -256.0, 255.0));\n", comp[i], r, comp[i]);
            }
        }
        return;
    case DST_TEMP:     arr = "T"; lim = R300_PVS_NUM_TEMPS; break;
    case DST_OUT:
    case DST_OUT_REPL_X: arr = "OUT"; lim = R300_PVS_NUM_OUTPUTS; break;
    case DST_ALT:      arr = "A"; lim = R300_PVS_NUM_ALT_TEMPS; break;
    default:
        return;
    }
    if (!mask) {
        return;
    }
    if (mode == 0) {
        if (offset >= lim) {
            return;
        }
        for (int i = 0; i < 4; i++) {
            if (mask & (1u << i)) {
                sb_printf(sb, "    %s[%u].%c = %s.%c;\n", arr, offset, comp[i], r, comp[repl ? 0 : i]);
            }
        }
        return;
    }
    /* relative destination: A0.sel (mode 1) or the loop index (0 here) */
    if (mode == 1) {
        sb_printf(sb, "    { int di_ = %u + a0[%u]; if (di_ >= 0 && di_ < %u) {\n", offset, (d0 >> 29) & 3u, lim);
    } else {
        sb_printf(sb, "    { int di_ = %u; if (di_ >= 0 && di_ < %u) {\n", offset, lim);
    }
    for (int i = 0; i < 4; i++) {
        if (mask & (1u << i)) {
            sb_printf(sb, "        %s[di_].%c = %s.%c;\n", arr, comp[i], r, comp[repl ? 0 : i]);
        }
    }
    sb_printf(sb, "    } }\n");
}

static const char prelude[] =
    "float pvs_pow_ff(float base, float e)\n"
    "{\n"
    "    if (base == 0.0) return e < 0.0 ? uintBitsToFloat(0x7f800000u) : 0.0;\n"
    "    if (e == 0.0) return 1.0;\n"
    "    if (base < 0.0) return -pow(-base, e);\n"
    "    return pow(base, e);\n"
    "}\n";

char *r300_pvs_to_glsl(const R300PVSProgram *prog, uint32_t *unsup)
{
    Sb sb = { 0 };
    uint32_t bad = 0;

    if (prog->fc_opc) {
        bad |= R300_PVS_GLSL_FLOW;
    }
    sb_printf(&sb, "%s", prelude);
    sb_printf(&sb, "void r300_pvs(vec4 IN[%d], inout vec4 OUT[%d])\n{\n", R300_PVS_NUM_INPUTS, R300_PVS_NUM_OUTPUTS);
    if (prog->ntemps) {
        sb_printf(&sb, "    vec4 T[%u];\n    for (int i_ = 0; i_ < %u; i_++) { T[i_] = vec4(0.0); }\n",
                  prog->ntemps, prog->ntemps);
    }
    if (prog->uses_alt) {
        sb_printf(&sb, "    vec4 A[%d];\n    for (int i_ = 0; i_ < %d; i_++) { A[i_] = vec4(0.0); }\n",
                  R300_PVS_NUM_ALT_TEMPS, R300_PVS_NUM_ALT_TEMPS);
    }
    sb_printf(&sb, "    ivec4 a0 = ivec4(0);\n");

    for (unsigned pc = prog->first_inst; !bad && pc <= prog->last_inst && pc < R300_PVS_MAX_INSTS; pc++) {
        const R300PVSInst *ins = &prog->dec[pc];
        uint32_t d0 = ins->d[0];
        unsigned op = d0 & 0x3F;
        bool math = (d0 >> 6) & 1;
        bool macro = (d0 >> 7) & 1;
        bool dual = !math && !macro && ((d0 >> 28) & 1);

        if ((d0 >> 26) & 1) {
            bad |= R300_PVS_GLSL_PRED;
            break;
        }
        sb_printf(&sb, "    { /* %u */\n", pc);
        emit_src(&sb, prog, &ins->src[0], "a");
        emit_src(&sb, prog, &ins->src[1], "b");
        if (dual) {
            sb_printf(&sb, "    vec4 cc = vec4(0.0);\n");
        } else {
            emit_src(&sb, prog, &ins->src[2], "cc");
        }
        sb_printf(&sb, "    vec4 r;\n");
        if (math) {
            if (!emit_math(&sb, op, "a.w", "b.w", "cc.w", "r")) {
                bad |= R300_PVS_GLSL_OPCODE;
                break;
            }
            if ((d0 >> 25) & 1) {
                sb_printf(&sb, "    r = clamp(r, 0.0, 1.0);\n");
            }
            emit_write(&sb, d0, "r");
        } else {
            unsigned vop = macro ? ((op & 1) ? VE_MULTIPLYX2_ADD : VE_MULTIPLY_ADD) : op;

            if (!emit_vector(&sb, vop, "r")) {
                bad |= R300_PVS_GLSL_OPCODE;
                break;
            }
            if ((d0 >> 24) & 1) {
                sb_printf(&sb, "    r = clamp(r, 0.0, 1.0);\n");
            }
            if (dual) {
                /* The third source dword describes a math op writing ATRM 0-3. */
                uint32_t s = ins->d[3];
                unsigned mop = ((s >> 21) & 0xF) | (((s >> 2) & 1) << 4);
                char reg[160];
                char xe[48], ye[48];
                unsigned xs = (s >> 13) & 7, ys = (s >> 16) & 7;

                reg_expr(reg, sizeof(reg), prog, &ins->src[2]);
                sb_printf(&sb, "    vec4 dv = %s;\n    vec4 mr;\n", reg);
                snprintf(xe, sizeof(xe), "%s", xs < 4 ? "" : xs == 5 ? "1.0" : "0.0");
                snprintf(ye, sizeof(ye), "%s", ys < 4 ? "" : ys == 5 ? "1.0" : "0.0");
                if (xs < 4) {
                    snprintf(xe, sizeof(xe), "dv.%c", comp[xs]);
                }
                if (ys < 4) {
                    snprintf(ye, sizeof(ye), "dv.%c", comp[ys]);
                }
                sb_printf(&sb, "    float dx = %s%s%s, dy = %s%s%s;\n",
                          (s & (1u << 3)) ? "abs(" : "(", xe, ")",
                          (s & (1u << 3)) ? "abs(" : "(", ye, ")");
                if (s & (1u << 25)) {
                    sb_printf(&sb, "    dx = -dx;\n");
                }
                if (s & (1u << 26)) {
                    sb_printf(&sb, "    dy = -dy;\n");
                }
                if (!emit_math(&sb, mop, "dx", "dy", "dy", "mr")) {
                    bad |= R300_PVS_GLSL_OPCODE;
                    break;
                }
                if ((d0 >> 25) & 1) {
                    sb_printf(&sb, "    mr = clamp(mr, 0.0, 1.0);\n");
                }
                emit_write(&sb, d0, "r");    /* vector result first... */
                sb_printf(&sb, "    A[%u].%c = mr.%c;\n", (s >> 19) & 3, comp[(s >> 27) & 3], comp[(s >> 27) & 3]);
            } else {
                emit_write(&sb, d0, "r");
            }
        }
        sb_printf(&sb, "    }\n");
    }
    sb_printf(&sb, "}\n");
    if (bad) {
        free(sb.p);
        if (unsup) {
            *unsup = bad;
        }
        return NULL;
    }
    if (unsup) {
        *unsup = 0;
    }
    return sb.p;
}
