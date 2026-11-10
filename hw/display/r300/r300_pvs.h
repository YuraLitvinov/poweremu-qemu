/*
 * R300-family Programmable Vertex Shader (PVS).
 *
 * The PVS executes 128-bit instructions (one opcode/destination dword and
 * three source dwords).  Encodings and opcode semantics follow the AMD
 * "R5xx Acceleration" guide, section 7.5, restricted to what R3xx/R4xx
 * implement.
 *
 * This is a reference interpreter: the device runs vertex programs on the
 * host CPU and hands Metal post-transform vertices.  Pure C, no QEMU
 * dependencies.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#ifndef HW_DISPLAY_R300_PVS_H
#define HW_DISPLAY_R300_PVS_H

#include <stdbool.h>
#include <stdint.h>

#define R300_PVS_MAX_INSTS      256     /* R3xx/R4xx code memory, in insts */
#define R300_PVS_NUM_INPUTS     32
#define R300_PVS_NUM_OUTPUTS    32
#define R300_PVS_NUM_TEMPS      128
#define R300_PVS_NUM_ALT_TEMPS  20

/* Opcodes and operand kinds (shared with the GLSL translator, r300_pvs_glsl.c). */
/* Vector engine opcodes */
enum {
    VE_NOP = 0, VE_DOT_PRODUCT, VE_MULTIPLY, VE_ADD, VE_MULTIPLY_ADD,
    VE_DISTANCE_VECTOR, VE_FRACTION, VE_MAXIMUM, VE_MINIMUM,
    VE_SET_GREATER_THAN_EQUAL, VE_SET_LESS_THAN, VE_MULTIPLYX2_ADD,
    VE_MULTIPLY_CLAMP, VE_FLT2FIX_DX, VE_FLT2FIX_DX_RND,
    /* R5xx, harmless to implement */
    VE_COND_MUX_EQ = 23, VE_COND_MUX_GT, VE_COND_MUX_GTE,
    VE_SET_GREATER_THAN, VE_SET_EQUAL, VE_SET_NOT_EQUAL,
};

/* Math engine opcodes */
enum {
    ME_NOP = 0, ME_EXP_BASE2_DX, ME_LOG_BASE2_DX, ME_EXP_BASEE_FF,
    ME_LIGHT_COEFF_DX, ME_POWER_FUNC_FF, ME_RECIP_DX, ME_RECIP_FF,
    ME_RECIP_SQRT_DX, ME_RECIP_SQRT_FF, ME_MULTIPLY, ME_EXP_BASE2_FULL_DX,
    ME_LOG_BASE2_FULL_DX, ME_POWER_FUNC_FF_CLAMP_B,
    ME_POWER_FUNC_FF_CLAMP_B1, ME_POWER_FUNC_FF_CLAMP_01, ME_SIN, ME_COS,
    ME_LOG_BASE2_IEEE, ME_RECIP_IEEE, ME_RECIP_SQRT_IEEE,
};

enum { SRC_TEMP = 0, SRC_INPUT, SRC_CONST, SRC_ALT };
enum { DST_TEMP = 0, DST_A0, DST_OUT, DST_OUT_REPL_X, DST_ALT, DST_INPUT };


/* A source operand decoded once per draw (r300_pvs_prepare) instead of once per vertex. */
typedef struct R300PVSSrc {
    uint8_t type;           /* SRC_TEMP, SRC_INPUT, SRC_CONST, SRC_ALT */
    uint8_t mode;           /* ADDR_MODE: 0 absolute, 1 A0 relative, 2 loop index */
    uint8_t sel;            /* A0 component (s >> 29) */
    uint8_t swz[4];         /* component select: 0-3 channel, 5 one, else zero */
    uint16_t offset;
    bool plain;             /* identity swizzle, no abs, no negate: use the register in place */
    uint32_t absmask;       /* 0x7fffffff with abs, else 0xffffffff */
    uint32_t neg[4];        /* 0x80000000 where negated */
} R300PVSSrc;

typedef struct R300PVSInst {
    uint32_t d[4];          /* the raw instruction */
    R300PVSSrc src[3];
} R300PVSInst;

typedef struct R300PVSProgram {
    const uint32_t *code;   /* PVS code memory, 4 dwords per inst */
    unsigned first_inst;    /* VAP_PVS_CODE_CNTL_0.PVS_FIRST_INST */
    unsigned last_inst;     /* VAP_PVS_CODE_CNTL_0.PVS_LAST_INST */
    const float (*consts)[4];   /* constant memory, from CONST_BASE_OFFSET */
    int max_const;          /* reads beyond this return 0 */
    /* Flow control (AMD R5xx guide 7.5.5): VAP_PVS_FLOW_CNTL_OPC (2 bits
     * per instruction: 0 none, 1 JUMP, 2 LOOP, 3 JSR), _ADDRS and
     * _LOOP_INDEX.  Zero fc_opc for straight-line programs. */
    uint32_t fc_opc;
    uint32_t fc_addrs[16];
    uint32_t fc_loop[16];
    /* Filled by r300_pvs_prepare: */
    unsigned ntemps;        /* temporaries a vertex can read before writing: zeroed per vertex */
    bool uses_alt;          /* the program reads or writes the ATRM registers */
    uint32_t in_mask;       /* input vertex slots the program reads (all of them when addressed relatively) */
    R300PVSInst dec[R300_PVS_MAX_INSTS];
} R300PVSProgram;

/* Decode the program once (code, first_inst, last_inst, flow control must be set); call before r300_pvs_run. */
void r300_pvs_prepare(R300PVSProgram *prog);

/*
 * Run the program for one vertex.  in[] is the input vertex memory (IVM),
 * out[] receives the output vertex memory (OVM); outputs the program does
 * not write keep their previous contents.  Returns a bitmask of flags for
 * things the interpreter does not implement (0 when fully handled).
 */
#define R300_PVS_UNSUP_FLOW     (1u << 0)   /* flow control ran past its step limit */
#define R300_PVS_UNSUP_PRED     (1u << 1)   /* R5xx predication */
#define R300_PVS_UNSUP_OPCODE   (1u << 2)   /* unknown opcode */
#define R300_PVS_UNSUP_RELDST   (1u << 3)   /* relative destination address */

uint32_t r300_pvs_run(const R300PVSProgram *prog,
                      const float in[R300_PVS_NUM_INPUTS][4],
                      float out[R300_PVS_NUM_OUTPUTS][4]);

/*
 * Run the program for n vertices: vertex i reads in[i] and writes out[oidx[i]].  Large batches are split across
 * a small pool of worker threads; the results are exactly those of n calls to r300_pvs_run.  Returns the OR of
 * their flags.
 */
uint32_t r300_pvs_run_batch(const R300PVSProgram *prog, unsigned n,
                            const float (*in)[R300_PVS_NUM_INPUTS][4],
                            float (*out)[R300_PVS_NUM_OUTPUTS][4],
                            const uint32_t *oidx);

/* One line of disassembly per instruction into buf; for traces. */
void r300_pvs_disasm_inst(const uint32_t d[4], char *buf, unsigned len);

#endif
