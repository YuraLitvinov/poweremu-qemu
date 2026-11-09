//! PPCGPU_RECORD=dir: record the R300 draws the guest makes, for offline
//! replay and inspection without the guest.
//!
//! Recording runs only while the file `dir/ON` exists (touch it when the
//! scene of interest starts, remove it to stop), unless PPCGPU_RECORD_ALWAYS=1.
//! PPCGPU_RECORD_MAX_MB caps the size (default 4096).
//!
//! `dir/draws.bin` (little endian, host layout of the C structs):
//!
//!   file header: "R300REC2", u32 sizeof(R300DrawPacket), u32 sizeof(R300Vertex),
//!                u32 sizeof(R300FSUniforms), u32 NTEX
//!   per draw:    u32 0xD4A70001, u64 draw number (counted from the VM's start),
//!                u64 glsl blob id, u32 glsl length (the GLSL text, no NUL, is a blob),
//!                the R300DrawPacket as raw bytes (its pointers are meaningless),
//!                u32 num vertices, R300Vertex x num (num_verts + num_line_verts),
//!                NTEX x { u64 blob id, u32 blob length }   (id 0: no texels)
//!
//! `dir/blobs/<id:016x>` holds the texel bytes of a texture (the GART copy if it
//! has one, else guest VRAM at its address), once per distinct content.
//!
//! Not recorded: the contents of the colour and depth buffers before a draw
//! (a replay starts from cleared buffers), and the vertex programs' inputs
//! (only their output, the vertices the backend receives).

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;

use crate::state::*;
use crate::sys::*;

const MAGIC: &[u8; 8] = b"R300REC2";
const TAG: u32 = 0xD4A7_0001;
const MAX_BLOB: u64 = 128 << 20;

pub struct Rec {
    dir: PathBuf,
    always: bool,
    seen: HashSet<u64>,
    bytes: u64,
    max: u64,
    draws: u64,
    full: bool,
}

impl Rec {
    pub fn from_env() -> Option<Rec> {
        let dir = PathBuf::from(std::env::var("PPCGPU_RECORD").ok().filter(|p| !p.is_empty())?);
        if let Err(e) = std::fs::create_dir_all(dir.join("blobs")) {
            eprintln!("ppc-mac-gpu vulkan: PPCGPU_RECORD={}: {e}", dir.display());
            return None;
        }
        let always = std::env::var("PPCGPU_RECORD_ALWAYS").map_or(false, |v| v == "1");
        let max = std::env::var("PPCGPU_RECORD_MAX_MB").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(4096) << 20;
        let mut head = Vec::new();
        head.extend_from_slice(MAGIC);
        for v in [
            std::mem::size_of::<R300DrawPacket>(),
            std::mem::size_of::<R300Vertex>(),
            std::mem::size_of::<R300FSUniforms>(),
            NTEX,
        ] {
            head.extend_from_slice(&(v as u32).to_le_bytes());
        }
        if let Err(e) = std::fs::write(dir.join("draws.bin"), &head) {
            eprintln!("ppc-mac-gpu vulkan: PPCGPU_RECORD: draws.bin: {e}");
            return None;
        }
        eprintln!(
            "ppc-mac-gpu vulkan: PPCGPU_RECORD={}: recording {} (up to {} MB)",
            dir.display(),
            if always { "from the start".to_string() } else { format!("while {}/ON exists", dir.display()) },
            max >> 20
        );
        Some(Rec { dir, always, seen: HashSet::new(), bytes: head.len() as u64, max, draws: 0, full: false })
    }
}

fn as_bytes<T>(v: &T) -> &[u8] {
    // SAFETY: plain C data (bindgen structs, no drop glue).
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

impl Vk {
    pub fn record_draw(&mut self, n: u64, pkt: &R300DrawPacket) {
        let vram = self.vram;
        let vram_size = self.vram_size;
        let Some(r) = self.rec.as_mut() else {
            return;
        };
        if r.full || !(r.always || r.dir.join("ON").exists()) {
            return;
        }
        if r.bytes >= r.max {
            r.full = true;
            eprintln!("ppc-mac-gpu vulkan: PPCGPU_RECORD: size limit reached after {} draws, stopped", r.draws);
            return;
        }

        let nv = (pkt.num_verts + pkt.num_line_verts) as usize;
        let glsl = unsafe { std::ffi::CStr::from_ptr(pkt.glsl) }.to_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(glsl.len() + nv * std::mem::size_of::<R300Vertex>() + 4096);
        out.extend_from_slice(&TAG.to_le_bytes());
        out.extend_from_slice(&n.to_le_bytes());
        let mut h = DefaultHasher::new();
        glsl.hash(&mut h);
        let gid = h.finish() | 1;
        if r.seen.insert(gid) {
            if std::fs::write(r.dir.join("blobs").join(format!("{gid:016x}")), glsl).is_ok() {
                r.bytes += glsl.len() as u64;
            } else {
                r.seen.remove(&gid);
            }
        }
        out.extend_from_slice(&gid.to_le_bytes());
        out.extend_from_slice(&(glsl.len() as u32).to_le_bytes());
        out.extend_from_slice(as_bytes(pkt));
        out.extend_from_slice(&(nv as u32).to_le_bytes());
        if nv != 0 {
            // SAFETY: the packet owns num_verts + num_line_verts vertices.
            out.extend_from_slice(unsafe {
                std::slice::from_raw_parts(pkt.verts as *const u8, nv * std::mem::size_of::<R300Vertex>())
            });
        }

        let mut blobs = 0u64;
        for t in 0..NTEX {
            let td = &pkt.tex[t];
            let mut id = 0u64;
            let mut len = 0u32;
            let size = td.size_bytes as u64;
            if td.bound && size != 0 && size <= MAX_BLOB {
                let data: Option<&[u8]> = if !td.host_data.is_null() {
                    // SAFETY: the packet owns size_bytes bytes at host_data.
                    Some(unsafe { std::slice::from_raw_parts(td.host_data, size as usize) })
                } else if !vram.is_null() && td.gpu_addr as u64 + size <= vram_size {
                    // SAFETY: in range of the guest VRAM mapping.
                    Some(unsafe { std::slice::from_raw_parts(vram.add(td.gpu_addr as usize), size as usize) })
                } else {
                    None
                };
                if let Some(d) = data {
                    let mut h = DefaultHasher::new();
                    d.hash(&mut h);
                    id = h.finish() | 1; // never 0
                    len = d.len() as u32;
                    if r.seen.insert(id) {
                        match std::fs::write(r.dir.join("blobs").join(format!("{id:016x}")), d) {
                            Ok(()) => {
                                r.bytes += d.len() as u64;
                                blobs += 1;
                            }
                            Err(e) => {
                                eprintln!("ppc-mac-gpu vulkan: PPCGPU_RECORD: blob: {e}");
                                r.seen.remove(&id);
                                id = 0;
                                len = 0;
                            }
                        }
                    }
                }
            }
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
        }

        match std::fs::OpenOptions::new().append(true).open(r.dir.join("draws.bin")).and_then(|mut f| f.write_all(&out)) {
            Ok(()) => {
                r.bytes += out.len() as u64;
                r.draws += 1;
                if r.draws == 1 || r.draws % 1000 == 0 {
                    eprintln!(
                        "ppc-mac-gpu vulkan: PPCGPU_RECORD: {} draws, {} MB ({} new textures this draw)",
                        r.draws,
                        r.bytes >> 20,
                        blobs
                    );
                }
            }
            Err(e) => {
                eprintln!("ppc-mac-gpu vulkan: PPCGPU_RECORD: draws.bin: {e}");
                r.full = true;
            }
        }
    }
}
