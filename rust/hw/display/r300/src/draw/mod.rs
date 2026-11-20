//! R300-family draw assembly (`r300_draw.c`): turns a `3D_DRAW_*` packet plus
//! the register state into a self-contained draw for the host renderer.
//!
//! Vertex fetch (`VAP_PROG_STREAM_CNTL`, immediate data or `3D_LOAD_VBPNTR`
//! arrays), the vertex program (PVS, on the CPU), the viewport transform
//! (`VAP_VTE_CNTL`, `SE_VPORT_*`), primitive assembly into a triangle, line or
//! point list, and the rasterizer routing (`RS_IP`/`RS_INST`) of vertex
//! outputs into fragment temporaries.
//!
//! The fragment program, blending and colour-buffer packing are compiled from
//! [`crate::us::to_glsl`]; the renderer only binds and draws.

pub mod assemble;
pub mod build;
pub mod emit;
pub mod fetch;
pub mod gpu_vs;
pub mod layout;
pub mod misc;
pub mod pvsrec;
pub mod regs;
pub mod route;
pub mod setup;

use crate::types::R300TexDesc;
use crate::us::{CbView, R300FSUniforms, R300Vertex, MAX_TARGETS, NUM_TEX_UNITS};

pub use build::{draw_build, draw_build_indexed, Indices};

pub const WARN_PVS: u32 = 1 << 0;
pub const WARN_DEPTH: u32 = 1 << 1;
pub const WARN_TEXFMT: u32 = 1 << 2;
pub const WARN_VTXFMT: u32 = 1 << 3;
pub const WARN_PRIM: u32 = 1 << 4;
pub const WARN_RTFMT: u32 = 1 << 5;
pub const WARN_VARYINGS: u32 = 1 << 6;
/// PVS flow control ran away.
pub const WARN_FLOW: u32 = 1 << 7;

/// `SU_CULL_MODE`
pub const CULL_FRONT: u32 = 1 << 0;
pub const CULL_BACK: u32 = 1 << 1;
/// Else counter-clockwise front.
pub const FACE_CW: u32 = 1 << 2;
pub(crate) const CULL_FACE_CW_MASK: u32 = FACE_CW;

/// Depth/stencil buffer (`ZB_*`).  It stays in guest memory in the card's
/// layout; the renderer tests and writes it in the fragment shader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DepthDesc {
    /// `ZB_CNTL` enables the Z or stencil test.
    pub attach: bool,
    /// `ZB_DEPTHOFFSET`.
    pub gpu_addr: u32,
    /// Pixels.
    pub pitch: u32,
    /// 2 (Z16, Z13E3) or 4 (Z24S8).
    pub bpp: u32,
    /// `ZB_FORMAT.DEPTHFORMAT`.
    pub format: u32,
}

/// Colour buffers 1-3 of a multiple-render-target draw (buffer 0 is the
/// packet's `rt_*` fields).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ColorDesc {
    /// `RB3D_COLOROFFSETn`.
    pub gpu_addr: u32,
    /// Pixels.
    pub pitch: u32,
    /// `RB3D_COLORPITCHn` format field.
    pub format: u32,
    pub bpp: u32,
    /// `RTV_*`.
    pub view: u32,
}

/// A self-contained draw for the renderer.
#[derive(Debug)]
pub struct DrawPacket {
    // Colour buffer 0
    pub rt_gpu_addr: u32,
    /// Pixels.
    pub rt_pitch: u32,
    pub rt_width: u32,
    pub rt_height: u32,
    /// `RB3D_COLORPITCH0` format field.
    pub rt_format: u32,
    /// Bytes per pixel.
    pub rt_bpp: u32,
    /// Colour writes are all masked and the guest's colour buffer is a dummy
    /// narrower than the depth buffer (Mesa binds a tiny one when a draw has
    /// only depth or stencil): `rt_pitch`/`rt_width` are the depth buffer's and
    /// the renderer puts the (never changed) colour target in scratch VRAM
    /// instead of at `rt_gpu_addr`.
    pub rt_dummy: bool,
    /// The renderer must collect the VRAM dirty log before this draw.
    pub harvest_hint: bool,
    /// Viewport depth range and z clipping.  `gl_Position.z` is clip-space z
    /// mapped to Vulkan's 0..w clip volume (GL: (z + w) / 2, D3D clip space
    /// `VAP_CNTL.DX_CLIP_SPACE_DEF`: z), and the viewport's depth range applies
    /// the hardware's zscale/zoffset: depth = depth_min + z_ndc * (depth_max -
    /// depth_min).  `depth_clip`: clip against the near and far planes
    /// (`VAP_CLIP_CNTL.CLIP_DISABLE` turns it off), else depth-clamp.
    pub depth_min: f32,
    pub depth_max: f32,
    pub depth_clip: bool,
    /// `VAP_CNTL.DX_CLIP_SPACE_DEF`: clip-space z is 0..w.
    pub depth_dx: bool,
    pub rt_view: CbView,
    /// x0, y0, x1, y1 (exclusive).
    pub scissor: [u32; 4],
    /// Colour buffers 1..num_cb-1 (render targets B-D the program writes, or
    /// `RB3D_CCTL` multiwrites); the depth buffer is bound after them, at
    /// colour attachment `num_cb`.
    pub num_cb: u32,
    pub cb: [ColorDesc; MAX_TARGETS],
    pub depth: DepthDesc,
    /// `SU_CULL_MODE` (0 for lines and points).
    pub cull: u32,
    /// Multisampling (`GB_AA_CONFIG`): with `aa_samples` > 1 every sample is
    /// rendered, as its own copy of the image.  Sample k of row y is row y *
    /// aa_samples + k of each colour and depth buffer, which fills the
    /// aa_samples-times footprint the driver allocates for them.  `aa_pos` is
    /// each sample's offset from the pixel centre in pixels (`GB_MSPOS`,
    /// twelfths of a pixel from the pixel's corner).
    pub aa_samples: u32,
    pub aa_pos: [[f32; 2]; 6],

    // Fragment stage
    /// The `us::to_glsl` source.
    pub glsl: String,
    pub uniforms: R300FSUniforms,
    pub tex: [R300TexDesc; NUM_TEX_UNITS],

    /// Geometry, expanded (no indices): `num_verts` vertices of a triangle or
    /// line list (`prim_class`), then `num_line_verts` more of a line list
    /// (polygon-mode edges drawn with filled polygons).  Points and wide lines
    /// arrive as triangles.  Empty when the GPU runs the vertex stage
    /// (`gpu_vs`).
    pub verts: Vec<R300Vertex>,
    /// The vertex program and the vertex-stage math run in the vertex shader
    /// ([`crate::pvs_glsl`]) instead of on the CPU.  `gpu_data` is the shader's
    /// buffer: see [`gpu_vs::GPU_IDX_BASE`] for its layout.
    pub gpu_vs: bool,
    pub gpu_data: Vec<u32>,
    pub num_verts: u32,
    pub num_line_verts: u32,
    /// 0 triangles, 1 lines (of `num_verts`).
    pub prim_class: u32,
    /// `WARN_*` bits for things skipped.
    pub warn: u32,
}

impl Default for DrawPacket {
    fn default() -> Self {
        Self {
            rt_gpu_addr: 0,
            rt_pitch: 0,
            rt_width: 0,
            rt_height: 0,
            rt_format: 0,
            rt_bpp: 0,
            rt_dummy: false,
            harvest_hint: false,
            depth_min: 0.0,
            depth_max: 0.0,
            depth_clip: false,
            depth_dx: false,
            rt_view: CbView::None,
            scissor: [0; 4],
            num_cb: 0,
            cb: [ColorDesc::default(); MAX_TARGETS],
            depth: DepthDesc::default(),
            cull: 0,
            aa_samples: 0,
            aa_pos: [[0.0; 2]; 6],
            glsl: String::new(),
            uniforms: R300FSUniforms::default(),
            tex: core::array::from_fn(|_| R300TexDesc::default()),
            verts: Vec::new(),
            gpu_vs: false,
            gpu_data: Vec::new(),
            num_verts: 0,
            num_line_verts: 0,
            prim_class: 0,
            warn: 0,
        }
    }
}
