/*
 * R300-family PVS programs as GLSL.
 *
 * r300_pvs_to_glsl() turns a prepared program (r300_pvs_prepare) into the text of
 *
 *     void r300_pvs(vec4 IN[32], inout vec4 OUT[32])
 *
 * that does what r300_pvs_run() does for one vertex.  The program's constants are read from a global
 * `pvs_consts[]` array of vec4 that the consumer declares (a buffer or a uniform block); entries past
 * the program's max_const are never read, as in the interpreter.  OUT must be zero-filled by the caller
 * (outputs the program does not write keep their previous contents).
 *
 * Only straight-line programs are handled; anything the translator does not do is reported through
 * the return value and the caller keeps running the interpreter for that draw.
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 */

#ifndef HW_DISPLAY_R300_PVS_GLSL_H
#define HW_DISPLAY_R300_PVS_GLSL_H

#include "r300_pvs.h"

#define R300_PVS_GLSL_OK        0
#define R300_PVS_GLSL_FLOW      (1u << 0)   /* loops, jumps or subroutines */
#define R300_PVS_GLSL_PRED      (1u << 1)   /* R5xx predication */
#define R300_PVS_GLSL_OPCODE    (1u << 2)   /* an opcode the interpreter does not know */

/* Returns a malloc'd string, or NULL with *unsup set to the R300_PVS_GLSL_* reasons. */
char *r300_pvs_to_glsl(const R300PVSProgram *prog, uint32_t *unsup);

#endif
