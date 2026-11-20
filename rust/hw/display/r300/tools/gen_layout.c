/* Prints the C layout of the structs Rust mirrors, one "name offset" line each.
 * Regenerate with tools/gen_golden.sh after every change to r300_draw.h. */
#include <stddef.h>
#include <stdio.h>
#include "r300_texel.h"
#include "r300_us.h"
#include "r300_draw.h"
#define F(f) printf("R300TexDesc.%s %zu\n", #f, offsetof(R300TexDesc, f))
int main(void)
{
    printf("R300TexDesc.size %zu\n", sizeof(R300TexDesc));
    printf("R300TexDesc.align %zu\n", _Alignof(R300TexDesc));
    F(bound); F(gpu_addr); F(width); F(height); F(endian); F(le); F(tile);
    F(pitch_bytes); F(format); F(kind); F(view_bpp); F(filter0); F(filter1);
    F(host_data); F(dim); F(depth); F(levels); F(lvl_off); F(lvl_pitch);
    F(lvl_rows); F(size_bytes);
    printf("R300State.size %zu\nR300State.align %zu\n", sizeof(R300State), _Alignof(R300State));
#define S(f) printf("R300State.%s %zu\n", #f, offsetof(R300State, f))
    S(regs); S(pvs_mem); S(pvs_upload_dw); S(pvs_gen); S(draws); S(win_immd);
    S(le_screen_h); S(glsl_flags);
    printf("R300_REG_COUNT %d\nR300_PVS_MEM_VECS %d\n", R300_REG_COUNT, R300_PVS_MEM_VECS);
    printf("R300FSUniforms.size %zu\nR300FSUniforms.align %zu\n", sizeof(R300FSUniforms), _Alignof(R300FSUniforms));
#define U(f) printf("R300FSUniforms.%s %zu\n", #f, offsetof(R300FSUniforms, f))
    U(consts); U(blend_color); U(tex_swz); U(tex_info); U(cblend); U(ablend); U(chanmask); U(alpha_func);
    U(out_sel); U(rt_swap32); U(clip_rule); U(rt_endian); U(rop); U(cliprect); U(zinfo); U(zpass_count);
    U(poly_en); U(pad2); U(tex_border); U(tex_lod); U(tex_dim); U(fog_color); U(fog_blend); U(depth_src);
    U(pad3); U(poly_offset); U(tex_addr); U(tex_lvl);
    printf("R300Vertex.size %zu\nR300Vertex.align %zu\n", sizeof(R300Vertex), _Alignof(R300Vertex));
#define X(f) printf("R300Vertex.%s %zu\n", #f, offsetof(R300Vertex, f))
    X(pos); X(v); X(aux); X(ucp);
    printf("R300DrawPacket.size %zu\nR300DrawPacket.align %zu\n", sizeof(R300DrawPacket), _Alignof(R300DrawPacket));
#define P(f) printf("R300DrawPacket.%s %zu\n", #f, offsetof(R300DrawPacket, f))
    P(rt_gpu_addr); P(rt_pitch); P(rt_width); P(rt_height); P(rt_format); P(rt_bpp); P(rt_dummy); P(harvest_hint);
    P(depth_min); P(depth_max); P(depth_clip); P(depth_dx); P(rt_view); P(scissor); P(num_cb); P(cb); P(depth); P(cull);
    P(aa_samples); P(aa_pos); P(glsl); P(uniforms); P(tex); P(verts); P(gpu_vs); P(gpu_data); P(gpu_words);
    P(num_verts); P(num_line_verts); P(prim_class); P(warn);
    printf("R300ColorDesc.size %zu\nR300DepthDesc.size %zu\nR300Arrays.size %zu\nR300Indices.size %zu\n",
           sizeof(R300ColorDesc), sizeof(R300DepthDesc), sizeof(R300Arrays), sizeof(R300Indices));
    printf("R300Arrays.a %zu\nR300Indices.ndw %zu\nR300DepthDesc.gpu_addr %zu\n",
           offsetof(R300Arrays, a), offsetof(R300Indices, ndw), offsetof(R300DepthDesc, gpu_addr));
    printf("R300_BIND.UNIFORMS %d\nR300_BIND.ZPASS %d\nR300_BIND.VERTS %d\nR300_BIND.MS %d\nR300_BIND.VRAM %d\n"
           "R300_BIND.AUX %d\nR300_BIND.FB0 %d\nR300_BIND.TEX0 %d\nR300_BIND.COUNT %d\n",
           R300_BIND_UNIFORMS, R300_BIND_ZPASS, R300_BIND_VERTS, R300_BIND_MS, R300_BIND_VRAM, R300_BIND_AUX,
           R300_BIND_FB0, R300_BIND_TEX0, R300_BIND_COUNT);
    printf("R300_US.NUM_TEMPS %d\nR300_US.NUM_CONSTS %d\nR300_US.MAX_ALU %d\nR300_US.MAX_TEX %d\nR300_US.NUM_TEX_UNITS %d\n"
           "R300_US.NUM_VARYINGS %d\nR300_US.MAX_TARGETS %d\n",
           R300_US_NUM_TEMPS, R300_US_NUM_CONSTS, R300_US_MAX_ALU, R300_US_MAX_TEX, R300_NUM_TEX_UNITS,
           R300_NUM_VARYINGS, R300_US_MAX_TARGETS);
    printf("R300_RTV.NONE %d\nR300_RTV.RGBA8 %d\nR300_RTV.R8U %d\nR300_RTV.R16U %d\nR300_RTV.R32U %d\nR300_RTV.RG32U %d\nR300_RTV.RGBA32U %d\n",
           R300_RTV_NONE, R300_RTV_RGBA8, R300_RTV_R8U, R300_RTV_R16U, R300_RTV_R32U, R300_RTV_RG32U, R300_RTV_RGBA32U);
    printf("R300_GLSL.VRAM_SSBO %u\nR300_GLSL.FB_IMAGE %u\nR300_TEXF.SIGNED_X %u\nR300_TEXF.GAMMA %u\nR300_TEXF.POT_ROWS %u\n"
           "R300_ZFMT.ENDIAN_MASK %u\nR300_ZFMT.Z16 %u\n",
           R300_GLSL_VRAM_SSBO, R300_GLSL_FB_IMAGE, R300_TEXF_SIGNED_X, R300_TEXF_GAMMA, R300_TEXF_POT_ROWS,
           R300_ZFMT_ENDIAN_MASK, R300_ZFMT_Z16);
    printf("R300_TEXK.RGBA8 %d\nR300_TEXK.R8 %d\nR300_TEXK.RG8 %d\nR300_TEXK.CONVERT16 %d\n"
           "R300_TEXK.RAW %d\nR300_TEXK.DXT1 %d\nR300_TEXK.DXT3 %d\nR300_TEXK.DXT5 %d\n"
           "R300_TEX_MAX_LEVELS %d\n",
           R300_TEXK_RGBA8, R300_TEXK_R8, R300_TEXK_RG8, R300_TEXK_CONVERT16,
           R300_TEXK_RAW, R300_TEXK_DXT1, R300_TEXK_DXT3, R300_TEXK_DXT5,
           R300_TEX_MAX_LEVELS);
    return 0;
}
