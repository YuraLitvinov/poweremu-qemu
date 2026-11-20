#!/bin/sh
# Regenerate tests/golden/*.txt from the C originals in ../../hw/display/r300.
# Run after any change to r300_draw.h or the C texel code.
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
c=$here/../r300
tmp=$(mktemp -d)
cc=${CC:-clang}
$cc -std=gnu11 -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_layout.c" -o "$tmp/layout"
$cc -std=gnu11 -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_texel.c" "$c/r300_texel.c" "$c/r300_texel_le.c" -o "$tmp/texel"
$cc -std=gnu11 -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_state.c" "$c/r300_state.c" -o "$tmp/state"
"$tmp/state" > "$here/tests/golden/state.txt"
"$tmp/state" dump > "$here/tests/golden/state_dump.txt"
# fmaxf/fminf of -0 and +0 is unspecified, and a vectorised loop (maxps) picks
# the other operand than scalar code does; pin the reference to scalar code,
# which is what Rust's f32::max/min give.
$cc -std=gnu11 -O2 -fno-vectorize -fno-slp-vectorize -ffp-contract=off -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_pvs.c" "$c/r300_pvs.c" "$c/r300_pvs_glsl.c" -o "$tmp/pvs" -lm -lpthread
"$tmp/pvs" > "$here/tests/golden/pvs.txt"
$cc -std=gnu11 -DPPC_MAC_GPU_BE=0 -I"$c" $(pkg-config --cflags shaderc spirv-cross-c-shared) "$here/tools/gen_spirv.c" "$c/r300_spirv.c" -o "$tmp/spirv" $(pkg-config --libs shaderc spirv-cross-c-shared)
"$tmp/spirv" "$here/tests/golden/shaders" > "$here/tests/golden/spirv.txt"
$cc -std=gnu11 -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_us.c" "$c/r300_us.c" "$c/r300_state.c" -o "$tmp/us" -lm
"$tmp/us" > "$here/tests/golden/us.txt"
$cc -std=gnu11 -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_draw_misc.c" "$c/r300_draw.c" "$c/r300_pvs.c" "$c/r300_pvs_glsl.c" "$c/r300_us.c" "$c/r300_state.c" "$c/r300_texel.c" "$c/r300_texel_le.c" -o "$tmp/dmisc" -lm -lpthread
"$tmp/dmisc" > "$here/tests/golden/draw_misc.txt"
$cc -std=gnu11 -O2 -fno-vectorize -fno-slp-vectorize -ffp-contract=off -DPPC_MAC_GPU_BE=0 -I"$c" "$here/tools/gen_draw.c" "$c/r300_draw.c" "$c/r300_pvs.c" "$c/r300_pvs_glsl.c" "$c/r300_us.c" "$c/r300_state.c" "$c/r300_texel.c" "$c/r300_texel_le.c" -o "$tmp/draw" -lm -lpthread
"$tmp/draw" > "$here/tests/golden/draw.txt" 2> "$tmp/draw.err" || { cat "$tmp/draw.err"; exit 1; }
"$tmp/layout" > "$here/tests/golden/layout.txt"
"$tmp/texel" > "$here/tests/golden/texel.txt"
rm -rf "$tmp"
