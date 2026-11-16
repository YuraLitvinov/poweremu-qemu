/*
 * R300-family draw assembly.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#include "r300_draw.h"
#include "r300_pvs_glsl.h"

#include <math.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "r300_pvs.h"

/* Registers used here */
#define SE_VPORT_XSCALE             0x1D98  /* then XOFFSET, YSCALE, ... ZOFFSET */
#define GB_ENABLE                   0x4008
#define GA_ROUND_MODE              0x428C  /* bit 4 RGB / bit 5 alpha: 1 = FP20 range, 0 = clamp to [0, 1] */
#define GB_MSPOS0                   0x4010
#define GB_MSPOS1                   0x4014
#define GB_SELECT                   0x401C
#define GB_AA_CONFIG                0x4020
#define VAP_CLIP_CNTL               0x221C
#define VAP_PVS_FLOW_CNTL_ADDRS_0   0x2230
#define VAP_PVS_FLOW_CNTL_LOOP_INDEX_0 0x2290
#define VAP_PVS_FLOW_CNTL_OPC       0x22DC
#define VAP_CNTL                    0x2080
#define VAP_CNTL_STATUS             0x2140
#define VAP_OUTPUT_VTX_FMT_0        0x2090
#define VAP_OUTPUT_VTX_FMT_1        0x2094
#define VAP_VTE_CNTL                0x20B0
#define VAP_VTX_SIZE                0x20B4
#define VAP_PROG_STREAM_CNTL_0      0x2150
#define VAP_PROG_STREAM_CNTL_EXT_0  0x21E0
#define VAP_PVS_CODE_CNTL_0         0x22D0
#define VAP_PVS_CONST_CNTL          0x22D4
#define TX_ENABLE                   0x4104
#define RS_COUNT                    0x4300
#define RS_INST_COUNT               0x4304
#define RS_IP_0                     0x4310
#define RS_INST_0                   0x4330
#define SC_CLIPRECT_TL_0            0x43B0  /* then BR_0, TL_1, BR_1, ... */
#define SC_CLIP_RULE                0x43D0
#define SC_SCISSORS_TL              0x43E0
#define SC_SCISSORS_BR              0x43E4
#define TX_FILTER0_0                0x4400
#define TX_FILTER1_0                0x4440
#define TX_FORMAT0_0                0x4480
#define TX_FORMAT1_0                0x44C0
#define TX_FORMAT2_0                0x4500
#define TX_OFFSET_0                 0x4540
#define TX_BORDER_COLOR_0           0x45C0
#define US_OUT_FMT_0                0x46A4
#define FG_FOG_BLEND                0x4BC0
#define FG_FOG_FACTOR               0x4BC4
#define FG_FOG_COLOR_R              0x4BC8  /* then G, B */
#define FG_ALPHA_FUNC               0x4BD4
#define FG_DEPTH_SRC                0x4BD8
#define PFS_PARAM_0_X               0x4C00
#define RB3D_CBLEND                 0x4E04
#define RB3D_ABLEND                 0x4E08
#define RB3D_COLOR_CHANNEL_MASK     0x4E0C
#define RB3D_BLEND_COLOR            0x4E10
#define RB3D_ROPCNTL                0x4E18
#define RB3D_COLOROFFSET0           0x4E28
#define RB3D_COLORPITCH0            0x4E38
#define SU_POLY_OFFSET_FRONT_SCALE  0x42A4  /* then FRONT_OFFSET, BACK_SCALE, BACK_OFFSET */
#define SU_POLY_OFFSET_ENABLE       0x42B4
#define SU_CULL_MODE                0x42B8
#define ZB_CNTL                     0x4F00
#define ZB_ZSTENCILCNTL             0x4F04
#define ZB_STENCILREFMASK           0x4F08
#define ZB_FORMAT                   0x4F10
#define ZB_DEPTHOFFSET              0x4F20
#define ZB_DEPTHPITCH               0x4F24
#define GA_POINT_S0                 0x4200  /* then T0, S1, T1 */
#define GA_POINT_T0                 0x4204
#define GA_POINT_S1                 0x4208
#define GA_POINT_T1                 0x420C
#define GA_POINT_SIZE               0x421C
#define GA_POINT_MINMAX             0x4230
#define GA_LINE_CNTL                0x4234
#define GA_LINE_STIPPLE_CONFIG      0x4238
#define GA_LINE_STIPPLE_VALUE       0x4260
#define GA_LINE_S0                  0x4264
#define GA_LINE_S1                  0x4268
#define GA_COLOR_CONTROL            0x4278
#define GA_SOLID_RG                 0x427C
#define GA_SOLID_BA                 0x4280
#define GA_POLY_MODE                0x4288
#define GA_FOG_SCALE                0x4294
#define GA_FOG_OFFSET               0x4298

#define SC_COORD_BIAS               1440    /* scissor/cliprect origin */
#define CLIP_MIN(a, b)              ((a) < (b) ? (a) : (b))
#define CLIP_MAX(a, b)              ((a) > (b) ? (a) : (b))

static float bits_to_float(uint32_t v)
{
    float f;
    memcpy(&f, &v, 4);
    return f;
}

static uint32_t vc_swap(uint32_t v, unsigned mode)
{
    switch (mode & 3) {
    case 1:  return ((v & 0x00FF00FFu) << 8) | ((v >> 8) & 0x00FF00FFu);
    case 2:  return __builtin_bswap32(v);
    case 3:  return (v << 16) | (v >> 16);
    default: return v;
    }
}

static float half_to_float(uint16_t h)
{
    uint32_t s = (h >> 15) & 1, e = (h >> 10) & 0x1F, m = h & 0x3FF;
    float f;

    if (e == 0) {
        f = ldexpf((float)m, -24);
    } else if (e == 31) {
        f = m ? NAN : INFINITY;
    } else {
        f = ldexpf(1.0f + m / 1024.0f, (int)e - 15);
    }
    return s ? -f : f;
}

void r300_load_vbpntr(R300Arrays *arr, const uint32_t *d, uint32_t ndw)
{
    unsigned n, i;

    memset(arr, 0, sizeof(*arr));
    if (ndw < 1) {
        return;
    }
    n = d[0] & 0x1F;
    for (i = 0; i < n && i < R300_MAX_ARRAYS; i++) {
        unsigned pair = i / 2, half = i % 2;
        unsigned base = 1 + pair * 3;
        uint32_t fmt;

        if (base + 1 + half >= ndw) {
            break;
        }
        fmt = d[base] >> (16 * half);
        arr->a[i].size_dw = fmt & 0x7F;
        arr->a[i].stride_dw = (fmt >> 8) & 0x7F;
        arr->a[i].addr = d[base + 1 + half];
    }
    arr->count = i;
}

/* ---- vertex fetch --------------------------------------------------- */

typedef struct Fetch {
    const R300State *st;
    const R300Arrays *arr;
    R300ReadFn read;
    void *opaque;
    const uint32_t *immd;       /* immediate vertex data, or NULL */
    uint32_t immd_dw;
    uint32_t vtx_size;          /* VAP_VTX_SIZE, dwords */
    unsigned swap;              /* VAP_CNTL_STATUS.VC_SWAP */
    uint32_t warn;
    const char *why;            /* first fetch failure */
} Fetch;

static unsigned psc_dwords(unsigned type)
{
    static const unsigned n[16] = { 1, 2, 3, 4, 1, 1, 1, 2, 1, 1, 8, 1, 2, 0, 0, 0 };
    return n[type & 15];
}

/* Fetch dwords for PSC element k of vertex vtx starting at *cursor. */
static bool fetch_dwords(Fetch *f, unsigned k, uint32_t vtx, unsigned cursor,
                         unsigned count, uint32_t *out)
{
    if (f->immd) {
        uint64_t at = (uint64_t)vtx * f->vtx_size + cursor;
        if (at + count > f->immd_dw) {
            f->why = "immediate vertex data shorter than VAP_VTX_SIZE says";
            return false;
        }
        memcpy(out, &f->immd[at], count * 4);   /* CP data: already in order */
        return true;
    }
    if (!f->arr->count) {
        f->why = "vertex element with no 3D_LOAD_VBPNTR array";
        return false;
    }
    /* More elements than arrays: the rest are interleaved in the last one
     * (fetch_vertex keeps the cursor running for them). */
    if (k >= f->arr->count) {
        k = f->arr->count - 1;
    }
    uint32_t addr = f->arr->a[k].addr +
                    (vtx * f->arr->a[k].stride_dw + cursor) * 4;
    if (!f->read(f->opaque, addr, out, count * 4)) {
        f->why = "vertex array outside VRAM and the GART";
        return false;
    }
    for (unsigned i = 0; i < count; i++) {
        out[i] = vc_swap(out[i], f->swap);
    }
    return true;
}

static void decode_element(unsigned type, bool sgn, bool norm,
                           const uint32_t *w, float c[4])
{
    c[0] = c[1] = c[2] = 0.0f;
    c[3] = 1.0f;
    switch (type) {
    case 0: case 1: case 2: case 3:         /* FLOAT_1..4 */
        for (unsigned i = 0; i <= type; i++) {
            c[i] = bits_to_float(w[i]);
        }
        break;
    case 4:                                 /* BYTE x4 */
    case 5:                                 /* D3DCOLOR */
        for (unsigned i = 0; i < 4; i++) {
            unsigned b = (w[0] >> (8 * i)) & 0xFF;
            float x = sgn ? (float)(int8_t)b : (float)b;
            if (norm || type == 5) {
                x = sgn ? fmaxf(x / 127.0f, -1.0f) : x / 255.0f;
            }
            c[i] = x;
        }
        if (type == 5) {                    /* stored B,G,R,A */
            float t = c[0];
            c[0] = c[2];
            c[2] = t;
        }
        break;
    case 6:                                 /* SHORT_2 */
    case 7:                                 /* SHORT_4 */
        for (unsigned i = 0; i < (type == 6 ? 2u : 4u); i++) {
            unsigned h = (w[i / 2] >> (16 * (i % 2))) & 0xFFFF;
            float x = sgn ? (float)(int16_t)h : (float)h;
            if (norm) {
                x = sgn ? fmaxf(x / 32767.0f, -1.0f) : x / 65535.0f;
            }
            c[i] = x;
        }
        break;
    case 8:                                 /* VECTOR_3_TTT: 10:10:10 */
    case 9: {                               /* VECTOR_3_EET: 11:11:10 */
        static const uint8_t sh[2][3] = { { 0, 10, 20 }, { 0, 11, 22 } };
        static const uint8_t nb[2][3] = { { 10, 10, 10 }, { 11, 11, 10 } };
        for (unsigned i = 0; i < 3; i++) {
            unsigned n = nb[type - 8][i];
            uint32_t b = (w[0] >> sh[type - 8][i]) & ((1u << n) - 1);
            float x = sgn ? (float)((int32_t)(b << (32 - n)) >> (32 - n)) : (float)b;
            if (norm) {
                float m = (float)((1u << (sgn ? n - 1 : n)) - 1);
                x = sgn ? fmaxf(x / m, -1.0f) : x / m;
            }
            c[i] = x;
        }
        break;
    }
    case 11:                                /* FLT16_2 */
    case 12:                                /* FLT16_4 */
        for (unsigned i = 0; i < (type == 11 ? 2u : 4u); i++) {
            c[i] = half_to_float((w[i / 2] >> (16 * (i % 2))) & 0xFFFF);
        }
        break;
    default:
        break;
    }
}

/* PSC element k's EXT swizzle and write mask into one input vector. */
static void psc_store(unsigned x, const float c[4], float *dst)
{
    for (unsigned i = 0; i < 4; i++) {
        if ((x >> (12 + i)) & 1) {
            unsigned s = (x >> (3 * i)) & 7;
            dst[i] = s < 4 ? c[s] : s == 5 ? 1.0f : 0.0f;
        }
    }
}

/* Fill the input vertex memory for one vertex through the PSC. */
static bool fetch_vertex(Fetch *f, uint32_t vtx, float in[R300_PVS_NUM_INPUTS][4])
{
    unsigned cursor = 0;

    memset(in, 0, sizeof(float) * 4 * R300_PVS_NUM_INPUTS);
    for (unsigned k = 0; k < 16; k++) {
        uint32_t reg = r300_reg(f->st, VAP_PROG_STREAM_CNTL_0 + 4 * (k / 2));
        uint32_t ext = r300_reg(f->st, VAP_PROG_STREAM_CNTL_EXT_0 + 4 * (k / 2));
        unsigned e = (reg >> (16 * (k % 2))) & 0xFFFF;
        unsigned x = (ext >> (16 * (k % 2))) & 0xFFFF;
        unsigned type = e & 0xF, skip = (e >> 4) & 0xF, dst = (e >> 8) & 0x1F;
        unsigned n = psc_dwords(type);
        uint32_t w[8];
        float c[4];

        if (!f->immd && k < f->arr->count) {
            cursor = 0;                 /* each array starts afresh */
        }
        cursor += skip;
        if (!n) {
            f->warn |= R300_WARN_VTXFMT;
        } else if (!fetch_dwords(f, k, vtx, cursor, n, w)) {
            return false;
        } else if (type == 10) {
            /* FLOAT_8: two FLOAT_4 vectors at DST_VEC_LOC and the next */
            decode_element(3, false, false, w, c);
            psc_store(x, c, in[dst]);
            decode_element(3, false, false, w + 4, c);
            psc_store(x, c, in[(dst + 1) % R300_PVS_NUM_INPUTS]);
        } else {
            decode_element(type, (e >> 14) & 1, (e >> 15) & 1, w, c);
            psc_store(x, c, in[dst]);
        }
        cursor += n;
        if (e & (1u << 13)) {           /* LAST_VEC */
            break;
        }
    }
    return true;
}

/* ---- output routing ------------------------------------------------- */

#define LAY_GEN 0xFF

typedef struct Layout {
    int pos, psize, color[4];
    int tex_slot[8];
    unsigned tex_n[8];
    /* texture interpolants as one scalar list: (slot, component); a unit whose GB_ENABLE source is ST and that has no
     * vertex components contributes the rasterizer's own (s, t) (point sprite, line stipple) at its place in the list:
     * slot LAY_GEN, component 0 or 1 */
    unsigned nscal;
    uint8_t scal_slot[32], scal_comp[32];
} Layout;

static void output_layout(const R300State *st, Layout *l)
{
    uint32_t f0 = r300_reg(st, VAP_OUTPUT_VTX_FMT_0);
    uint32_t f1 = r300_reg(st, VAP_OUTPUT_VTX_FMT_1);
    int slot = 0;

    memset(l, -1, sizeof(*l));
    l->nscal = 0;
    l->pos = slot++;                    /* always present */
    if (f0 & (1u << 16)) {
        l->psize = slot++;              /* point size (x) */
    }
    for (int c = 0; c < 4; c++) {
        if (f0 & (1u << (1 + c))) {
            l->color[c] = slot++;
        }
    }
    for (int t = 0; t < 8; t++) {
        unsigned n = (f1 >> (3 * t)) & 7;
        l->tex_n[t] = n;
        if (n) {
            l->tex_slot[t] = slot++;
            for (unsigned i = 0; i < n && l->nscal < 32; i++) {
                l->scal_slot[l->nscal] = l->tex_slot[t];
                l->scal_comp[l->nscal++] = i;
            }
        } else if (((r300_reg(st, GB_ENABLE) >> (16 + 2 * t)) & 3) == 1) {
            for (unsigned i = 0; i < 2 && l->nscal < 32; i++) {
                l->scal_slot[l->nscal] = LAY_GEN;
                l->scal_comp[l->nscal++] = i;
            }
        }
    }
}

typedef struct Route {
    unsigned nvary;
    struct {
        bool is_tex;
        unsigned ip;
    } vary[R300_NUM_VARYINGS];
} Route;

/* RS_INST -> varyings and the temp routing for the fragment program. */
static void rs_route(const R300State *st, Route *r, R300FSDesc *desc,
                     uint32_t *warn)
{
    unsigned count = (r300_reg(st, RS_INST_COUNT) & 0xF) + 1;

    memset(r, 0, sizeof(*r));
    memset(desc->route, -1, sizeof(desc->route));
    for (unsigned i = 0; i < count; i++) {
        uint32_t inst = r300_reg(st, RS_INST_0 + 4 * i);

        for (int pass = 0; pass < 2; pass++) {
            bool tex = pass == 0;
            bool write = tex ? (inst >> 3) & 1 : (inst >> 14) & 1;
            unsigned ip = tex ? inst & 7 : (inst >> 11) & 7;
            unsigned addr = tex ? (inst >> 6) & 0x1F : (inst >> 17) & 0x1F;

            if (!write) {
                continue;
            }
            if (r->nvary == R300_NUM_VARYINGS) {
                *warn |= R300_WARN_VARYINGS;
                continue;
            }
            r->vary[r->nvary].is_tex = tex;
            r->vary[r->nvary].ip = ip;
            desc->route[addr] = r->nvary++;
        }
    }
}

static float rs_col_fmt(unsigned fmt, const float *c, unsigned i)
{
    static const int8_t tbl[16][4] = {
        /* per component: 0-3 = channel, -1 = 0.0, -2 = 1.0 */
        [0] = { 0, 1, 2, 3 },  [1] = { 0, 1, 2, -1 }, [2] = { 0, 1, 2, -2 },
        [4] = { -1, -1, -1, 3 }, [5] = { -1, -1, -1, -1 }, [6] = { -1, -1, -1, -2 },
        [8] = { -2, -2, -2, 3 }, [9] = { -2, -2, -2, -1 }, [10] = { -2, -2, -2, -2 },
    };
    int s = tbl[fmt & 15][i];
    return s >= 0 ? c[s] : s == -2 ? 1.0f : 0.0f;
}

/* The four rasterized colours of a vertex (RS colour pointer 0-3). */
static void vtx_colors(const Layout *l, float out[][4], float cols[4][4])
{
    for (int cp = 0; cp < 4; cp++) {
        if (l->color[cp] >= 0) {
            memcpy(cols[cp], out[l->color[cp]], sizeof(cols[cp]));
        } else {
            memset(cols[cp], 0, sizeof(cols[cp]));
        }
    }
}

/*
 * Interpolant values of one vertex: texture coordinates from the output
 * vertex, colours from cols (vtx_colors, or substituted for flat shading
 * and back faces), and the rasterizer's generated (s, t) (point sprites,
 * line stipple) past the vertex's own texture components.
 */
static void rs_values(const R300State *st, const Layout *l, const Route *r,
                      float out[][4], const float (*cols)[4],
                      const float *sprite, float v[R300_NUM_VARYINGS][4])
{
    for (unsigned k = 0; k < r->nvary; k++) {
        uint32_t ip = r300_reg(st, RS_IP_0 + 4 * r->vary[k].ip);

        if (r->vary[k].is_tex) {
            unsigned ptr = ip & 0x3F;
            for (unsigned i = 0; i < 4; i++) {
                unsigned sel = (ip >> (13 + 3 * i)) & 7;
                float x = 0.0f;
                if (sel < 4) {
                    unsigned s = ptr + sel;
                    if (s < l->nscal && l->scal_slot[s] == LAY_GEN) {
                        x = sprite ? sprite[l->scal_comp[s]] : 0.0f;
                    } else if (s < l->nscal) {
                        x = out[l->scal_slot[s]][l->scal_comp[s]];
                    } else if (sprite) {
                        /* Point sprite: the rasterizer generates (s, t). */
                        unsigned c = s - l->nscal;
                        x = c < 2 ? sprite[c] : c == 3 ? 1.0f : 0.0f;
                    }
                } else if (sel == 5) {
                    x = 1.0f;
                }
                v[k][i] = x;
            }
        } else {
            static const float zero[4];
            unsigned cp = (ip >> 6) & 7;
            const float *c = cp < 4 ? cols[cp] : zero;
            float cc[4];
            uint32_t rm = r300_reg(st, GA_ROUND_MODE);

            /* vertex colours are clamped to [0, 1] unless GA_ROUND_MODE selects the FP20 range (GL_CLAMP_VERTEX_COLOR) */
            for (unsigned i = 0; i < 4; i++) {
                bool keep = (rm >> (i < 3 ? 4 : 5)) & 1;
                cc[i] = keep ? c[i] : fminf(fmaxf(c[i], 0.0f), 1.0f);
            }
            c = cc;
            for (unsigned i = 0; i < 4; i++) {
                v[k][i] = rs_col_fmt((ip >> 9) & 0xF, c, i);
            }
        }
    }
}

/* ---- draw ----------------------------------------------------------- */

static void set_uniforms(const R300State *st, R300DrawPacket *pkt)
{
    R300FSUniforms *u = &pkt->uniforms;
    uint32_t outfmt = r300_reg(st, US_OUT_FMT_0);
    uint32_t bc = r300_reg(st, RB3D_BLEND_COLOR);

    memset(u, 0, sizeof(*u));
    for (int i = 0; i < R300_US_NUM_CONSTS; i++) {
        for (int c = 0; c < 4; c++) {
            u->consts[i][c] = r300_float24(r300_reg(st, PFS_PARAM_0_X + 16 * i + 4 * c));
        }
    }
    /*
     * RB3D_BLEND_COLOR byte n belongs to stored component n of the colour buffer, i.e. to the channel US_OUT_FMT's
     * select n names (0 A, 1 R, 2 G, 3 B); Mesa pre-swaps the colour for buffers that are not BGRA.  Fixed-size work.
     */
    {
        uint32_t ofr = r300_reg(st, US_OUT_FMT_0);

        for (int n = 0; n < 4; n++) {
            unsigned sel = (ofr >> (8 + 2 * n)) & 3;

            u->blend_color[sel == 0 ? 3 : sel - 1] = ((bc >> (8 * n)) & 0xFF) / 255.0f;
        }
    }
    u->cblend = r300_reg(st, RB3D_CBLEND);
    u->ablend = r300_reg(st, RB3D_ABLEND);
    u->chanmask = r300_reg(st, RB3D_COLOR_CHANNEL_MASK) & 0xF;
    u->alpha_func = r300_reg(st, FG_ALPHA_FUNC);
    for (int n = 0; n < 4; n++) {
        u->out_sel[n] = (outfmt >> (8 + 2 * n)) & 3;
    }
    u->rt_swap32 = ((r300_reg(st, RB3D_COLORPITCH0) >> 19) & 3) == 2;
    u->rt_endian = (r300_reg(st, RB3D_COLORPITCH0) >> 19) & 3;
    u->clip_rule = r300_reg(st, SC_CLIP_RULE) & 0xFFFF;
    for (int i = 0; i < 4; i++) {
        uint32_t tl = r300_reg(st, SC_CLIPRECT_TL_0 + 8 * i);
        uint32_t br = r300_reg(st, SC_CLIPRECT_TL_0 + 8 * i + 4);
        u->cliprect[i][0] = (int32_t)(tl & 0x1FFF) - SC_COORD_BIAS;
        u->cliprect[i][1] = (int32_t)((tl >> 13) & 0x1FFF) - SC_COORD_BIAS;
        u->cliprect[i][2] = (int32_t)(br & 0x1FFF) - SC_COORD_BIAS;
        u->cliprect[i][3] = (int32_t)((br >> 13) & 0x1FFF) - SC_COORD_BIAS;
    }
    u->rop = r300_reg(st, RB3D_ROPCNTL);
    /* Fog: colours and the constant factor are 10-bit fractions. */
    for (int c = 0; c < 3; c++) {
        u->fog_color[c] = (r300_reg(st, FG_FOG_COLOR_R + 4 * c) & 0x3FF) / 1023.0f;
    }
    u->fog_color[3] = (r300_reg(st, FG_FOG_FACTOR) & 0x3FF) / 1023.0f;
    u->fog_blend = r300_reg(st, FG_FOG_BLEND) & 7;
    u->depth_src = r300_reg(st, FG_DEPTH_SRC) & 1;
    /*
     * Polygon offset (SU_POLY_OFFSET_*): scale multiplies the depth slope
     * per 1/12-pixel subpixel and offset is in units of 2^-24 of the depth
     * range.  Mesa programs a 16-bit Z buffer's scale 16 times larger, so
     * the slope there is in 2^-20 units; set_depth adjusts for Z16.
     */
    u->poly_en = r300_reg(st, SU_POLY_OFFSET_ENABLE) & 7;
    for (int i = 0; i < 2; i++) {
        u->poly_offset[2 * i] = bits_to_float(r300_reg(st, SU_POLY_OFFSET_FRONT_SCALE + 8 * i)) / 12.0f;
        u->poly_offset[2 * i + 1] = bits_to_float(r300_reg(st, SU_POLY_OFFSET_FRONT_SCALE + 8 * i + 4)) /
                                    16777216.0f;
    }
}

static uint32_t pot_ceil(uint32_t v)
{
    uint32_t p = 1;
    while (p < v) {
        p <<= 1;
    }
    return p;
}

void r300_tex_level_dims(const R300TexDesc *td, uint32_t l, uint32_t *w,
                         uint32_t *h, uint32_t *d)
{
    *w = td->width >> l ? td->width >> l : 1;
    *h = td->height >> l ? td->height >> l : 1;
    if (td->dim == R300_TEXDIM_CUBE) {
        *d = 6;
    } else if (td->dim == R300_TEXDIM_3D) {
        *d = td->depth >> l ? td->depth >> l : 1;
    } else {
        *d = 1;
    }
}

/* The hardware's addressing, as Mesa's r300_texture_desc.c lays out
 * linear (untiled) textures: 32-byte rows, power-of-two heights for
 * mipmapped, 3D and cube textures, levels one after another. */
void r300_tex_layout(R300TexDesc *td, uint32_t bpp, bool dxt, bool pitch_en)
{
    bool pot_rows = td->levels > 1 || td->dim != R300_TEXDIM_2D;
    uint32_t off = 0;

    for (uint32_t l = 0; l < td->levels; l++) {
        uint32_t w, h, d, pitch, rows;

        r300_tex_level_dims(td, l, &w, &h, &d);
        if (pot_rows) {
            h = pot_ceil(h);
        }
        if (dxt) {
            pitch = (((w + 3) / 4) * bpp + 31) & ~31u;
            rows = (h + 3) / 4;
        } else {
            /* Linear textures: rows padded to 32 bytes (Mesa's rule).  Micro-tiled ones (TX_OFFSET bit 3) as
             * a little-endian guest's card lays them out, measured: 2x2, 3x2, 4x4 -> 16 bytes; 8x8 -> 32; 16x16
             * -> 64, so rows are aligned to 16 bytes. */
            /* The micro tile is 8 pixels wide for 8bpp (8 bytes) and 16 bytes wide for 16..64bpp. */
            uint32_t mt = bpp == 1 ? 7u : 15u;
            bool square = GUEST_LE && (td->tile & 0x10);   /* TX_OFFSET micro tile 2: 4x4 pixels (16bpp) */
            pitch = (GUEST_LE && (td->tile & 0x8)) ? (w * bpp + mt) & ~mt
                                                 : (w * bpp + 31) & ~31u;
            if (square) {
                pitch = (w * bpp + 4 * bpp - 1) & ~(4 * bpp - 1);
            }
            rows = h;
            if (square) {
                rows = (rows + 3) & ~3u;
            } else if (GUEST_LE && (td->tile & 0x8)) {
                /* The driver also rounds the height up to the micro tile's height (r300_get_pixel_alignment:
                 * 4 rows for 8bpp, 2 for 16..64bpp), so a 1-row level takes 2 rows and every level after it
                 * sits that much further on. */
                uint32_t th = bpp == 1 ? 4 : bpp <= 8 ? 2 : 1;
                rows = (rows + th - 1) & ~(th - 1);
            }
        }
        if (l == 0 && pitch_en) {
            pitch = td->pitch_bytes;
        }
        td->lvl_off[l] = off;
        td->lvl_pitch[l] = pitch;
        td->lvl_rows[l] = rows;
        off += pitch * rows * d;
    }
    td->size_bytes = off;
}

static float un_bits(uint32_t v, unsigned sh, unsigned n)
{
    return (float)((v >> sh) & ((1u << n) - 1)) / (float)((1u << n) - 1);
}

static float sn_bits(uint32_t v, unsigned sh, unsigned n)
{
    int32_t x = (int32_t)(((v >> sh) & ((1u << n) - 1)) << (32 - n)) >> (32 - n);
    return fmaxf((float)x / (float)((1u << (n - 1)) - 1), -1.0f);
}

void r300_border_color(uint32_t fmt, uint32_t v, float c[4])
{
    c[0] = c[1] = c[2] = c[3] = 0.0f;
    switch (fmt & 0x1F) {
    case 0x00: c[0] = un_bits(v, 0, 8); break;
    case 0x01: c[0] = un_bits(v, 0, 16); break;
    case 0x02: c[0] = un_bits(v, 0, 4); c[1] = un_bits(v, 4, 4); break;
    case 0x03: c[0] = un_bits(v, 0, 8); c[1] = un_bits(v, 8, 8); break;
    case 0x04: c[0] = un_bits(v, 0, 16); c[1] = un_bits(v, 16, 16); break;
    case 0x05: c[0] = un_bits(v, 0, 2); c[1] = un_bits(v, 2, 3); c[2] = un_bits(v, 5, 3); break;
    case 0x06: c[0] = un_bits(v, 0, 5); c[1] = un_bits(v, 5, 6); c[2] = un_bits(v, 11, 5); break;
    case 0x07: c[0] = un_bits(v, 0, 5); c[1] = un_bits(v, 5, 5); c[2] = un_bits(v, 10, 6); break;
    case 0x08: c[0] = un_bits(v, 0, 10); c[1] = un_bits(v, 10, 11); c[2] = un_bits(v, 21, 11); break;
    case 0x09: c[0] = un_bits(v, 0, 11); c[1] = un_bits(v, 11, 11); c[2] = un_bits(v, 22, 10); break;
    case 0x0A:
        for (int i = 0; i < 4; i++) c[i] = un_bits(v, 4 * i, 4);
        break;
    case 0x0B:
        for (int i = 0; i < 3; i++) c[i] = un_bits(v, 5 * i, 5);
        c[3] = un_bits(v, 15, 1);
        break;
    case 0x0D:
        for (int i = 0; i < 3; i++) c[i] = un_bits(v, 10 * i, 10);
        c[3] = un_bits(v, 30, 2);
        break;
    case 0x0F: case 0x10: case 0x11:        /* DXT: B8G8R8A8 */
        c[0] = un_bits(v, 16, 8); c[1] = un_bits(v, 8, 8);
        c[2] = un_bits(v, 0, 8); c[3] = un_bits(v, 24, 8);
        break;
    case 0x12:                              /* signed formats: R8G8B8A8_SNORM */
        for (int i = 0; i < 4; i++) c[i] = sn_bits(v, 8 * i, 8);
        break;
    case 0x16: case 0x17:                   /* R16G16_SNORM */
        c[0] = sn_bits(v, 0, 16); c[1] = sn_bits(v, 16, 16);
        break;
    case 0x18: case 0x19:                   /* R16G16_FLOAT */
        c[0] = half_to_float(v & 0xFFFF); c[1] = half_to_float(v >> 16);
        break;
    case 0x1B:
        c[0] = bits_to_float(v);
        break;
    default:                                /* 8888 */
        for (int i = 0; i < 4; i++) c[i] = un_bits(v, 8 * i, 8);
        break;
    }
}

static void set_textures(const R300State *st, R300DrawPacket *pkt)
{
    uint32_t enable = r300_reg(st, TX_ENABLE);

    for (unsigned k = 0; k < R300_NUM_TEX_UNITS; k++) {
        R300TexDesc *t = &pkt->tex[k];
        R300FSUniforms *u = &pkt->uniforms;
        uint32_t f0 = r300_reg(st, TX_FORMAT0_0 + 4 * k);
        uint32_t f1 = r300_reg(st, TX_FORMAT1_0 + 4 * k);
        uint32_t f2 = r300_reg(st, TX_FORMAT2_0 + 4 * k);
        uint32_t off = r300_reg(st, TX_OFFSET_0 + 4 * k);
        uint32_t fmt = f1 & 0x1F;

        memset(t, 0, sizeof(*t));
        /* swizzle: R 14:12, G 17:15, B 20:18, A 11:9 */
        u->tex_swz[k][0] = (f1 >> 12) & 7;
        u->tex_swz[k][1] = (f1 >> 15) & 7;
        u->tex_swz[k][2] = (f1 >> 18) & 7;
        u->tex_swz[k][3] = (f1 >> 9) & 7;
        if (!(enable & (1u << k))) {
            continue;
        }
        unsigned bpp, decode = 0;
        bool dxt = false;
        bool le = GUEST_LE;
        switch (fmt) {
        case 0x0:                       /* X8 */
            t->kind = R300_TEXK_R8; bpp = 1;
            break;
        case 0x3:                       /* Y8X8: big-endian halfwords */
            t->kind = R300_TEXK_RG8; bpp = 2; decode = le ? 0 : 2;
            break;
        case 0x6: case 0x7: case 0xA: case 0xB:
            t->kind = R300_TEXK_CONVERT16; bpp = 2;
            break;
        case 0xC:                       /* W8Z8Y8X8 */
            /* VRAM holds the guest CPU's big-endian words (see r300_tex). */
            t->kind = R300_TEXK_RGBA8; bpp = 4; decode = le ? 0 : (off & 3) == 0;
            break;
        /* The card hands the shader DXT1 texels as B,G,R,A and DXT3/5 as
         * A,B,G,R (Tiger's swizzles R=Z,G=Y,B=X and R=W,G=Z,B=Y,A=X);
         * the host decodes to R,G,B,A, so reorder before the swizzle. */
        case 0xF:  t->kind = R300_TEXK_DXT1; bpp = 8;  dxt = true; decode = le ? 0 : 3; break;   /* per block */
        case 0x10: t->kind = R300_TEXK_DXT3; bpp = 16; dxt = true; decode = le ? 0 : 1; break;
        case 0x11: t->kind = R300_TEXK_DXT5; bpp = 16; dxt = true; decode = le ? 0 : 1; break;
        default:
            bpp = r300_tex_raw_bpp(fmt);
            if (!bpp) {
                pkt->warn |= R300_WARN_TEXFMT;
                continue;
            }
            t->kind = R300_TEXK_RAW;
            t->view_bpp = bpp < 4 ? 4 : bpp;
            break;
        }
        {
            static uint32_t seen[256];
            static int nseen = -1;
            if (nseen < 0) {
                nseen = getenv("PPCGPU_TEXFMT_LOG") ? 0 : 256;
            }
            if (nseen < 256) {
                uint32_t key = (fmt << 24) ^ (f0 & 0x3FFFFFF) ^ (f1 << 3) ^ (off & 0x1F);
                bool dup = false;
                for (int i = 0; i < nseen; i++) {
                    dup |= seen[i] == key;
                }
                if (!dup) {
                    seen[nseen++] = key;
                    fprintf(stderr, "TEXFMT unit %u fmt %02x kind %d %ux%u levels %u f0 %08x f1 %08x f2 %08x off %08x\n",
                            k, fmt, t->kind, (f0 & 0x7FF) + 1, ((f0 >> 11) & 0x7FF) + 1,
                            ((f0 >> 26) & 0xF) + 1, f0, f1, f2, off);
                }
            }
        }
        t->bound = true;
        t->gpu_addr = off & ~0x1Fu;
        t->le = le;
        /* a little-endian guest's words are already the card's: endian 2 undoes the big-endian view */
        t->endian = le ? 2 : off & 3;
        t->tile = off & 0x1C;
        t->width = (f0 & 0x7FF) + 1;
        t->height = ((f0 >> 11) & 0x7FF) + 1;
        t->dim = (f1 >> 25) & 3;
        if (t->dim > R300_TEXDIM_CUBE) {
            t->dim = R300_TEXDIM_2D;
        }
        t->depth = t->dim == R300_TEXDIM_3D ? 1u << ((f0 >> 22) & 0xF) : 1;
        {
            /* NUM_LEVELS counts the levels below the base, up to 1x1x1 */
            uint32_t big = t->width > t->height ? t->width : t->height, chain = 1;
            if (t->depth > big) {
                big = t->depth;
            }
            while (big >> chain) {
                chain++;
            }
            t->levels = ((f0 >> 26) & 0xF) + 1;
            if (t->levels > chain) {
                t->levels = chain;
            }
            if (t->levels > R300_TEX_MAX_LEVELS) {
                t->levels = R300_TEX_MAX_LEVELS;
            }
        }
        if (f0 >> 31) {                 /* TX_PITCH_EN: level 0 pitch in texels */
            uint32_t texels = (f2 & 0x3FFF) + 1;
            t->pitch_bytes = dxt ? ((texels + 3) / 4) * bpp : texels * bpp;
        }
        r300_tex_layout(t, bpp, dxt, f0 >> 31);
        t->pitch_bytes = t->lvl_pitch[0];
        t->format = fmt;
        t->filter0 = r300_reg(st, TX_FILTER0_0 + 4 * k);
        t->filter1 = r300_reg(st, TX_FILTER1_0 + 4 * k);
        u->tex_info[k][0] = 1;
        /* RAW: the uint view's elements per row, as the renderer binds it */
        u->tex_info[k][1] = t->kind == R300_TEXK_RAW ? t->pitch_bytes / t->view_bpp : decode;
        u->tex_info[k][2] = t->width;
        u->tex_info[k][3] = t->height;

        r300_border_color(fmt, r300_reg(st, TX_BORDER_COLOR_0 + 4 * k), u->tex_border[k]);
        {
            int32_t bias = (int32_t)(((t->filter1 >> 3) & 0x3FF) << 22) >> 22;
            uint32_t minl = (t->filter0 >> 17) & 0xF;
            u->tex_lod[k][0] = bias / 32.0f;
            u->tex_lod[k][1] = (float)(minl < t->levels ? minl : t->levels - 1);
            u->tex_lod[k][2] = (float)(t->levels - 1);
            u->tex_lod[k][3] = (float)((t->filter0 >> 13) & 3);
        }
        u->tex_dim[k][0] = t->dim;
        u->tex_dim[k][1] = t->depth;
        u->tex_dim[k][2] = t->pitch_bytes;
        /* only the shader-decoded (raw) path reads the level table: keep the common draw cheap */
        for (uint32_t l = 0; t->kind == R300_TEXK_RAW && l < t->levels && l < R300_TEX_MAX_LEVELS; l++) {
            u->tex_lvl[k][l / 4][l % 4] = t->lvl_off[l];
            u->tex_lvl[k][3 + l / 4][l % 4] = t->lvl_pitch[l];
            u->tex_lvl[k][6 + l / 4][l % 4] = t->lvl_pitch[l] * t->lvl_rows[l];
        }
        /* Mesa gives channel i of the format (storage order) SIGNED_W, _Z, _Y, _X: a 1- or 2-channel format sets
         * only the top bits, so component j (X first) is signed when bit 5 + 3 - j is set. */
        u->tex_dim[k][3] = ((f1 >> 5) & 1) << 0 | ((f1 >> 6) & 1) << 1 | ((f1 >> 7) & 1) << 2 |
                           ((f1 >> 8) & 1) << 3 | ((f1 >> 21) & 1 ? R300_TEXF_GAMMA : 0) |
                           (t->levels > 1 || t->dim != R300_TEXDIM_2D ? R300_TEXF_POT_ROWS : 0);
    }
}

static void set_depth(const R300State *st, R300DrawPacket *pkt)
{
    uint32_t cntl = r300_reg(st, ZB_CNTL);
    uint32_t fmt = r300_reg(st, ZB_FORMAT) & 0xF;
    uint32_t pitch = r300_reg(st, ZB_DEPTHPITCH);
    R300DepthDesc *z = &pkt->depth;
    R300FSUniforms *u = &pkt->uniforms;

    memset(z, 0, sizeof(*z));
    /* STENCIL_ENABLE or Z_ENABLE; Z_WRITE_ENABLE alone does nothing. */
    if (!(cntl & 3)) {
        return;
    }
    z->attach = true;
    z->gpu_addr = r300_reg(st, ZB_DEPTHOFFSET) & ~0x1Fu;
    z->pitch = pitch & 0x3FFC;
    z->format = fmt;
    z->bpp = fmt == 2 ? 4 : 2;
    if (fmt > 2) {
        pkt->warn |= R300_WARN_DEPTH;       /* reserved format: as Z24S8 */
        z->bpp = 4;
    } else if (fmt == 1) {
        pkt->warn |= R300_WARN_DEPTH;       /* 13E3 float: kept as 16-bit unorm */
    }
    u->zinfo[0] = cntl;
    u->zinfo[1] = r300_reg(st, ZB_ZSTENCILCNTL);
    u->zinfo[2] = r300_reg(st, ZB_STENCILREFMASK);
    u->zinfo[3] = ((pitch >> 19) & R300_ZFMT_ENDIAN_MASK) |
                  (z->bpp == 2 ? R300_ZFMT_Z16 : 0);
    if (z->bpp == 2) {
        u->poly_offset[0] /= 16.0f;
        u->poly_offset[2] /= 16.0f;
    }
}

static void viewport_xform(const R300State *st, const R300DrawPacket *pkt,
                           const float c[4], float p[4])
{
    uint32_t vte = r300_reg(st, VAP_VTE_CNTL);
    float vp[6];
    float W = pkt->rt_width, H = pkt->rt_height;

    for (int i = 0; i < 6; i++) {
        bool en = (vte >> i) & 1;
        vp[i] = en ? bits_to_float(r300_reg(st, SE_VPORT_XSCALE + 4 * i))
                   : (i % 2 ? 0.0f : 1.0f);
    }
    /* vp: xscale, xoffset, yscale, yoffset, zscale, zoffset */
    if (!((vte >> 8) & 1)) {
        /* Perspective divide by the VTE: stay linear in clip space so Metal
         * clips and interpolates exactly as the card would. */
        float w = c[3];
        p[0] = (2.0f * vp[0] / W) * c[0] + (2.0f * vp[1] / W - 1.0f) * w;
        p[1] = -(2.0f * vp[2] / H) * c[1] + (1.0f - 2.0f * vp[3] / H) * w;
        /* clip-space z in Vulkan's 0..w clip volume; the viewport depth range (pkt->depth_min/max) does the rest */
        p[2] = pkt->depth_dx ? c[2] : 0.5f * (c[2] + w);
        p[3] = w;
    } else {
        float wx = vp[0] * c[0] + vp[1], wy = vp[2] * c[1] + vp[3];
        p[0] = 2.0f * wx / W - 1.0f;
        p[1] = 1.0f - 2.0f * wy / H;
        p[2] = vp[4] * c[2] + vp[5];
        p[3] = 1.0f;
    }
}

/*
 * Emit a primitive list as expanded triangles/lines/points.  With prov,
 * also the flat-shading candidates of each output primitive (the
 * FIRST, SECOND, THIRD and LAST vertex of the source primitive, per
 * GA_COLOR_CONTROL.PROVOKING_VERTEX) and, for triangles, the edge flags
 * polygon-mode lines honour (bit 0: edge 0-1, bit 1: 1-2, bit 2: 2-0;
 * the diagonals the card adds inside quads and polygons are not edges).
 */
uint32_t r300_assemble_prov(unsigned prim, uint32_t n, uint32_t *list,
                            uint32_t *cls, R300Prov *prov)
{
    uint32_t m = 0, np = 0;

#define PUT(a) (list[m++] = (a))
#define PROV(f, s2, t, l, e) do { if (prov) { \
        prov[np].v[0] = (f); prov[np].v[1] = (s2); prov[np].v[2] = (t); \
        prov[np].v[3] = (l); prov[np].edges = (e); } np++; } while (0)
    switch (prim) {
    case 1:                                     /* points */
        *cls = 2;
        for (uint32_t i = 0; i < n; i++) { PUT(i); PROV(i, i, i, i, 0); }
        break;
    case 2:                                     /* lines */
        *cls = 1;
        for (uint32_t i = 0; i + 1 < n; i += 2) {
            PUT(i); PUT(i + 1); PROV(i, i + 1, i + 1, i + 1, 1);
        }
        break;
    case 3:                                     /* line strip */
    case 12:                                    /* line loop */
        *cls = 1;
        for (uint32_t i = 0; i + 1 < n; i++) {
            PUT(i); PUT(i + 1); PROV(i, i + 1, i + 1, i + 1, 1);
        }
        if (prim == 12 && n > 2) { PUT(n - 1); PUT(0); PROV(n - 1, 0, 0, 0, 1); }
        break;
    case 4:                                     /* triangles */
    case 7:                                     /* triangles with wFlags */
        *cls = 0;
        for (uint32_t i = 0; i + 2 < n; i += 3) {
            PUT(i); PUT(i + 1); PUT(i + 2); PROV(i, i + 1, i + 2, i + 2, 7);
        }
        break;
    case 5:                                     /* fan */
        *cls = 0;
        for (uint32_t i = 1; i + 1 < n; i++) {
            PUT(0); PUT(i); PUT(i + 1); PROV(0, i, i + 1, i + 1, 7);
        }
        break;
    case 15:                                    /* polygon */
        *cls = 0;
        for (uint32_t i = 1; i + 1 < n; i++) {
            PUT(0); PUT(i); PUT(i + 1);
            /* PROVOKING_VERTEX_LAST on a polygon reduces to the first vertex (Mesa r300_provoking_vertex_fixes) */
            PROV(0, 1, 2, 0, (i == 1 ? 1 : 0) | 2 | (i + 2 == n ? 4 : 0));
        }
        break;
    case 6:                                     /* strip */
        *cls = 0;
        for (uint32_t i = 0; i + 2 < n; i++) {
            if (i & 1) { PUT(i + 1); PUT(i); PUT(i + 2); }
            else       { PUT(i); PUT(i + 1); PUT(i + 2); }
            PROV(i, i + 1, i + 2, i + 2, 7);
        }
        break;
    case 13:                                    /* quads */
        *cls = 0;
        for (uint32_t i = 0; i + 3 < n; i += 4) {
            PUT(i); PUT(i + 1); PUT(i + 2); PROV(i, i + 1, i + 2, i + 3, 3);
            PUT(i); PUT(i + 2); PUT(i + 3); PROV(i, i + 1, i + 2, i + 3, 6);
        }
        break;
    case 14:                                    /* quad strip */
        *cls = 0;
        for (uint32_t i = 0; i + 3 < n; i += 2) {
            PUT(i); PUT(i + 1); PUT(i + 3); PROV(i, i + 1, i + 2, i + 3, 3);
            PUT(i); PUT(i + 3); PUT(i + 2); PROV(i, i + 1, i + 2, i + 3, 6);
        }
        break;
    default:
        return 0;
    }
#undef PROV
#undef PUT
    return m;
}

uint32_t r300_assemble(unsigned prim, uint32_t n, uint32_t *list,
                       uint32_t *cls)
{
    return r300_assemble_prov(prim, n, list, cls, NULL);
}

/* ---- primitive emission --------------------------------------------- */

typedef struct Build {
    const R300State *st;
    R300DrawPacket *pkt;
    const Layout *lay;
    const Route *route;
    float (*outs)[R300_PVS_NUM_OUTPUTS][4];     /* per source vertex */
    R300Vertex *xv;                             /* per source vertex */
    uint32_t n;

    uint32_t colctl;            /* GA_COLOR_CONTROL */
    bool fancy_colors;          /* flat/solid shading or two-sided colours */
    bool back_colors;           /* output colours 2/3 are back-face 0/1 */
    float solid[4];             /* GA_SOLID_* */
    uint32_t poly_mode;         /* GA_POLY_MODE, 0 when off */
    uint32_t cull;              /* SU_CULL_MODE for CPU culling, else 0 */
    float line_hw;              /* half line width, pixels */
    uint32_t line_end;          /* GA_LINE_CNTL.END_TYPE */
    float stip_scale;           /* GA_LINE_STIPPLE_CONFIG.STIPPLE_SCALE */
    uint32_t stip_reset;
    float stip_acc;             /* stipple accumulator, subpixels */
    float point_hw, point_hh;   /* GA_POINT_SIZE, half, pixels */
    float point_min, point_max; /* GA_POINT_MINMAX diameters, pixels */
    bool point_psize;           /* per-vertex point size */

    R300Vertex *tri, *line;
    uint32_t ntri, nline, cap_tri, cap_line;
} Build;

static R300Vertex *push(R300Vertex **buf, uint32_t *n, uint32_t *cap, uint32_t k)
{
    if (*n + k > *cap) {
        *cap = (*n + k) * 2 + 16;
        *buf = realloc(*buf, sizeof(R300Vertex) * *cap);
    }
    *n += k;
    return *buf + *n - k;
}

/* Colours of source vertex i as the rasterizer sees them on a face. */
static void face_colors(Build *b, uint32_t i, bool back, float cols[4][4])
{
    vtx_colors(b->lay, b->outs[i], cols);
    if (back && b->back_colors) {
        /* Two-sided lighting: the setup engine picks colours 2 and 3 for
         * back faces (AMD R5xx guide 7.5). */
        for (int c = 0; c < 2; c++) {
            if (b->lay->color[2 + c] >= 0) {
                memcpy(cols[c], cols[2 + c], sizeof(cols[c]));
            }
        }
    }
}

/*
 * One output vertex from source vertex i: prov is its primitive's
 * provoking-vertex candidates (NULL: none), back its facing, sprite the
 * rasterizer's generated (s, t) or NULL.
 */
static void emit_vertex(Build *b, uint32_t i, const R300Prov *prov, bool back,
                        const float *sprite, R300Vertex *o)
{
    *o = b->xv[i];
    if (!b->fancy_colors && !sprite) {
        return;
    }
    float cols[4][4];
    face_colors(b, i, back, cols);
    if (b->fancy_colors) {
        float pc[4][4];
        uint32_t pv = prov ? prov->v[(b->colctl >> 16) & 3] : i;
        face_colors(b, pv < b->n ? pv : i, back, pc);
        for (int c = 0; c < 4; c++) {
            unsigned rgb = (b->colctl >> (4 * c)) & 3, a = (b->colctl >> (4 * c + 2)) & 3;
            for (int k = 0; k < 4; k++) {
                unsigned mode = k < 3 ? rgb : a;
                if (mode == 0) {
                    cols[c][k] = b->solid[k];
                } else if (mode == 1) {
                    cols[c][k] = pc[c][k];
                }
            }
        }
    }
    rs_values(b->st, b->lay, b->route, b->outs[i], (const float (*)[4])cols,
              sprite, o->v);
}

static void ndc_px(const Build *b, const float *p, float *x, float *y)
{
    float w = p[3] != 0.0f ? p[3] : 1.0f;
    *x = (p[0] / w + 1.0f) * b->pkt->rt_width * 0.5f;
    *y = (1.0f - p[1] / w) * b->pkt->rt_height * 0.5f;
}

/* Replace the x, y of clip-space p by pixel position (x, y), keeping w. */
static void px_ndc(const Build *b, float x, float y, float *p)
{
    float w = p[3] != 0.0f ? p[3] : 1.0f;
    p[0] = (2.0f * x / b->pkt->rt_width - 1.0f) * w;
    p[1] = (1.0f - 2.0f * y / b->pkt->rt_height) * w;
}

/*
 * A point: a screen-aligned square GA_POINT_SIZE wide (width in the high
 * half, height in the low half, in sixths of a pixel), or the vertex's
 * point size clamped to GA_POINT_MINMAX, with texture coordinates S0..S1
 * left to right and T0..T1 bottom to top.  Apple's compositor presents
 * the screen as one such point covering the whole frame buffer.
 */
static void emit_point(Build *b, uint32_t i, const R300Prov *prov, bool back)
{
    const R300State *st = b->st;
    const float *p = b->xv[i].pos;
    float hw = b->point_hw, hh = b->point_hh;
    float s0 = bits_to_float(r300_reg(st, GA_POINT_S0));
    float t0 = bits_to_float(r300_reg(st, GA_POINT_T0));
    float s1 = bits_to_float(r300_reg(st, GA_POINT_S1));
    float t1 = bits_to_float(r300_reg(st, GA_POINT_T1));
    static const int corner[6] = { 0, 1, 2, 0, 2, 3 };
    float cx, cy;

    if (b->point_psize) {
        float d = b->outs[i][b->lay->psize][0];
        d = fminf(fmaxf(d, b->point_min), b->point_max);
        hw = hh = d * 0.5f;
    }
    ndc_px(b, p, &cx, &cy);
    /* corners: top-left, top-right, bottom-right, bottom-left */
    float x[4] = { cx - hw, cx + hw, cx + hw, cx - hw };
    float y[4] = { cy - hh, cy - hh, cy + hh, cy + hh };
    float sc[4] = { s0, s1, s1, s0 };
    float tc[4] = { t1, t1, t0, t0 };
    R300Vertex *o = push(&b->tri, &b->ntri, &b->cap_tri, 6);

    for (int k = 0; k < 6; k++) {
        int c = corner[k];
        float st2[2] = { sc[c], tc[c] };

        emit_vertex(b, i, prov, back, st2, &o[k]);
        /* screen-space square at the vertex's depth */
        o[k].pos[2] = p[2] / (p[3] != 0.0f ? p[3] : 1.0f);
        o[k].pos[3] = 1.0f;
        px_ndc(b, x[c], y[c], o[k].pos);
    }
}

/*
 * A line from source vertex i0 to i1.  The rasterizer stuffs (s, t) for
 * the texture components past the vertex's own: s runs GA_LINE_S0 to S1
 * along the line, or with GA_LINE_STIPPLE_CONFIG.STIPPLE_SCALE set, is the
 * stipple accumulator (subpixels, 12 a pixel) times the scale, which the
 * driver's program looks up in its stipple pattern.  Lines wider than a
 * pixel become quads: offset across the major axis (END_TYPE horizontal,
 * vertical, square), or perpendicular to the line (computed).
 */
static void emit_line(Build *b, uint32_t i0, uint32_t i1, const R300Prov *prov,
                      bool back)
{
    const float *p0 = b->xv[i0].pos, *p1 = b->xv[i1].pos;
    float x0, y0, x1, y1;
    float sp0[2] = { 0, 0 }, sp1[2] = { 0, 0 };

    ndc_px(b, p0, &x0, &y0);
    ndc_px(b, p1, &x1, &y1);
    float dx = x1 - x0, dy = y1 - y0, len = sqrtf(dx * dx + dy * dy);
    if (b->stip_scale != 0.0f) {
        if (b->stip_reset == 1) {
            b->stip_acc = 0.0f;
        }
        sp0[0] = b->stip_acc * b->stip_scale;
        b->stip_acc += len * 12.0f;
        sp1[0] = b->stip_acc * b->stip_scale;
    } else {
        sp0[0] = bits_to_float(r300_reg(b->st, GA_LINE_S0));
        sp1[0] = bits_to_float(r300_reg(b->st, GA_LINE_S1));
    }

    if (b->line_hw <= 0.75f) {
        R300Vertex *o = push(&b->line, &b->nline, &b->cap_line, 2);
        emit_vertex(b, i0, prov, back, sp0, &o[0]);
        emit_vertex(b, i1, prov, back, sp1, &o[1]);
        return;
    }
    float ox, oy, hw = b->line_hw;
    if (b->line_end == 3 && len > 0.0f) {
        ox = -dy / len * hw;
        oy = dx / len * hw;
    } else if (fabsf(dx) >= fabsf(dy)) {
        ox = 0.0f; oy = hw;             /* x-major: widen vertically */
    } else {
        ox = hw; oy = 0.0f;
    }
    R300Vertex q[4];
    emit_vertex(b, i0, prov, back, sp0, &q[0]);
    q[1] = q[0];
    emit_vertex(b, i1, prov, back, sp1, &q[2]);
    q[3] = q[2];
    px_ndc(b, x0 - ox, y0 - oy, q[0].pos);
    px_ndc(b, x0 + ox, y0 + oy, q[1].pos);
    px_ndc(b, x1 + ox, y1 + oy, q[2].pos);
    px_ndc(b, x1 - ox, y1 - oy, q[3].pos);
    static const int tri[6] = { 0, 1, 2, 0, 2, 3 };
    R300Vertex *o = push(&b->tri, &b->ntri, &b->cap_tri, 6);
    for (int k = 0; k < 6; k++) {
        o[k] = q[tri[k]];
    }
}

/* Winding on screen, as Metal and SU_CULL_MODE judge it. */
static bool tri_ccw(const Build *b, uint32_t a, uint32_t c, uint32_t d)
{
    const float *p[3] = { b->xv[a].pos, b->xv[c].pos, b->xv[d].pos };
    float x[3], y[3];
    for (int k = 0; k < 3; k++) {
        float w = p[k][3] != 0.0f ? p[k][3] : 1.0f;
        x[k] = p[k][0] / w;
        y[k] = p[k][1] / w;
    }
    float area = (x[1] - x[0]) * (y[2] - y[0]) - (x[2] - x[0]) * (y[1] - y[0]);
    /* a vertex behind the eye flips the projected winding */
    if ((p[0][3] < 0) ^ (p[1][3] < 0) ^ (p[2][3] < 0)) {
        area = -area;
    }
    return area > 0.0f;
}

static void emit_tri(Build *b, const uint32_t *ix, const R300Prov *prov)
{
    bool front = tri_ccw(b, ix[0], ix[1], ix[2]) == r300_front_ccw(r300_reg(b->st, SU_CULL_MODE));
    bool back = !front;
    /* R300_FACING=front|back|flip: debug override of the facing used for culling and
     * the colour choice (told apart from depth and coverage problems). */
    static int facing = -1;
    if (facing < 0) {
        const char *e = getenv("R300_FACING");
        facing = !e ? 0 : !strcmp(e, "front") ? 1 : !strcmp(e, "back") ? 2 : !strcmp(e, "flip") ? 3 : 0;
        if (facing) {
            fprintf(stderr, "ppc-mac-gpu r300: R300_FACING=%s\n", e);
        }
    }
    if (facing == 1) {
        front = true;
        back = false;
    } else if (facing == 2) {
        front = false;
        back = true;
    } else if (facing == 3) {
        front = !front;
        back = !back;
    }

    if ((front && (b->cull & R300_CULL_FRONT)) || (back && (b->cull & R300_CULL_BACK))) {
        return;
    }
    /* GA_POLY_MODE.FRONT_PTYPE / BACK_PTYPE: 0 points, 1 lines, 2 triangles */
    unsigned ptype = !b->poly_mode ? 2 : front ? (b->poly_mode >> 4) & 7
                                               : (b->poly_mode >> 7) & 7;
    if (ptype == 0) {
        for (int k = 0; k < 3; k++) {
            emit_point(b, ix[k], prov, back);
        }
    } else if (ptype == 1) {
        for (int k = 0; k < 3; k++) {
            if (!prov || (prov->edges >> k) & 1) {
                emit_line(b, ix[k], ix[(k + 1) % 3], prov, back);
            }
        }
    } else {
        R300Vertex *o = push(&b->tri, &b->ntri, &b->cap_tri, 3);
        for (int k = 0; k < 3; k++) {
            emit_vertex(b, ix[k], prov, back, NULL, &o[k]);
        }
    }
}

/* S3.12 fixed point (GA_SOLID_*). */
static float s312(uint32_t v)
{
    return (int16_t)(v & 0xFFFF) / 4096.0f;
}

/*
 * PPCGPU_PVSREC=file: append the vertex program, its constants and the input
 * and output vertices of draws of exactly PPCGPU_PVSREC_N vertices (up to
 * PPCGPU_PVSREC_MAX of them, default 60), to find out what a program does to
 * a given mesh.  Binary, little endian host layout:
 *   "PVS1", u32 n, u32 first_inst, u32 last_inst, u32 max_const, u32 fc_opc,
 *   u32 fc_addrs[16], u32 fc_loop[16], u32 code[256*4], f32 consts[256*4],
 *   f32 in[n][32][4], f32 out[n][32][4]
 */
static struct {
    int init;
    const char *path;
    uint32_t n, max, done;
} pvsrec;

static bool pvsrec_wanted(uint32_t n)
{
    if (!pvsrec.init) {
        const char *p = getenv("PPCGPU_PVSREC"), *v = getenv("PPCGPU_PVSREC_N"),
                   *m = getenv("PPCGPU_PVSREC_MAX");
        pvsrec.init = 1;
        pvsrec.path = p && *p && v ? p : NULL;
        pvsrec.n = v ? (uint32_t)strtoul(v, NULL, 0) : 0;
        pvsrec.max = m ? (uint32_t)strtoul(m, NULL, 0) : 60;
        if (pvsrec.path) {
            fprintf(stderr, "ppc-mac-gpu r300: PPCGPU_PVSREC=%s: recording vertex programs of %u-vertex draws (up to %u)\n",
                    pvsrec.path, pvsrec.n, pvsrec.max);
        }
    }
    /* PPCGPU_PVSREC_N=0: every distinct program once, its first 64 vertices (see pvsrec_write) */
    return pvsrec.path && n && (pvsrec.n ? n == pvsrec.n : true) && pvsrec.done < pvsrec.max;
}

/* Has this program (code, flow control, constants) been recorded already?  Only used when PVSREC_N is 0. */
static bool pvsrec_seen(const R300PVSProgram *prog, const float (*consts)[4])
{
    static uint64_t seen[1024];
    static unsigned nseen;
    uint64_t h = 1469598103934665603ull;

    for (unsigned pc = prog->first_inst; pc <= prog->last_inst && pc < R300_PVS_MAX_INSTS; pc++) {
        for (int k = 0; k < 4; k++) {
            h = (h ^ prog->code[pc * 4 + k]) * 1099511628211ull;
        }
    }
    h = (h ^ prog->fc_opc) * 1099511628211ull;
    for (int i = 0; i <= prog->max_const && i < 256; i++) {
        for (int k = 0; k < 4; k++) {
            uint32_t w;

            memcpy(&w, &consts[i][k], sizeof(w));
            h = (h ^ w) * 1099511628211ull;
        }
    }
    for (unsigned i = 0; i < nseen; i++) {
        if (seen[i] == h) {
            return true;
        }
    }
    if (nseen < 1024) {
        seen[nseen++] = h;
    }
    return false;
}

static void pvsrec_write(const R300PVSProgram *prog, const float (*consts)[4],
                         uint32_t n, const float (*cin)[R300_PVS_NUM_INPUTS][4],
                         const float (*outs)[R300_PVS_NUM_OUTPUTS][4])
{
    FILE *f = fopen(pvsrec.path, "ab");
    uint32_t hdr[5] = { n, prog->first_inst, prog->last_inst,
                        (uint32_t)prog->max_const, prog->fc_opc };
    uint32_t code[R300_PVS_MAX_INSTS * 4] = { 0 };

    if (!f) {
        return;
    }
    memcpy(code, prog->code, sizeof(code));
    fwrite("PVS1", 1, 4, f);
    fwrite(hdr, sizeof(hdr), 1, f);
    fwrite(prog->fc_addrs, sizeof(prog->fc_addrs), 1, f);
    fwrite(prog->fc_loop, sizeof(prog->fc_loop), 1, f);
    fwrite(code, sizeof(code), 1, f);
    fwrite(consts, sizeof(float) * 4, 256, f);
    fwrite(cin, sizeof(*cin), n, f);
    fwrite(outs, sizeof(*outs), n, f);
    fclose(f);
    pvsrec.done++;
}


/*
 * Everything after vertex ordering.  order[] (n entries, malloc'd) is
 * taken over; immd is the DRAW_IMMD_2 vertex data or NULL.
 */

/* ---- vertex program on the GPU ---------------------------------------- */

typedef struct Gb {
    char *p;
    size_t n, cap;
} Gb;

static void __attribute__((format(printf, 2, 3))) gb_printf(Gb *g, const char *fmt, ...)
{
    va_list ap;
    int k;

    for (;;) {
        va_start(ap, fmt);
        k = vsnprintf(g->p ? g->p + g->n : NULL, g->cap - g->n, fmt, ap);
        va_end(ap);
        if (k >= 0 && g->n + (size_t)k + 1 <= g->cap) {
            g->n += k;
            return;
        }
        g->cap = g->cap ? g->cap * 2 : 8192;
        g->p = realloc(g->p, g->cap);
    }
}

/*
 * Word layout of the vertex shader's buffer (R300DrawPacket.gpu_data):
 *   0..1023     the 256 constants (vec4), zero past max_const
 *   1024 W  1025 H  1026 VTE bit 8 (uint)  1027 first word of the input vertices (uint)
 *   1028..1033  viewport xscale, xoffset, yscale, yoffset, zscale, zoffset
 *   1034 fog scale  1035 fog offset  1036 user clip enables (uint)
 *   1038..1061  the six user clip planes (vec4)
 *   1064..      one source vertex index per vertex drawn, then the input vertices (nin vec4 each)
 */
#define GPU_IDX_BASE        1064u

/*
 * Does this draw take the GPU vertex path?  Triangles only, with the colours as they come out of the vertex
 * program and no polygon mode: the CPU path then does nothing to a vertex but copy it, so the shader can do all of
 * it.  R300_PVS_GPU=0 forces the CPU path, 1 the GPU path; the default is the CPU path until the GPU one is
 * proven (see work move_r300_vertex_stage_to_gpu).
 */
static bool gpu_vs_wanted(const R300State *st, unsigned prim, bool rects, bool bypass)
{
    static int mode = -1;

    if (mode < 0) {
        const char *e = getenv("R300_PVS_GPU");

        mode = e ? atoi(e) : 1;   /* default on; R300_PVS_GPU=0 disables */
    }
    if (!mode || rects || bypass || !((1u << prim) & 0xE0F0u)) {
        return false;
    }
    if ((r300_reg(st, GA_POLY_MODE) & 3) == 1) {
        return false;
    }
    uint32_t colctl = r300_reg(st, GA_COLOR_CONTROL);
    for (int c = 0; c < 4; c++) {
        if (((colctl >> (4 * c)) & 3) != 2 || ((colctl >> (4 * c + 2)) & 3) != 2) {
            return false;                       /* flat or solid shading */
        }
    }
    return (r300_reg(st, VAP_OUTPUT_VTX_FMT_0) & (3u << 3)) == 0;   /* two-sided colours */
}

/* The vertex stage as GLSL: the vertex program, then what vtx_colors, viewport_xform and rs_values do. */
static char *gpu_vs_text(const R300State *st, const Layout *lay, const Route *route, const char *pvs_glsl,
                         uint32_t nin, const uint8_t *slots, bool fog, uint32_t fogsel, uint32_t clip)
{
    Gb g = { 0 };
    static const int8_t tbl[16][4] = {
        [0] = { 0, 1, 2, 3 },  [1] = { 0, 1, 2, -1 }, [2] = { 0, 1, 2, -2 },
        [4] = { -1, -1, -1, 3 }, [5] = { -1, -1, -1, -1 }, [6] = { -1, -1, -1, -2 },
        [8] = { -2, -2, -2, 3 }, [9] = { -2, -2, -2, -1 }, [10] = { -2, -2, -2, -2 },
    };

    gb_printf(&g,
        "layout(std430, set = 0, binding = 2) readonly buffer R300PVS { uint pvs_data[]; };\n"
        "/* ms.xy: the multisample being drawn, as a clip-space shift */\n"
        "layout(std140, set = 0, binding = 3) uniform R300MS { vec4 ms; };\n"
        "layout(location = 0) out vec4 v0; layout(location = 1) out vec4 v1;\n"
        "layout(location = 2) out vec4 v2; layout(location = 3) out vec4 v3;\n"
        "layout(location = 4) out vec4 v4; layout(location = 5) out vec4 v5;\n"
        "layout(location = 6) out vec4 v6; layout(location = 7) out vec4 v7;\n"
        "layout(location = 8) out vec4 v8; layout(location = 9) out vec4 v9;\n"
        "layout(location = 10) out vec4 aux;\n");
    if (clip & 0x3F) {
        gb_printf(&g, "out gl_PerVertex { vec4 gl_Position; float gl_ClipDistance[6]; };\n");
    }
    gb_printf(&g,
        "float pvs_f(uint w) { return uintBitsToFloat(pvs_data[w]); }\n"
        "vec4 pvs_v4(uint w) { return vec4(pvs_f(w), pvs_f(w + 1u), pvs_f(w + 2u), pvs_f(w + 3u)); }\n"
        "vec4 pvs_const(int i) { return pvs_v4(uint(i) * 4u); }\n"
        "%s"
        "void main()\n{\n"
        "    uint sv = pvs_data[%uu + uint(gl_VertexIndex)];\n"
        "    uint vb = pvs_data[1027u] + sv * %uu;\n"
        "    vec4 IN[32]; vec4 OUT[32];\n"
        "    for (int i = 0; i < 32; i++) { IN[i] = vec4(0.0); OUT[i] = vec4(0.0); }\n",
        pvs_glsl, GPU_IDX_BASE, nin * 4);
    for (uint32_t j = 0; j < nin; j++) {
        gb_printf(&g, "    IN[%u] = pvs_v4(vb + %uu);\n", slots[j], j * 4);
    }
    gb_printf(&g,
        "    r300_pvs(IN, OUT);\n"
        "    vec4 c = OUT[%d];\n"
        "    float W = pvs_f(1024u), H = pvs_f(1025u);\n"
        "    float xs = pvs_f(1028u), xo = pvs_f(1029u), ys = pvs_f(1030u), yo = pvs_f(1031u), zs = pvs_f(1032u), zo = pvs_f(1033u);\n"
        "    vec4 p;\n"
        "    if (pvs_data[1026u] == 0u) {\n"
        "        /* no perspective divide by the VTE: stay linear in clip space */\n"
        "        p = vec4((2.0 * xs / W) * c.x + (2.0 * xo / W - 1.0) * c.w,\n"
        "                 -(2.0 * ys / H) * c.y + (1.0 - 2.0 * yo / H) * c.w,\n"
        "                 pvs_data[1037u] != 0u ? c.z : 0.5 * (c.z + c.w), c.w);\n"
        "    } else {\n"
        "        float wx = xs * c.x + xo, wy = ys * c.y + yo;\n"
        "        p = vec4(2.0 * wx / W - 1.0, 1.0 - 2.0 * wy / H, zs * c.z + zo, 1.0);\n"
        "    }\n",
        lay->pos);
    for (int cp = 0; cp < 4; cp++) {
        if (lay->color[cp] >= 0) {
            uint32_t rm = r300_reg(st, GA_ROUND_MODE);

            gb_printf(&g, "    vec4 col%d = OUT[%d];\n", cp, lay->color[cp]);
            if (!(rm & 0x10)) {
                gb_printf(&g, "    col%d.xyz = clamp(col%d.xyz, 0.0, 1.0);\n", cp, cp);
            }
            if (!(rm & 0x20)) {
                gb_printf(&g, "    col%d.w = clamp(col%d.w, 0.0, 1.0);\n", cp, cp);
            }
        } else {
            gb_printf(&g, "    vec4 col%d = vec4(0.0);\n", cp);
        }
    }
    for (unsigned k = 0; k < R300_NUM_VARYINGS; k++) {
        if (k >= route->nvary) {
            gb_printf(&g, "    v%u = vec4(0.0);\n", k);
            continue;
        }
        uint32_t ip = r300_reg(st, RS_IP_0 + 4 * route->vary[k].ip);
        gb_printf(&g, "    v%u = vec4(", k);
        for (unsigned i = 0; i < 4; i++) {
            char e[48];

            if (route->vary[k].is_tex) {
                unsigned ptr = ip & 0x3F, sel = (ip >> (13 + 3 * i)) & 7;

                if (sel < 4 && ptr + sel < lay->nscal && lay->scal_slot[ptr + sel] == LAY_GEN) {
                    snprintf(e, sizeof(e), "0.0");      /* rasterizer-generated (s, t): not a point here */
                } else if (sel < 4 && ptr + sel < lay->nscal) {
                    snprintf(e, sizeof(e), "OUT[%u].%c", lay->scal_slot[ptr + sel], "xyzw"[lay->scal_comp[ptr + sel]]);
                } else {
                    snprintf(e, sizeof(e), "%s", sel == 5 ? "1.0" : "0.0");
                }
            } else {
                unsigned cp = (ip >> 6) & 7;
                int sl = tbl[((ip >> 9) & 0xF) & 15][i];

                if (sl >= 0 && cp < 4) {
                    snprintf(e, sizeof(e), "col%u.%c", cp, "xyzw"[sl]);
                } else {
                    snprintf(e, sizeof(e), "%s", sl == -2 ? "1.0" : "0.0");
                }
            }
            gb_printf(&g, "%s%s", i ? ", " : "", e);
        }
        gb_printf(&g, ");\n");
    }
    if (fog) {
        if (fogsel < 4) {
            gb_printf(&g, "    float fsrc = %s;\n", lay->color[fogsel] >= 0 ? "" : "0.0");
            if (lay->color[fogsel] >= 0) {
                g.n -= 0;
                gb_printf(&g, "    fsrc = OUT[%d].w;\n", lay->color[fogsel]);
            }
        } else if (fogsel == 4) {
            gb_printf(&g, "    float fsrc = c.w;\n");
        } else {
            gb_printf(&g, "    float fsrc = p.z / (p.w != 0.0 ? p.w : 1.0);\n");
        }
        gb_printf(&g, "    aux = vec4(fsrc * pvs_f(1034u) + pvs_f(1035u), 0.0, 0.0, 0.0);\n");
    } else {
        gb_printf(&g, "    aux = vec4(0.0);\n");
    }
    if (clip & 0x3F) {
        for (int k = 0; k < 6; k++) {
            gb_printf(&g, "    gl_ClipDistance[%d] = ((pvs_data[1036u] >> %du) & 1u) != 0u ? dot(c, pvs_v4(%uu)) : 1.0;\n",
                      k, k, 1038 + 4 * k);
        }
    }
    gb_printf(&g, "    gl_Position = vec4(p.xy + ms.xy * p.w, p.zw);\n}\n");
    return g.p;
}

/*
 * Build a draw whose vertex stage runs on the GPU.  Returns 1 with pkt filled (verts NULL, gpu_data set) and *vs the
 * vertex stage text, 0 when this draw must take the CPU path (nothing consumed), -1 with *err set on a fetch failure.
 */
static int gpu_vs_build(const R300State *st, Fetch *f, R300PVSProgram *prog, const float (*consts)[4],
                        const Layout *lay, const Route *route, R300DrawPacket *pkt, const uint32_t *order,
                        uint32_t n, unsigned prim, bool fog, uint32_t fogsel, uint32_t clip,
                        char **vs, const char **err)
{
    uint32_t unsup = 0, nidx, cls, nin = 0, words;
    uint8_t slots[R300_PVS_NUM_INPUTS];
    uint32_t *list, *gd;
    R300Prov *prov;
    char *glsl;
    uint32_t vte = r300_reg(st, VAP_VTE_CNTL);

    glsl = r300_pvs_to_glsl(prog, &unsup);
    if (!glsl) {
        return 0;
    }
    list = malloc(sizeof(uint32_t) * (n * 3 + 6));
    prov = malloc(sizeof(R300Prov) * (n * 2 + 2));
    nidx = r300_assemble_prov(prim, n, list, &cls, prov);
    free(prov);
    if (!nidx || cls != 0) {
        free(list);
        free(glsl);
        return 0;
    }
    for (unsigned i = 0; i < R300_PVS_NUM_INPUTS; i++) {
        if ((prog->in_mask >> i) & 1) {
            slots[nin++] = i;
        }
    }
    words = GPU_IDX_BASE + nidx + n * nin * 4;
    gd = calloc(words, sizeof(uint32_t));
    for (int i = 0; i < 256 && i <= prog->max_const; i++) {
        memcpy(&gd[i * 4], consts[i], 16);
    }
    {
        float w[6];

        w[0] = pkt->rt_width;
        w[1] = pkt->rt_height;
        memcpy(&gd[1024], w, 8);
        gd[1026] = (vte >> 8) & 1;
        gd[1027] = GPU_IDX_BASE + nidx;
        for (int i = 0; i < 6; i++) {
            w[i] = ((vte >> i) & 1) ? bits_to_float(r300_reg(st, SE_VPORT_XSCALE + 4 * i)) : (i % 2 ? 0.0f : 1.0f);
        }
        memcpy(&gd[1028], w, sizeof(w));
        w[0] = bits_to_float(r300_reg(st, GA_FOG_SCALE));
        w[1] = bits_to_float(r300_reg(st, GA_FOG_OFFSET));
        memcpy(&gd[1034], w, 8);
        gd[1036] = clip & 0x3F;
        gd[1037] = pkt->depth_dx;
        memcpy(&gd[1038], &st->pvs_mem[R300_PVS_UCP_START * 4], 6 * 16);
    }
    for (uint32_t i = 0; i < nidx; i++) {
        gd[GPU_IDX_BASE + i] = list[i];
    }
    free(list);
    for (uint32_t v = 0; v < n; v++) {
        float in[R300_PVS_NUM_INPUTS][4];

        if (!fetch_vertex(f, order[v], in)) {
            *err = f->why ? f->why : "vertex data out of range";
            free(gd);
            free(glsl);
            return -1;
        }
        for (uint32_t j = 0; j < nin; j++) {
            memcpy(&gd[GPU_IDX_BASE + nidx + (v * nin + j) * 4], in[slots[j]], 16);
        }
    }
    *vs = gpu_vs_text(st, lay, route, glsl, nin, slots, fog, fogsel, clip);
    free(glsl);
    {
        static bool said;

        if (!said) {
            said = true;
            fprintf(stderr, "ppc-mac-gpu r300: vertex programs run on the GPU (R300_PVS_GPU)\n");
        }
    }
    pkt->gpu_vs = true;
    pkt->gpu_data = gd;
    pkt->gpu_words = words;
    pkt->num_verts = nidx;
    pkt->prim_class = 0;
    pkt->cull = r300_reg(st, SU_CULL_MODE) & 7;
    return 1;
}

static bool draw_core(const R300State *st, const R300Arrays *arr,
                      uint32_t vf, uint32_t *order, uint32_t n,
                      const uint32_t *immd, uint32_t immd_dw,
                      R300ReadFn read, void *opaque,
                      R300DrawPacket *pkt, const char **err)
{
    uint32_t prim, nidx = 0;
    uint32_t *list = NULL;
    R300Prov *prov = NULL;
    Fetch f = { 0 };
    Layout lay;
    Route route;
    R300FSDesc desc;
    uint32_t pitch_reg = r300_reg(st, RB3D_COLORPITCH0);
    uint32_t cntl0 = r300_reg(st, VAP_PVS_CODE_CNTL_0);
    uint32_t cc = r300_reg(st, VAP_PVS_CONST_CNTL);
    bool bypass = (r300_reg(st, VAP_CNTL_STATUS) >> 8) & 1;
    uint32_t tl = r300_reg(st, SC_SCISSORS_TL), br = r300_reg(st, SC_SCISSORS_BR);
    uint32_t clip = r300_reg(st, VAP_CLIP_CNTL);
    uint32_t fogsel = r300_reg(st, GB_SELECT) & 7;
    bool fog = r300_reg(st, FG_FOG_BLEND) & 1;
    float fog_scale = bits_to_float(r300_reg(st, GA_FOG_SCALE));
    float fog_off = bits_to_float(r300_reg(st, GA_FOG_OFFSET));
    R300PVSProgram prog;
    float consts[256][4];
    Build b;

    prim = vf & 0xF;

    /* Colour buffer */
    pkt->rt_gpu_addr = r300_reg(st, RB3D_COLOROFFSET0) & ~0x1Fu;
    pkt->rt_pitch = pitch_reg & 0x3FFE;
    pkt->rt_format = (pitch_reg >> 21) & 0xF;
    pkt->rt_view = r300_cb_view(pkt->rt_format, r300_reg(st, US_OUT_FMT_0),
                                &pkt->rt_bpp);
    if ((pkt->rt_bpp == 1 && (pitch_reg >> 19) & 3) ||
        (pkt->rt_bpp == 2 && ((pitch_reg >> 19) & 3) >= 2)) {
        pkt->warn |= R300_WARN_RTFMT;   /* swap across pixels: not done */
    }
    if (pkt->rt_view == R300_RTV_NONE) {
        pkt->warn |= R300_WARN_RTFMT;
        *err = "colour buffer format not supported";
        free(order);
        return false;
    }
    /* Render targets B-D (multiple render targets or multiwrites) */
    pkt->num_cb = r300_us_num_targets(st);
    for (uint32_t k = 1; k < pkt->num_cb; k++) {
        R300ColorDesc *c = &pkt->cb[k];
        uint32_t pr = r300_reg(st, RB3D_COLORPITCH0 + 4 * k);
        c->gpu_addr = r300_reg(st, RB3D_COLOROFFSET0 + 4 * k) & ~0x1Fu;
        c->pitch = pr & 0x3FFE;
        c->format = (pr >> 21) & 0xF;
        c->view = r300_cb_view(c->format, r300_us_out_fmt(st, k), &c->bpp);
        if (c->view == R300_RTV_NONE || !c->pitch) {
            pkt->warn |= R300_WARN_RTFMT;
            pkt->num_cb = k;            /* bind the ones before it */
            break;
        }
    }
    pkt->scissor[0] = ((tl & 0x1FFF) > SC_COORD_BIAS) ? (tl & 0x1FFF) - SC_COORD_BIAS : 0;
    pkt->scissor[1] = (((tl >> 13) & 0x1FFF) > SC_COORD_BIAS) ?
                      ((tl >> 13) & 0x1FFF) - SC_COORD_BIAS : 0;
    pkt->scissor[2] = ((br & 0x1FFF) >= SC_COORD_BIAS) ?
                      (br & 0x1FFF) - SC_COORD_BIAS + 1 : 0;
    pkt->scissor[3] = (((br >> 13) & 0x1FFF) >= SC_COORD_BIAS) ?
                      ((br >> 13) & 0x1FFF) - SC_COORD_BIAS + 1 : 0;
    if (GUEST_LE) {
        /* The DRM clips with up to four cliprects (SC_CLIPRECT_TL/BR_0..3) chosen by
         * SC_CLIP_RULE: bit (in0 | in1<<1 | in2<<2 | in3<<3) says whether a pixel inside
         * exactly those rectangles is drawn.  Use the bounding box of the rectangles that
         * take part in a passing combination; a rule that passes pixels outside every
         * rectangle (index 0) clips nothing. */
        uint32_t rule = r300_reg(st, 0x43D0) & 0xFFFF;
        bool outside_ok = rule & 1;
        uint32_t bx0 = ~0u, by0 = ~0u, bx1 = 0, by1 = 0;
        for (int k = 0; k < 4 && !outside_ok; k++) {
            bool used = false;
            for (int idx = 0; idx < 16; idx++) {
                if ((idx >> k) & 1 && (rule >> idx) & 1) {
                    used = true;
                }
            }
            if (!used) {
                continue;
            }
            uint32_t ctl = r300_reg(st, 0x43B0 + 8 * k), cbr = r300_reg(st, 0x43B4 + 8 * k);
            uint32_t x0 = (ctl & 0x1FFF) > SC_COORD_BIAS ? (ctl & 0x1FFF) - SC_COORD_BIAS : 0;
            uint32_t y0 = ((ctl >> 13) & 0x1FFF) > SC_COORD_BIAS ? ((ctl >> 13) & 0x1FFF) - SC_COORD_BIAS : 0;
            uint32_t x1 = (cbr & 0x1FFF) >= SC_COORD_BIAS ? (cbr & 0x1FFF) - SC_COORD_BIAS + 1 : 0;
            uint32_t y1 = ((cbr >> 13) & 0x1FFF) >= SC_COORD_BIAS ? ((cbr >> 13) & 0x1FFF) - SC_COORD_BIAS + 1 : 0;
            bx0 = CLIP_MIN(bx0, x0); by0 = CLIP_MIN(by0, y0);
            bx1 = CLIP_MAX(bx1, x1); by1 = CLIP_MAX(by1, y1);
        }
        if (!outside_ok && bx0 != ~0u) {
            pkt->scissor[0] = CLIP_MAX(pkt->scissor[0], bx0);
            pkt->scissor[1] = CLIP_MAX(pkt->scissor[1], by0);
            pkt->scissor[2] = CLIP_MIN(pkt->scissor[2], bx1);
            pkt->scissor[3] = CLIP_MIN(pkt->scissor[3], by1);
        } else if (!outside_ok) {
            pkt->scissor[2] = pkt->scissor[3] = 0;      /* nothing passes */
        }
    }
    pkt->rt_width = pkt->rt_pitch;
    /* an open scissor (Linux) says nothing about the buffer: it is as tall as the screen */
    pkt->rt_height = st->le_screen_h && (((br >> 13) & 0x1FFF) >= 0x1F00)
                         ? st->le_screen_h : pkt->scissor[3];
    if (!pkt->rt_width || !pkt->rt_height) {
        *err = "empty colour buffer";
        free(order);
        return false;
    }

    pkt->aa_samples = r300_aa_samples(st);
    r300_aa_positions(st, pkt->aa_pos);

    /* Viewport depth range and z clipping, once per draw (see R300DrawPacket.depth_min). */
    {
        uint32_t vte = r300_reg(st, VAP_VTE_CNTL);
        bool zs_en = (vte >> 4) & 1;
        float zs = zs_en ? bits_to_float(r300_reg(st, SE_VPORT_XSCALE + 4 * 4)) : 1.0f;
        float zo = (vte >> 5) & 1 ? bits_to_float(r300_reg(st, SE_VPORT_XSCALE + 4 * 5)) : 0.0f;

        pkt->depth_dx = (r300_reg(st, VAP_CNTL) >> 22) & 1;
        if (zs_en && !((vte >> 8) & 1)) {
            pkt->depth_min = pkt->depth_dx ? zo : zo - zs;
            pkt->depth_max = zo + zs;
        } else {
            pkt->depth_min = 0.0f;                  /* z is already the depth (or not transformed) */
            pkt->depth_max = 1.0f;
        }
        pkt->depth_min = fminf(fmaxf(pkt->depth_min, 0.0f), 1.0f);
        pkt->depth_max = fminf(fmaxf(pkt->depth_max, 0.0f), 1.0f);
        pkt->depth_clip = !((r300_reg(st, VAP_CLIP_CNTL) >> 16) & 1);
    }

    set_uniforms(st, pkt);
    set_textures(st, pkt);
    set_depth(st, pkt);
    /*
     * Mesa gives a window's colour buffer a wider row pitch than its depth buffer (64 against 32 pixels for a
     * 32 pixel window).  The depth buffer only has to cover what is drawn, so when the scissor fits inside the
     * depth pitch the render area is that wide; wider than the pitch the draw cannot use the depth buffer.
     */
    if (pkt->depth.attach && pkt->depth.pitch && pkt->depth.pitch < pkt->rt_width &&
        pkt->scissor[2] <= pkt->depth.pitch) {
        pkt->rt_width = pkt->depth.pitch;
    }
    /*
     * Also when the masked colour target is the depth buffer's own memory (Mesa blits packed depth/stencil as
     * ARGB8888 words, then draws stencil ops with that same buffer bound as colour): the colour store would write
     * the old word back over the stencil and depth the same invocation just updated.
     */
    if (pkt->depth.attach && (pkt->depth.pitch > pkt->rt_pitch || pkt->rt_gpu_addr == pkt->depth.gpu_addr) &&
        pkt->depth.pitch >= pkt->rt_pitch && pkt->scissor[2] <= pkt->depth.pitch &&
        pkt->num_cb == 1 && pkt->aa_samples == 1 && (r300_reg(st, RB3D_COLOR_CHANNEL_MASK) & 0xF) == 0) {
        pkt->rt_dummy = true;
        pkt->rt_pitch = pkt->rt_width = pkt->depth.pitch;
    }

    /* Vertex order */
    f.st = st;
    f.arr = arr;
    f.read = read;
    f.opaque = opaque;
    f.swap = r300_reg(st, VAP_CNTL_STATUS) & 3;
    f.vtx_size = r300_reg(st, VAP_VTX_SIZE) & 0x7F;
    f.immd = immd;
    f.immd_dw = immd_dw;

    /* Vertex program */
    for (int i = 0; i < 256; i++) {
        memcpy(consts[i], &st->pvs_mem[(R300_PVS_CONST_START + (cc & 0xFF) + i) * 4],
               sizeof(consts[i]));
        if (R300_PVS_CONST_START + (cc & 0xFF) + i + 1 >= R300_PVS_MEM_VECS) {
            break;
        }
    }
    prog.code = &st->pvs_mem[R300_PVS_CODE_START * 4];
    prog.first_inst = cntl0 & 0x3FF;
    prog.last_inst = (cntl0 >> 20) & 0x3FF;
    prog.consts = (const float (*)[4])consts;
    prog.max_const = (cc >> 16) & 0xFF;
    prog.fc_opc = r300_reg(st, VAP_PVS_FLOW_CNTL_OPC);
    for (int i = 0; i < 16; i++) {
        prog.fc_addrs[i] = r300_reg(st, VAP_PVS_FLOW_CNTL_ADDRS_0 + 4 * i);
        prog.fc_loop[i] = r300_reg(st, VAP_PVS_FLOW_CNTL_LOOP_INDEX_0 + 4 * i);
    }

    desc.vs_gpu = NULL;
    r300_pvs_prepare(&prog);
    output_layout(st, &lay);
    rs_route(st, &route, &desc, &pkt->warn);

    if (!((prim == 8)) && !bypass && !pvsrec_wanted(n) && gpu_vs_wanted(st, prim, false, bypass)) {
        char *vs = NULL;
        int r = gpu_vs_build(st, &f, &prog, (const float (*)[4])consts, &lay, &route, pkt, order, n, prim,
                             fog, fogsel, clip, &vs, err);

        if (r) {
            free(order);
            if (r < 0) {
                return false;
            }
            pkt->warn |= f.warn;
            desc.vs_gpu = vs;
            pkt->glsl = r300_us_to_glsl(st, &desc, st->glsl_flags, err);
            free(vs);
            if (!pkt->glsl) {
                r300_draw_free(pkt);
                return false;
            }
            return true;
        }
    }

    /* Rectangle lists: three corners a rectangle, the card completes the
     * parallelogram (v0 + v2 - v1) and draws it as a quad. */
    bool rects = prim == 8;
    uint32_t nsrc = rects ? n / 3 * 4 : n;

    /* Transform every vertex in draw order (cheap next to the GPU work). */
    R300Vertex *xv = calloc(nsrc ? nsrc : 1, sizeof(R300Vertex));
    float (*outs)[R300_PVS_NUM_OUTPUTS][4] = calloc(nsrc ? nsrc : 1, sizeof(*outs));
    bool rec = !rects && !bypass && pvsrec_wanted(n);
    float (*cin)[R300_PVS_NUM_INPUTS][4] = rec ? calloc(n, sizeof(*cin)) : NULL;
    /*
     * Fetch every vertex first, then run the vertex program over all of them at once (r300_pvs_run_batch splits a
     * big batch across worker threads).  oidx[k] is the output slot of the k-th vertex; a rectangle's fourth
     * corner is derived from the other three once their outputs exist.
     */
    float (*ins)[R300_PVS_NUM_INPUTS][4] = bypass || !n ? NULL : malloc(n * sizeof(*ins));
    uint32_t *oidx = bypass || !n ? NULL : malloc(n * sizeof(*oidx));
    uint32_t *corner = rects ? malloc((n / 3 + 1) * sizeof(*corner)) : NULL;
    uint32_t nrun = 0, ncorner = 0;

    if (!bypass && n && (!ins || !oidx)) {
        *err = "out of memory";
        free(ins);
        free(oidx);
        free(corner);
        free(cin);
        free(outs);
        free(xv);
        free(order);
        return false;
    }
    for (uint32_t i = 0, j = 0; i < n && j < nsrc; i++, j++) {
        float in_local[R300_PVS_NUM_INPUTS][4];
        float (*in)[4] = ins ? ins[nrun] : in_local;

        if (!fetch_vertex(&f, order[i], in)) {
            *err = f.why ? f.why : "vertex data out of range";
            free(ins);
            free(oidx);
            free(corner);
            free(cin);
            free(outs);
            free(xv);
            free(order);
            return false;
        }
        if (cin) {
            memcpy(cin[i], in, sizeof(cin[i]));
        }
        if (bypass) {
            memcpy(outs[j], in, sizeof(outs[j]));
        } else {
            oidx[nrun++] = j;
        }
        if (rects && i % 3 == 2) {
            corner[ncorner++] = ++j;
        }
    }
    if (nrun) {
        uint32_t u = r300_pvs_run_batch(&prog, nrun, (const float (*)[R300_PVS_NUM_INPUTS][4])ins, outs, oidx);

        if (u & R300_PVS_UNSUP_FLOW) {
            pkt->warn |= R300_WARN_FLOW;
        }
        if (u & ~R300_PVS_UNSUP_FLOW) {
            pkt->warn |= R300_WARN_PVS;
        }
    }
    for (uint32_t k = 0; k < ncorner; k++) {
        uint32_t j = corner[k];

        for (int o = 0; o < R300_PVS_NUM_OUTPUTS; o++) {
            for (int c = 0; c < 4; c++) {
                outs[j][o][c] = outs[j - 3][o][c] + outs[j - 1][o][c] - outs[j - 2][o][c];
            }
        }
    }
    free(ins);
    free(oidx);
    free(corner);
    if (cin) {
        if (pvsrec.n || !pvsrec_seen(&prog, (const float (*)[4])consts)) {
            pvsrec_write(&prog, (const float (*)[4])consts, !pvsrec.n && n > 64 ? 64 : n, cin, outs);
        }
        free(cin);
    }
    for (uint32_t j = 0; j < nsrc; j++) {
        float (*out)[4] = outs[j];
        float cols[4][4];

        viewport_xform(st, pkt, out[lay.pos], xv[j].pos);
        vtx_colors(&lay, out, cols);
        rs_values(st, &lay, &route, out, (const float (*)[4])cols, NULL, xv[j].v);
        if (fog) {
            float src = fogsel < 4 ? (lay.color[fogsel] >= 0 ? out[lay.color[fogsel]][3] : 0.0f)
                      : fogsel == 4 ? out[lay.pos][3]
                      : xv[j].pos[2] / (xv[j].pos[3] != 0.0f ? xv[j].pos[3] : 1.0f);
            xv[j].aux[0] = src * fog_scale + fog_off;
        }
        /* User clip planes (VAP_CLIP_CNTL.UCP_ENA_n): the distance of the
         * clip-space position from plane n in PVS memory 1024 + n. */
        for (int k = 0; k < 6; k++) {
            const float *pl = (const float *)&st->pvs_mem[(R300_PVS_UCP_START + k) * 4];
            const float *cp = out[lay.pos];
            xv[j].ucp[k] = (clip >> k) & 1 ? cp[0] * pl[0] + cp[1] * pl[1] +
                                             cp[2] * pl[2] + cp[3] * pl[3] : 1.0f;
        }
    }
    if (rects) {
        prim = 13;
        n = nsrc;
    }
    pkt->warn |= f.warn;

    /* Primitive assembly */
    memset(&b, 0, sizeof(b));
    b.st = st;
    b.pkt = pkt;
    b.lay = &lay;
    b.route = &route;
    b.outs = outs;
    b.xv = xv;
    b.n = n;
    b.colctl = r300_reg(st, GA_COLOR_CONTROL);
    b.back_colors = (r300_reg(st, VAP_OUTPUT_VTX_FMT_0) & (3u << 3)) != 0;
    for (int c = 0; c < 4 && !b.fancy_colors; c++) {
        b.fancy_colors = ((b.colctl >> (4 * c)) & 3) != 2 ||
                         ((b.colctl >> (4 * c + 2)) & 3) != 2;
    }
    b.fancy_colors |= b.back_colors;
    b.solid[0] = s312(r300_reg(st, GA_SOLID_RG) >> 16);
    b.solid[1] = s312(r300_reg(st, GA_SOLID_RG));
    b.solid[2] = s312(r300_reg(st, GA_SOLID_BA) >> 16);
    b.solid[3] = s312(r300_reg(st, GA_SOLID_BA));
    b.poly_mode = (r300_reg(st, GA_POLY_MODE) & 3) == 1 ? r300_reg(st, GA_POLY_MODE) : 0;
    {
        uint32_t lc = r300_reg(st, GA_LINE_CNTL);
        uint32_t sc = r300_reg(st, GA_LINE_STIPPLE_CONFIG);
        uint32_t ps = r300_reg(st, GA_POINT_SIZE);
        uint32_t mm = r300_reg(st, GA_POINT_MINMAX);
        b.line_hw = (lc & 0xFFFF) / 12.0f;
        b.line_end = (lc >> 16) & 3;
        b.stip_scale = bits_to_float(sc & ~3u);
        b.stip_reset = sc & 3;
        b.stip_acc = b.stip_reset ? 0.0f : (float)(r300_reg(st, GA_LINE_STIPPLE_VALUE) & 0xFFFFFF);
        b.point_hw = ((ps >> 16) & 0xFFFF) / 12.0f;
        b.point_hh = (ps & 0xFFFF) / 12.0f;
        b.point_min = (mm & 0xFFFF) / 6.0f;
        b.point_max = (mm >> 16) / 6.0f;
        b.point_psize = lay.psize >= 0;
    }
    /* The setup unit culls polygons only (triangles, fans, strips, quads,
     * quad strips, polygons).  Metal culls filled triangles; the CPU does
     * it when polygon mode turns them into lines or points. */
    pkt->cull = ((1u << prim) & 0xE0F0u) ? r300_reg(st, SU_CULL_MODE) & 7 : 0;
    if (b.poly_mode && pkt->cull) {
        b.cull = pkt->cull;
        pkt->cull &= R300_FACE_CW;
    }

    list = malloc(sizeof(uint32_t) * (n * 3 + 6));
    prov = malloc(sizeof(R300Prov) * (n * 2 + 2));
    uint32_t cls;
    nidx = r300_assemble_prov(prim, n, list, &cls, prov);
    if (!nidx) {
        if (prim != 0) {
            pkt->warn |= R300_WARN_PRIM;
        }
        free(prov);
        free(list);
        free(outs);
        free(xv);
        free(order);
        *err = "primitive type not supported";
        return false;
    }
    if (cls == 0) {
        for (uint32_t i = 0, t = 0; i + 2 < nidx; i += 3, t++) {
            emit_tri(&b, &list[i], &prov[t]);
        }
    } else if (cls == 1) {
        if (b.stip_reset == 2) {
            b.stip_acc = 0.0f;
        }
        for (uint32_t i = 0, t = 0; i + 1 < nidx; i += 2, t++) {
            emit_line(&b, list[i], list[i + 1], &prov[t], false);
        }
    } else {
        for (uint32_t i = 0; i < nidx; i++) {
            emit_point(&b, list[i], &prov[i], false);
        }
    }
    if (!b.ntri && b.nline) {
        pkt->verts = b.line;
        pkt->num_verts = b.nline;
        pkt->prim_class = 1;
        free(b.tri);
    } else {
        if (b.nline) {
            b.tri = realloc(b.tri, sizeof(R300Vertex) * (b.ntri + b.nline));
            memcpy(b.tri + b.ntri, b.line, sizeof(R300Vertex) * b.nline);
        }
        free(b.line);
        pkt->verts = b.tri;
        pkt->num_verts = b.ntri;
        pkt->num_line_verts = b.nline;
        pkt->prim_class = 0;
    }
    free(prov);
    free(list);
    free(outs);
    free(xv);
    free(order);

    pkt->glsl = r300_us_to_glsl(st, &desc, st->glsl_flags, err);
    if (!pkt->glsl) {
        r300_draw_free(pkt);
        return false;
    }
    return true;
}

bool r300_draw_build(const R300State *st, const R300Arrays *arr,
                     uint32_t opcode, const uint32_t *d, uint32_t ndw,
                     R300ReadFn read, void *opaque,
                     R300DrawPacket *pkt, const char **err)
{
    const uint32_t *payload = d + 1;
    uint32_t payload_dw = ndw ? ndw - 1 : 0;
    uint32_t vf, n, *order;

    memset(pkt, 0, sizeof(*pkt));
    *err = NULL;
    if (ndw < 1) {
        *err = "empty draw packet";
        return false;
    }
    vf = d[0];
    n = vf >> 16;
    if (!n) {
        *err = "no vertices";
        return false;
    }
    order = malloc(sizeof(uint32_t) * n);
    if (opcode == 0x36) {                       /* DRAW_INDX_2, inline */
        bool i32 = (vf >> 11) & 1;
        for (uint32_t i = 0; i < n; i++) {
            order[i] = r300_index_at(payload, payload_dw, i32, i);
        }
    } else {                                    /* DRAW_VBUF_2, DRAW_IMMD_2 */
        for (uint32_t i = 0; i < n; i++) {
            order[i] = i;
        }
    }
    return draw_core(st, arr, vf, order, n, opcode == 0x35 ? payload : NULL,
                     opcode == 0x35 ? payload_dw : 0, read, opaque, pkt, err);
}

uint32_t r300_aa_samples(const R300State *st)
{
    static const uint8_t n[4] = { 2, 3, 4, 6 };
    uint32_t aa = r300_reg(st, GB_AA_CONFIG);

    return (aa & 1) ? n[(aa >> 1) & 3] : 1;
}

void r300_aa_positions(const R300State *st, float pos[6][2])
{
    uint64_t v = r300_reg(st, GB_MSPOS0) & 0xFFFFFF;

    v |= (uint64_t)(r300_reg(st, GB_MSPOS1) & 0xFFFFFF) << 24;
    for (int k = 0; k < 6; k++, v >>= 8) {
        pos[k][0] = ((float)(v & 0xF) - 6.0f) / 12.0f;
        pos[k][1] = ((float)((v >> 4) & 0xF) - 6.0f) / 12.0f;
    }
}

uint32_t r300_msaa_offset(uint32_t x, uint32_t y, uint32_t ns,
                          uint32_t pitch_px, uint32_t bpp, uint32_t sample)
{
    uint32_t sh = bpp == 2 ? 1 : 2;             /* log2(bytes per sample) */
    uint32_t pitch = pitch_px * ns;             /* in samples, as the driver keeps it */
    uint32_t base, idx;

    if ((ns != 2 && ns != 4) || sample >= ns) {
        return ~0u;
    }
    base = 32 * ((pitch / ns / 4) * ns * sh * 2 * (y >> 3) +
                 ((((x >> 2) << 1) | ((y >> 2) & 1)) * ns * sh));
    if (ns == 2) {
        idx = (((y >> 1) & 1) << 4) | (((x >> 1) & 1) << 3) | (sample << 2);
    } else {
        idx = (((((x >> 2) & 1) ^ ((y >> 1) & 1))) << 5) |
              (((x >> 1) & 1) << 4) | (sample << 2);
    }
    idx |= ((y & 1) << 1) | (x & 1);
    return base + (idx << sh);
}

uint32_t r300_index_at(const uint32_t *dw, uint32_t ndw, bool i32, uint32_t i)
{
    if (i32) {
        return i < ndw ? dw[i] : 0;
    }
    return i / 2 < ndw ? (dw[i / 2] >> (16 * (i & 1))) & 0xFFFF : 0;
}

bool r300_draw_build_indexed(const R300State *st, const R300Arrays *arr,
                             uint32_t vf, const R300Indices *idx,
                             R300ReadFn read, void *opaque,
                             R300DrawPacket *pkt, const char **err)
{
    uint32_t n = vf >> 16, *order, *sw;
    unsigned swap = r300_reg(st, VAP_CNTL_STATUS) & 3;
    bool i32 = (vf >> 11) & 1;

    memset(pkt, 0, sizeof(*pkt));
    *err = NULL;
    if (!n) {
        *err = "no vertices";
        return false;
    }
    if ((uint64_t)(i32 ? n : (n + 1) / 2) > idx->ndw) {
        *err = "index buffer shorter than the draw";
        return false;
    }
    sw = malloc(sizeof(uint32_t) * (idx->ndw ? idx->ndw : 1));
    for (uint32_t k = 0; k < idx->ndw; k++) {
        sw[k] = vc_swap(idx->dw[k], swap);
    }
    order = malloc(sizeof(uint32_t) * n);
    for (uint32_t i = 0; i < n; i++) {
        order[i] = r300_index_at(sw, idx->ndw, i32, i);
    }
    free(sw);
    return draw_core(st, arr, vf, order, n, NULL, 0, read, opaque, pkt, err);
}

void r300_draw_free(R300DrawPacket *pkt)
{
    free(pkt->glsl);
    free(pkt->verts);
    free(pkt->gpu_data);
    pkt->glsl = NULL;
    pkt->verts = NULL;
    pkt->gpu_data = NULL;
    for (int t = 0; t < R300_NUM_TEX_UNITS; t++) {
        free(pkt->tex[t].host_data);
        pkt->tex[t].host_data = NULL;
    }
}

uint64_t r300_hash_bytes(const uint8_t *p, size_t n)
{
    const uint64_t m = 0x9E3779B97F4A7C15ull;
    uint64_t h = n * m, v;
    size_t i = 0;

    for (; i + 8 <= n; i += 8) {
        memcpy(&v, p + i, 8);
        h = (h ^ v) * m;
        h ^= h >> 29;
    }
    for (; i < n; i++) {
        h = (h ^ p[i]) * m;
    }
    return h ^ (h >> 32);
}

/*
 * Identity of a texture's texels for the renderers' caches.  Hashing every
 * byte of a mip chain on every draw cost a third of the render thread in
 * Doom 3 (megabytes of DXT per draw), so large chains are sampled: both
 * ends plus a spread of words across the rest.  The driver uploads whole
 * textures, which changes every sampled stretch; $R300_TEX_FULLHASH=1
 * hashes everything.
 */
uint64_t r300_hash_tex(const uint8_t *p, size_t n)
{
    static int full = -1;
    uint64_t h;

    if (full < 0) {
        full = getenv("R300_TEX_FULLHASH") != NULL;
    }
    if (full || n <= 32768) {
        return r300_hash_bytes(p, n);
    }
    h = r300_hash_bytes(p, 8192) ^ (r300_hash_bytes(p + n - 8192, 8192) * 3);
    size_t step = (n / 2048) & ~(size_t)7;      /* 2048 spots, 8-byte aligned */
    for (size_t i = 8192; i + 8 <= n - 8192; i += step) {
        uint64_t v;
        memcpy(&v, p + i, 8);
        h = (h ^ v) * 0x9E3779B97F4A7C15ull;
        h ^= h >> 29;
    }
    return h ^ n;
}

