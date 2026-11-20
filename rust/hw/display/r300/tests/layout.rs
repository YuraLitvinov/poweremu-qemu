//! The `#[repr(C)]` mirrors must match the C headers (design
//! dc_rust_ffi_mirrors_checked).  `tests/golden/layout.txt` is generated from
//! `r300_draw.h` by `tools/gen_golden.sh`.

use std::collections::HashMap;
use std::mem::{align_of, offset_of, size_of};

use r300::state::{R300State, PVS_MEM_VECS, REG_COUNT};
use r300::us::{R300FSUniforms, R300Vertex};
use r300::types::{R300TexDesc, TexKind, TEX_MAX_LEVELS};

fn golden() -> HashMap<&'static str, usize> {
    include_str!("golden/layout.txt")
        .lines()
        .map(|l| {
            let (k, v) = l.split_once(' ').unwrap();
            (k, v.parse().unwrap())
        })
        .collect()
}

#[test]
fn tex_desc_layout_matches_header() {
    let g = golden();
    assert_eq!(size_of::<R300TexDesc>(), g["R300TexDesc.size"]);
    assert_eq!(align_of::<R300TexDesc>(), g["R300TexDesc.align"]);
    macro_rules! check {
        ($($f:ident),*) => {$(
            assert_eq!(offset_of!(R300TexDesc, $f), g[concat!("R300TexDesc.", stringify!($f))],
                       "offset of {}", stringify!($f));
        )*};
    }
    check!(
        bound, gpu_addr, width, height, endian, le, tile, pitch_bytes, format, kind, view_bpp,
        filter0, filter1, host_data, dim, depth, levels, lvl_off, lvl_pitch, lvl_rows, size_bytes
    );
}

#[test]
fn tex_kind_values_match_header() {
    let g = golden();
    for (name, kind) in [
        ("RGBA8", TexKind::Rgba8),
        ("R8", TexKind::R8),
        ("RG8", TexKind::Rg8),
        ("CONVERT16", TexKind::Convert16),
        ("RAW", TexKind::Raw),
        ("DXT1", TexKind::Dxt1),
        ("DXT3", TexKind::Dxt3),
        ("DXT5", TexKind::Dxt5),
    ] {
        assert_eq!(kind as usize, g[format!("R300_TEXK.{name}").as_str()], "{name}");
    }
    assert_eq!(TEX_MAX_LEVELS, g["R300_TEX_MAX_LEVELS"]);
}

#[test]
fn state_layout_matches_header() {
    let g = golden();
    assert_eq!(size_of::<R300State>(), g["R300State.size"]);
    assert_eq!(align_of::<R300State>(), g["R300State.align"]);
    assert_eq!(REG_COUNT, g["R300_REG_COUNT"]);
    assert_eq!(PVS_MEM_VECS, g["R300_PVS_MEM_VECS"]);
    macro_rules! check {
        ($($f:ident),*) => {$(
            assert_eq!(offset_of!(R300State, $f), g[concat!("R300State.", stringify!($f))],
                       "offset of {}", stringify!($f));
        )*};
    }
    check!(regs, pvs_mem, pvs_upload_dw, pvs_gen, draws, win_immd, le_screen_h, glsl_flags);
}

#[test]
fn uniform_block_and_vertex_layout_match_header() {
    let g = golden();
    assert_eq!(size_of::<R300FSUniforms>(), g["R300FSUniforms.size"]);
    assert_eq!(align_of::<R300FSUniforms>(), g["R300FSUniforms.align"]);
    assert_eq!(size_of::<R300Vertex>(), g["R300Vertex.size"]);
    assert_eq!(align_of::<R300Vertex>(), g["R300Vertex.align"]);
    macro_rules! check {
        ($ty:ident: $($f:ident),*) => {$(
            assert_eq!(offset_of!($ty, $f), g[concat!(stringify!($ty), ".", stringify!($f))],
                       "offset of {}.{}", stringify!($ty), stringify!($f));
        )*};
    }
    check!(R300FSUniforms: consts, blend_color, tex_swz, tex_info, cblend, ablend, chanmask, alpha_func,
        out_sel, rt_swap32, clip_rule, rt_endian, rop, cliprect, zinfo, zpass_count, poly_en, pad2,
        tex_border, tex_lod, tex_dim, fog_color, fog_blend, depth_src, pad3, poly_offset, tex_addr, tex_lvl);
    check!(R300Vertex: pos, v, aux, ucp);
}

#[test]
fn translator_constants_match_header() {
    use r300::us::*;
    let g = golden();
    for (name, value) in [
        ("R300_BIND.UNIFORMS", bind::UNIFORMS), ("R300_BIND.ZPASS", bind::ZPASS), ("R300_BIND.VERTS", bind::VERTS),
        ("R300_BIND.MS", bind::MS), ("R300_BIND.VRAM", bind::VRAM), ("R300_BIND.AUX", bind::AUX),
        ("R300_BIND.FB0", bind::FB0), ("R300_BIND.TEX0", bind::TEX0), ("R300_BIND.COUNT", bind::COUNT),
        ("R300_GLSL.VRAM_SSBO", GLSL_VRAM_SSBO), ("R300_GLSL.FB_IMAGE", GLSL_FB_IMAGE),
        ("R300_TEXF.SIGNED_X", TEXF_SIGNED_X), ("R300_TEXF.GAMMA", TEXF_GAMMA), ("R300_TEXF.POT_ROWS", TEXF_POT_ROWS),
        ("R300_ZFMT.ENDIAN_MASK", ZFMT_ENDIAN_MASK), ("R300_ZFMT.Z16", ZFMT_Z16),
    ] {
        assert_eq!(value as usize, g[name], "{name}");
    }
    for (name, value) in [
        ("R300_US.NUM_TEMPS", NUM_TEMPS), ("R300_US.NUM_CONSTS", NUM_CONSTS), ("R300_US.MAX_ALU", MAX_ALU),
        ("R300_US.MAX_TEX", MAX_TEX), ("R300_US.NUM_TEX_UNITS", NUM_TEX_UNITS), ("R300_US.NUM_VARYINGS", NUM_VARYINGS),
        ("R300_US.MAX_TARGETS", MAX_TARGETS),
    ] {
        assert_eq!(value, g[name], "{name}");
    }
    for (name, view) in [
        ("R300_RTV.NONE", CbView::None), ("R300_RTV.RGBA8", CbView::Rgba8), ("R300_RTV.R8U", CbView::R8u),
        ("R300_RTV.R16U", CbView::R16u), ("R300_RTV.R32U", CbView::R32u), ("R300_RTV.RG32U", CbView::Rg32u),
        ("R300_RTV.RGBA32U", CbView::Rgba32u),
    ] {
        assert_eq!(view as usize, g[name], "{name}");
    }
}

#[test]
fn draw_packet_layout_matches_header() {
    use r300::capi::{CArrays, CColorDesc, CDepthDesc, CDrawPacket, CIndices};
    let g = golden();
    assert_eq!(size_of::<CDrawPacket>(), g["R300DrawPacket.size"]);
    assert_eq!(align_of::<CDrawPacket>(), g["R300DrawPacket.align"]);
    macro_rules! check {
        ($($f:ident),*) => {$(
            assert_eq!(offset_of!(CDrawPacket, $f), g[concat!("R300DrawPacket.", stringify!($f))],
                       "offset of {}", stringify!($f));
        )*};
    }
    check!(rt_gpu_addr, rt_pitch, rt_width, rt_height, rt_format, rt_bpp, rt_dummy, harvest_hint, depth_min, depth_max,
        depth_clip, depth_dx, rt_view, scissor, num_cb, cb, depth, cull, aa_samples, aa_pos, glsl, uniforms, tex, verts,
        gpu_vs, gpu_data, gpu_words, num_verts, num_line_verts, prim_class, warn);
    assert_eq!(size_of::<CColorDesc>(), g["R300ColorDesc.size"]);
    assert_eq!(size_of::<CDepthDesc>(), g["R300DepthDesc.size"]);
    assert_eq!(offset_of!(CDepthDesc, gpu_addr), g["R300DepthDesc.gpu_addr"]);
    assert_eq!(size_of::<CArrays>(), g["R300Arrays.size"]);
    assert_eq!(offset_of!(CArrays, a), g["R300Arrays.a"]);
    assert_eq!(size_of::<CIndices>(), g["R300Indices.size"]);
    assert_eq!(offset_of!(CIndices, ndw), g["R300Indices.ndw"]);
}
