//! The backend's state: one Vulkan context per process, as in the C
//! backend this was ported from (ppc_mac_gpu_vulkan.c).

use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Condvar, Mutex};

use ash::vk;

use crate::sys::*;

pub const PAGE: u64 = PPC_MAC_GPU_DIRTY_PAGE as u64;
pub const MAX_IMG: usize = 128;
pub const MAX_TEXFULL: usize = 48;
pub const MAX_FB: usize = 64;
pub const MAX_RP: usize = 32;
pub const NBATCH: usize = 6;
pub const ARENA_CHUNK: u64 = 8 << 20;
pub const SETS_PER_POOL: u32 = 256;
pub const NTEX: usize = R300_NUM_TEX_UNITS as usize;
pub const MAX_ATT: usize = R300_US_MAX_TARGETS as usize + 1;
pub const MAX_DR: usize = 8;

pub type DoneFn = unsafe extern "C" fn(*mut c_void, u32);

/// A bump-allocated, host-visible slab for per-draw data and staging.
pub struct Chunk {
    pub buf: vk::Buffer,
    pub mem: vk::DeviceMemory,
    pub map: *mut u8,
    pub size: u64,
    pub off: u64,
}

pub enum Trash {
    Image(vk::Image),
    View(vk::ImageView),
    Mem(vk::DeviceMemory),
    Fb(vk::Framebuffer),
}

/// One command buffer's worth of work, with what it keeps alive.
pub struct Batch {
    /// 0 free, 1 open, 2 submitted
    pub state: u8,
    pub seq: u32,
    pub cb: vk::CommandBuffer,
    pub fence: vk::Fence,
    pub pools: Vec<vk::DescriptorPool>,
    pub pool_cur: usize,
    pub chunks: Vec<Chunk>,
    pub trash: Vec<Trash>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImgCls {
    Rt,
    Tex,
}

/// An image caching a range of guest VRAM.
pub struct Img {
    pub live: bool,
    pub cls: ImgCls,
    pub addr: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub fmt: vk::Format,
    pub bpp: u32,
    pub img: vk::Image,
    pub mem: vk::DeviceMemory,
    pub view: vk::ImageView,
    /// holds the contents of generation `gen`
    pub valid: bool,
    pub gen: u64,
    /// rendered into since its last write-back, in these rectangles
    pub dirty: bool,
    pub ndr: usize,
    pub dr: [[u32; 4]; MAX_DR],
    pub used: u64,
}

impl Img {
    pub fn empty() -> Img {
        Img {
            live: false,
            cls: ImgCls::Rt,
            addr: 0,
            width: 0,
            height: 0,
            pitch: 0,
            fmt: vk::Format::UNDEFINED,
            bpp: 0,
            img: vk::Image::null(),
            mem: vk::DeviceMemory::null(),
            view: vk::ImageView::null(),
            valid: false,
            gen: 0,
            dirty: false,
            ndr: 0,
            dr: [[0; 4]; MAX_DR],
            used: 0,
        }
    }

    /// One past the last VRAM byte the image covers.
    pub fn hi(&self) -> u64 {
        self.addr as u64
            + self.pitch as u64 * (self.height as u64 - 1)
            + self.width as u64 * self.bpp as u64
    }
}

/// Textures rebuilt on the CPU (mips, 3D, cube, 16bpp, DXT, GART copies).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TexKey {
    pub addr: u32,
    pub format: u32,
    pub kind: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub dim: u32,
    pub levels: u32,
    pub pitch: u32,
    pub host: u32,
    pub hash: u64,
}

pub struct TexFull {
    pub live: bool,
    pub key: TexKey,
    pub img: vk::Image,
    pub mem: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub used: u64,
}

pub struct Rp {
    pub fmt: Vec<vk::Format>,
    pub rp: vk::RenderPass,
}

pub struct Fb {
    pub rp: vk::RenderPass,
    pub n: usize,
    pub w: u32,
    pub h: u32,
    pub view: [vk::ImageView; MAX_ATT],
    pub fb: vk::Framebuffer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PipeKey {
    pub rp: vk::RenderPass,
    pub ncb: u32,
    pub z: bool,
    pub topo: i32,
    pub cull: u32,
    pub front: i32,
    /// the depth buffer is 16 bits (storage-image shaders name the format)
    pub z16: bool,
}

/// A program is named by its cached GLSL id when it has one, else by the
/// text itself.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ProgKey {
    Id(u32),
    Text(String),
}

/// The pipelines of one GLSL source.
#[derive(Default)]
pub struct Prog {
    pub mods: [vk::ShaderModule; 4],
    pub failed: [bool; 4],
    pub pipes: Vec<(PipeKey, vk::Pipeline)>,
}

#[derive(Clone)]
pub struct DumpStat {
    pub draws: u64,
    pub verts: u64,
    pub black: u64,
    pub v0min: [f32; 4],
    pub v0max: [f32; 4],
    pub v1min: [f32; 4],
    pub v1max: [f32; 4],
    pub first: u64,
    pub state: String,
    /// clip-space positions: vertices with w <= 0, with w < 1e-3 (>0), smallest w,
    /// z/w range, widest |x/w| or |y/w|, vertices outside 0<=z<=w
    pub w_neg: u64,
    pub w_tiny: u64,
    pub w_min: f32,
    pub zw_min: f32,
    pub zw_max: f32,
    pub xyw_max: f32,
    pub z_out: u64,
}

pub struct WaitItem {
    pub fence: vk::Fence,
    pub seq: u32,
    pub done: Option<DoneFn>,
    pub arg: usize,
}

/// What the completion thread and the vCPU thread share.
pub struct Shared {
    pub q: Mutex<VecDeque<WaitItem>>,
    pub wake: Condvar,
    pub idle: Condvar,
    /// every batch up to here has completed
    pub done_seq: AtomicU32,
    /// ... and the waiter is finished with it
    pub worker_seq: AtomicU32,
}

pub struct Vk {
    pub _entry: ash::Entry,
    pub inst: ash::Instance,
    pub pdev: vk::PhysicalDevice,
    pub props: vk::PhysicalDeviceProperties,
    pub memprops: vk::PhysicalDeviceMemoryProperties,
    pub dev: ash::Device,
    pub queue: vk::Queue,
    pub depth_clamp: bool,
    pub aniso: bool,
    pub bc: bool,
    pub mirror_clamp: bool,
    /// VK_EXT_rasterization_order_attachment_access: the shaders read the
    /// attachments they blend into and do the depth test by hand, so
    /// overlapping primitives of one draw must read and write them in order
    pub ordered: bool,
    /// VK_EXT_fragment_shader_interlock (pixel, ordered): what AMD and most GPUs
    /// offer instead; the fragment shaders then bracket their attachment
    /// read-modify-write with begin/endInvocationInterlockARB
    pub interlock: bool,
    /// colour and depth buffers are coherent storage images read and written
    /// inside the pixel interlock (R300_GLSL_FB_IMAGE), not input attachments:
    /// input-attachment reads do not see what earlier fragments of the same
    /// draw wrote, which broke depth testing and blending inside a draw
    pub fb_image: bool,
    pub align: u64,

    pub cmdpool: vk::CommandPool,
    pub dsl: vk::DescriptorSetLayout,
    pub layout: vk::PipelineLayout,

    // VRAM
    pub vram: *mut u8,
    pub vram_size: u64,
    pub vram_mem: vk::DeviceMemory,
    pub vram_buf: vk::Buffer,
    pub npages: u64,
    /// write generation of each page
    pub page_gen: Vec<u64>,
    /// batch that last wrote each page (GPU)
    pub page_wseq: Vec<u32>,
    /// batch that last read each page (GPU)
    pub page_rseq: Vec<u32>,
    pub dirty_bm: Vec<u64>,
    pub gen: u64,
    pub dirty_fn: PPCMacGPUDirtyFn,
    pub dirty_arg: *mut c_void,

    pub zpass_buf: vk::Buffer,
    pub zpass_mem: vk::DeviceMemory,
    pub zpass: *mut u32,

    // batches
    pub batch: Vec<Batch>,
    pub cur: Option<usize>,
    pub seq: u32,
    pub free_chunks: Vec<Chunk>,
    /// the arena chunk small allocations come from (index into the open
    /// batch's chunks)
    pub chunk: Option<usize>,
    pub shared: Arc<Shared>,

    // the open render pass
    pub in_pass: bool,
    pub pass_rp: vk::RenderPass,
    pub pass_n: usize,
    pub pass_img: [usize; MAX_ATT],
    pub pass_z: bool,
    pub pass_z16: bool,

    pub img: Vec<Img>,
    pub texfull: Vec<TexFull>,
    pub clock: u64,
    pub rps: Vec<Rp>,
    pub fbs: Vec<Option<Fb>>,
    pub fb_next: usize,
    pub progs: HashMap<ProgKey, Prog>,
    pub samplers: HashMap<u32, vk::Sampler>,
    pub dummy: [(vk::Image, vk::DeviceMemory, vk::ImageView); 3],
    pub dummy_ready: bool,

    pub warned: u32,
    /// PPCGPU_DUMP=path: one line per distinct kind of draw (and the GLSL of each program)
    pub dump: Option<(String, std::collections::HashSet<String>, std::collections::HashSet<u64>)>,
    pub dump_draws: u64,
    /// PPCGPU_RECORD=dir: draws recorded for offline replay (see record.rs)
    pub rec: Option<crate::record::Rec>,
    /// per draw kind: draws, v0 min/max, v1 min/max, black-colour vertices, vertices
    pub dump_stats: std::collections::HashMap<String, DumpStat>,
    pub stat_draws: u64,
    pub stat_passes: u64,
    pub stat_uploads: u64,
    pub stat_writebacks: u64,
    pub stat_flushes: u64,
}

static mut VK: *mut Vk = std::ptr::null_mut();
static ERR: Mutex<String> = Mutex::new(String::new());

/// The backend, once a context exists.
pub fn vk_opt() -> Option<&'static mut Vk> {
    // SAFETY: only ever set once, never freed; the device calls into the
    // backend from one thread at a time (the BQL, or the CP thread under it).
    unsafe {
        let p = VK;
        if p.is_null() {
            None
        } else {
            Some(&mut *p)
        }
    }
}

pub fn set_vk(v: Vk) -> &'static mut Vk {
    // SAFETY: see vk_opt.
    unsafe {
        VK = Box::into_raw(Box::new(v));
        &mut *VK
    }
}

pub fn fail(msg: &str) {
    eprintln!("ppc-mac-gpu vulkan: {msg}");
    *ERR.lock().unwrap() = msg.to_string();
}

pub fn last_error() -> String {
    ERR.lock().unwrap().clone()
}
