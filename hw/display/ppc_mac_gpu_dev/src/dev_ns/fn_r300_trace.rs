//! Debug tracing and dumps for R300 draws (all gated by environment variables or `r3_dump`).
use super::*;

/// Number of leading texture bytes `PPCGPU_TEX_LOG` inspects for non-zero data.
const TRACE_WINDOW_BYTES: uint64_t = 1 << 20;
/// Largest texture `PPCGPU_TEX_DUMP_FROM` dumps; without it only small RGBA textures are dumped.
const DUMP_MAX_BYTES_FROM: uint64_t = 4 << 20;
const DUMP_MAX_BYTES_DEFAULT: uint64_t = 32768;
const DUMP_MAX_FILES: uint32_t = 6000;
const DUMPED_TABLE_SIZE: usize = 512;

/// Textures already dumped as (VRAM address, content hash), so each distinct one is written once.
static DUMPED_TEXTURES: std::sync::Mutex<Vec<(uint32_t, uint64_t)>> = std::sync::Mutex::new(Vec::new());

/// `len` bytes of VRAM at `addr`; bytes outside VRAM read as zero.
fn vram_bytes(vram: crate::src::qemu::Vram, addr: uint64_t, len: uint64_t) -> Vec<u8> {
    let mut buf = vec![0u8; len as usize];
    if !vram.read_bytes(addr, &mut buf) {
        for (i, b) in buf.iter_mut().enumerate() {
            *b = vram.get8(addr + i as u64).unwrap_or(0);
        }
    }
    buf
}

/// FNV-1a, 64 bit.
fn fnv1a(data: &[u8]) -> uint64_t {
    data.iter()
        .fold(0xcbf29ce484222325, |h, &b| (h ^ b as uint64_t).wrapping_mul(0x100000001b3))
}

/// `PPCGPU_TEX_DUMP_FROM`: the first draw number to dump from (C `strtoull` base 0 syntax;
/// unset or unparsable means 0).
fn tex_dump_from() -> uint64_t {
    static FROM: std::sync::OnceLock<uint64_t> = std::sync::OnceLock::new();
    *FROM.get_or_init(|| {
        crate::src::qemu::env_string(c"PPCGPU_TEX_DUMP_FROM")
            .map_or(0, |v| crate::src::qemu::parse_c_uint(&v))
    })
}

/// Records (addr, hash) as dumped; returns false if that exact texture was dumped before.
fn note_texture_dump(addr: uint32_t, hash: uint64_t) -> bool {
    let mut seen = DUMPED_TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
    if seen.contains(&(addr, hash)) {
        return false;
    }
    if seen.len() < DUMPED_TABLE_SIZE {
        seen.push((addr, hash));
    }
    true
}

/// `PPCGPU_TEX_LOG` / `PPCGPU_TEX_DUMP`: logs a texture that lives in VRAM and optionally
/// dumps it to a file in the `PPCGPU_TEX_DUMP` directory.
pub fn r300_trace_vram_texture(
    s: &PPCMacGPUFields,
    td: &R300TexDesc,
    ga: uint32_t,
    tlen: uint64_t,
    vram: crate::src::qemu::Vram,
) {
    static NDUMP: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
    let addr = td.gpu_addr as uint64_t;
    let window = tlen.min(TRACE_WINDOW_BYTES);
    let head = vram_bytes(vram, addr, window);
    let nonzero = head.iter().filter(|&&b| b != 0).count();
    let dump_dir = crate::src::qemu::env_string(c"PPCGPU_TEX_DUMP");
    let dump_from = tex_dump_from();

    let mut is_new = true;
    if dump_dir.is_some() && dump_from != 0 && tlen <= DUMP_MAX_BYTES_FROM {
        is_new = note_texture_dump(td.gpu_addr, fnv1a(&vram_bytes(vram, addr, tlen)));
    }
    let max_bytes = if dump_from != 0 { DUMP_MAX_BYTES_FROM } else { DUMP_MAX_BYTES_DEFAULT };
    if let Some(dir) = dump_dir {
        if nonzero != 0
            && is_new
            && s.r3_ref().draws >= dump_from
            && (dump_from != 0 || td.width.wrapping_mul(4) == td.pitch_bytes)
            && NDUMP.get() < DUMP_MAX_FILES
            && tlen <= max_bytes
        {
            let n = NDUMP.get();
            NDUMP.set(n + 1);
            let path = format!(
                "{}/tex_{:02x}_{:x}_{}x{}_{:03}_d{}.raw",
                dir, td.format, td.gpu_addr, td.width, td.height, n, s.r3_ref().draws,
            );
            let data = vram_bytes(vram, addr, tlen);
            if let Err(e) = std::fs::write(&path, data) {
                crate::src::logs::err(format_args!("texture dump: cannot write {}: {}\n", path, e));
            }
        }
    }
    let first = |i: usize| byte_at(&head, i) as u32;
    crate::src::logs::err(format_args!(
        "VRAMTEX {:08x}(vram+{:x}) fmt {:02x} {}x{} pitch {} size {} nonzero {}/{} first {:02x}{:02x}{:02x}{:02x}\n",
        ga, td.gpu_addr, td.format, td.width, td.height, td.pitch_bytes, tlen, nonzero, window,
        first(0), first(1), first(2), first(3),
    ));
}

/// `PPCGPU_GART_LOG`: logs the first bytes of a texture read through the GART.
pub fn r300_trace_gart_texture(td: &R300TexDesc, ga: uint32_t, tlen: uint64_t) {
    let bytes: &[uint8_t] = crate::src::qemu::host_bytes(td.host_data, tlen as usize).unwrap_or(&[]);
    let hex = |base: usize, n: usize| -> String {
        (0..n).map(|i| format!("{:02x}", byte_at(bytes, base + i))).collect()
    };
    let head = (0..4).map(|group| hex(group * 4, 4)).collect::<Vec<_>>().join(" ");
    let offset = if tlen > 64 { 64 } else { 0 };
    let sample = if tlen > 68 { 64 } else { 0 };
    crate::src::logs::err(format_args!(
        "GARTTEX {:08x} fmt {:02x} {}x{} size {} bytes: {} | at +{}: {}\n",
        ga, td.format, td.width, td.height, tlen, head, offset, hex(sample, 4),
    ));
}

/// `PPCGPU_GART_LOG`: logs a colour buffer found in GART memory and the rows it will touch.
pub fn r300_trace_gart_render_target(
    pkt: &R300DrawPacket,
    guest_addr: uint32_t,
    (min_y, max_y): (f32, f32),
    first_row: uint32_t,
    end_row: uint32_t,
) {
    crate::src::logs::err(format_args!(
        "GARTRT {:08x} pitch {} bpp {} rt_h {} scissor {},{}-{},{} bbox y {:.2}..{:.2} -> rows {}..{} ({} verts)\n",
        guest_addr, pkt.rt_pitch, pkt.rt_bpp, pkt.rt_height,
        pkt.scissor[0], pkt.scissor[1], pkt.scissor[2], pkt.scissor[3],
        min_y, max_y, first_row, end_row, pkt.num_verts,
    ));
}

/// Logs the state a draw is about to use (`r3_dump` tracing and the sequence log): which
/// texture formats and render targets are seen, once each.
pub fn r300_trace_draw_inputs(
    s: &PPCMacGPUFields,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
) {
    if !s.r3_dump.is_null() {
        static SEEN_TEX: crate::src::qemu::SeenSet = crate::src::qemu::SeenSet::new();
        let en: uint32_t = r300_reg(s.r3_ref(), 0x4104 as uint32_t);
        let mut t: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while t < 16 as ::core::ffi::c_int {
            if !(en as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << t == 0) {
                let f0: uint32_t = r300_reg(
                    s.r3_ref(),
                    (0x4480 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                );
                let f1: uint32_t = r300_reg(
                    s.r3_ref(),
                    (0x44c0 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                );
                let f2: uint32_t = r300_reg(
                    s.r3_ref(),
                    (0x4500 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                );
                let off: uint32_t = r300_reg(
                    s.r3_ref(),
                    (0x4540 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                );
                let fl0: uint32_t = r300_reg(
                    s.r3_ref(),
                    (0x4400 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                );
                let key = format!(
                    "{:08x} {:08x} {:08x} {:x} {:08x}",
                    f0,
                    f1,
                    f2,
                    off & 0x1f as uint32_t,
                    fl0
                );
                if SEEN_TEX.insert(key) {
                    crate::src::logs::err(format_args!("TEX unit{} fmt {:02x} {}x{} pitch {}{} off {:08x} flags {:x} f0 {:08x} f1 {:08x} f2 {:08x} filter {:08x} rt {:08x}/{}\n", ((t) as i32), ((f1 & 0x1f as uint32_t) as u32), (((f0 & 0x7ff as uint32_t).wrapping_add(1 as uint32_t)) as u32), (((f0 >> 11 as ::core::ffi::c_int & 0x7ff as uint32_t)
                            .wrapping_add(1 as uint32_t)) as u32), (((f2 & 0x3fff as uint32_t).wrapping_add(1 as uint32_t)) as u32), crate::src::qemu::cstr(if f0 >> 31 as ::core::ffi::c_int != 0 {
                            c"".as_ptr()
                        } else {
                            c" (pow2)".as_ptr()
                        }), ((off as ::core::ffi::c_uint & !(0x1f as ::core::ffi::c_uint)) as u32), ((off & 0x1f as uint32_t) as u32), ((f0) as u32), ((f1) as u32), ((f2) as u32), ((fl0) as u32), ((r300_reg(s.r3_ref(), 0x4e28 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4e38 as uint32_t) & 0x3ffe as uint32_t) as u32)));
                }
            }
            t += 1;
        }
    }
    let f0_0: uint32_t = r300_reg(s.r3_ref(), 0x4480 as uint32_t);
    let f2_0: uint32_t = r300_reg(s.r3_ref(), 0x4500 as uint32_t);
    if r300_reg(s.r3_ref(), 0x4e28 as uint32_t) as ::core::ffi::c_uint & !(0x1f as ::core::ffi::c_uint)
        == 0 as ::core::ffi::c_uint
    {
        crate::src::logs::seq(format_args!("R3DCLIP rule={:04x} r0={:08x}/{:08x} r1={:08x}/{:08x} r2={:08x}/{:08x} r3={:08x}/{:08x} sc={:08x}/{:08x} zb={:08x} cb={:08x} mask={:08x}", ((r300_reg(s.r3_ref(), 0x43d0 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43b0 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43b4 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43b8 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43bc as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43c0 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43c4 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43c8 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43cc as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43e0 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x43e4 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4f00 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4e04 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4e0c as uint32_t)) as u32)));
    }
    crate::src::logs::seq(format_args!("R3D  rt={:06x}/{} tex0={}{:06x} {}x{}/{} vf={:08x} v0={:08x} {:08x}", ((r300_reg(s.r3_ref(), 0x4e28 as uint32_t) as ::core::ffi::c_uint
            & !(0x1f as ::core::ffi::c_uint)) as u32), (((r300_reg(s.r3_ref(), 0x4e38 as uint32_t) & 0x3ffe as uint32_t).wrapping_mul(4 as uint32_t)) as u32), crate::src::qemu::cstr(if r300_reg(s.r3_ref(), 0x4104 as uint32_t) & 1 as uint32_t != 0 {
            c"".as_ptr()
        } else {
            c"(off)".as_ptr()
        }), ((r300_reg(s.r3_ref(), 0x4540 as uint32_t) as ::core::ffi::c_uint
            & !(0x1f as ::core::ffi::c_uint)) as u32), (((f0_0 & 0x7ff as uint32_t).wrapping_add(1 as uint32_t)) as u32), (((f0_0 >> 11 as ::core::ffi::c_int & 0x7ff as uint32_t).wrapping_add(1 as uint32_t)) as u32), (((f2_0 & 0x3fff as uint32_t)
            .wrapping_add(1 as uint32_t)
            .wrapping_mul(4 as uint32_t)) as u32), ((if body_dw != 0 {
            dw(d, (0 as ::core::ffi::c_int) as usize)
        } else {
            0 as uint32_t
        }) as u32), ((if body_dw > 1 as uint32_t {
            dw(d, (1 as ::core::ffi::c_int) as usize)
        } else {
            0 as uint32_t
        }) as u32), ((if body_dw > 2 as uint32_t {
            dw(d, (2 as ::core::ffi::c_int) as usize)
        } else {
            0 as uint32_t
        }) as u32)));
}

/// `r3_dump` tracing: logs each distinct render target once.
pub fn r300_trace_render_target(s: &PPCMacGPUFields) {
    if !s.r3_dump.is_null() {
        static SEEN_RT: crate::src::qemu::SeenSet = crate::src::qemu::SeenSet::new();
        let key = format!(
            "{:08x} {:08x}",
            r300_reg(s.r3_ref(), 0x4e28 as uint32_t),
            r300_reg(s.r3_ref(), 0x4e38 as uint32_t)
        );
        if SEEN_RT.insert(key.clone()) {
            crate::src::logs::err(format_args!("RT {}\n", key));
        }
    }
}

/// Draw-log and `r3_dump` tracing of a freshly built packet.
pub fn r300_trace_built_packet(
    s: &mut PPCMacGPUFields,
    pkt: &R300DrawPacket,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
    build_ok: bool,
    err: *const ::core::ffi::c_char,
) {
    if r300_drawlog() {
        crate::src::logs::err(format_args!("  built ok={} err={} verts={} line_verts={} scissor={},{}-{},{}\n", ((build_ok as ::core::ffi::c_int) as i32), crate::src::qemu::cstr(if build_ok as ::core::ffi::c_int != 0 {
                c"-".as_ptr()
            } else {
                err
            }), ((if build_ok as ::core::ffi::c_int != 0 {
                pkt.num_verts
            } else {
                0 as uint32_t
            }) as u32), ((if build_ok as ::core::ffi::c_int != 0 {
                pkt.num_line_verts
            } else {
                0 as uint32_t
            }) as u32), ((pkt.scissor[0 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[1 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[2 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[3 as ::core::ffi::c_int as usize]) as u32)));
        let c0: uint32_t = r300_reg(s.r3_ref(), 0x22d0 as uint32_t);
        crate::src::logs::err(format_args!("  pvs code_cntl0={:08x} cntl1={:08x} const={:08x} flow={:08x}:", ((c0) as u32), ((r300_reg(s.r3_ref(), 0x22d8 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x22d4 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x22dc as uint32_t)) as u32)));
        let mut pc: uint32_t = c0 & 0x3ff as uint32_t;
        while pc <= c0 >> 20 as ::core::ffi::c_int & 0x3ff as uint32_t && pc < 16 as uint32_t {
            let w: &[uint32_t] = &s.r3_ref().pvs_mem[pc.wrapping_mul(4 as uint32_t) as usize..];
            crate::src::logs::err(format_args!(" [{}]{:08x}.{:08x}.{:08x}.{:08x}", ((pc) as u32), ((w[0]) as u32), ((w[1]) as u32), ((w[2]) as u32), ((w[3]) as u32)));
            pc = pc.wrapping_add(1);
        }
        crate::src::logs::err(format_args!("\n"));
        let mut v: uint32_t = 0 as uint32_t;
        while build_ok as ::core::ffi::c_int != 0
            && !pkt.verts.is_null()
            && v < pkt.num_verts
            && v < 12 as uint32_t
        {
            let x: &R300Vertex = &pkt.verts()[v as usize];
            crate::src::logs::err(format_args!("   v{} pos {:.3} {:.3} {:.3} {:.3}  t0 {:.4} {:.4} {:.4} {:.4}  t1 {:.4} {:.4} {:.4} {:.4}\n", ((v) as u32), ((x.pos[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x.pos[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x.pos[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x.pos[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[0 as ::core::ffi::c_int
                    as usize][0 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[0 as ::core::ffi::c_int
                    as usize][1 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[0 as ::core::ffi::c_int
                    as usize][2 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[0 as ::core::ffi::c_int
                    as usize][3 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[1 as ::core::ffi::c_int
                    as usize][0 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[1 as ::core::ffi::c_int
                    as usize][1 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[1 as ::core::ffi::c_int
                    as usize][2 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x
                    .v[1 as ::core::ffi::c_int
                    as usize][3 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64)));
            v = v.wrapping_add(1);
        }
        r300_drawlog_flush();
    }
    if !s.r3_dump.is_null()
        && g_r300_arm_rt.get() != 0
        && r300_reg(s.r3_ref(), 0x4e28 as uint32_t) as ::core::ffi::c_uint
            & !(0x1f as ::core::ffi::c_uint)
            == g_r300_arm_rt.get()
    {
        crate::src::logs::err(format_args!("TRY op {:02x} vf {:08x} ok {} err {} tex0 en {} off {:06x} fmt {:08x}/{:08x}/{:08x} scissor {},{}-{},{} verts {}\n", ((opcode) as u32), ((if body_dw != 0 {
                dw(d, (0 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32), ((build_ok as ::core::ffi::c_int) as i32), crate::src::qemu::cstr(if build_ok as ::core::ffi::c_int != 0 {
                c"-".as_ptr()
            } else {
                err
            }), ((r300_reg(s.r3_ref(), 0x4104 as uint32_t) & 1 as uint32_t) as u32), ((r300_reg(s.r3_ref(), 0x4540 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4480 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x44c0 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4500 as uint32_t)) as u32), ((pkt.scissor[0 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[1 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[2 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[3 as ::core::ffi::c_int as usize]) as u32), ((if build_ok as ::core::ffi::c_int != 0 {
                pkt.num_verts
            } else {
                0 as uint32_t
            }) as u32)));
        let mut v_0: uint32_t = 0 as uint32_t;
        while build_ok as ::core::ffi::c_int != 0
            && !pkt.verts.is_null()
            && v_0 < pkt.num_verts
            && v_0 < 48 as uint32_t
        {
            let x_0: &R300Vertex = &pkt.verts()[v_0 as usize];
            crate::src::logs::err(format_args!("   v{} ndc {:.3} {:.3} w {:.3}  t {:.4} {:.4} {:.4} {:.4} | {:.4} {:.4} {:.4} {:.4}\n", ((v_0) as u32), (((x_0.pos[0 as ::core::ffi::c_int as usize]
                    / x_0.pos[3 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_double) as f64), (((x_0.pos[1 as ::core::ffi::c_int as usize]
                    / x_0.pos[3 as ::core::ffi::c_int as usize])
                    as ::core::ffi::c_double) as f64), ((x_0.pos[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_double) as f64), ((x_0.v[0 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[0 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[0 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[0 as ::core::ffi::c_int as usize][3 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[1 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[1 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[1 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64), ((x_0.v[1 as ::core::ffi::c_int as usize][3 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double) as f64)));
            v_0 = v_0.wrapping_add(1);
        }
    }
}

/// Logs a failed draw build (once per message) and, with `r3_dump` on, the first few details.
pub fn r300_report_build_failure(
    s: &PPCMacGPUFields,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
    err: *const ::core::ffi::c_char,
) {

        static fail_dumps: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
        r300_warn_once(&crate::src::qemu::cstr(err), None);
        if !s.r3_dump.is_null() && {
            let fresh19 = fail_dumps.get();
            fail_dumps.set(fail_dumps.get() + 1);
            fresh19 < 3 as ::core::ffi::c_int
        } {
            crate::src::logs::err(format_args!("--- draw failed ({}): op {:02x} vf {:08x} vtx_size {} psc {:08x} {:08x} {:08x} {:08x} ext {:08x} {:08x}, {} arrays:", crate::src::qemu::cstr(err), ((opcode) as u32), ((if body_dw != 0 {
                    dw(d, (0 as ::core::ffi::c_int) as usize)
                } else {
                    0 as uint32_t
                }) as u32), ((r300_reg(s.r3_ref(), 0x20b4 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x2150 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x2154 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x2158 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x215c as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x21e0 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x21e4 as uint32_t)) as u32), ((s.r3_arrays.count) as u32)));
            let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
            while i < s.r3_arrays.count {
                crate::src::logs::err(format_args!(" [{:08x} size {} stride {}]", ((s.r3_arrays.a[i as usize].addr) as u32), ((s.r3_arrays.a[i as usize].size_dw) as u32), ((s.r3_arrays.a[i as usize].stride_dw) as u32)));
                i = i.wrapping_add(1);
            }
            crate::src::logs::err(format_args!("\n"));
        }
}

const MAX_GLSL_DUMPS: uint32_t = 80;
const MAX_DXT_DUMPS: uint32_t = 60;

/// `R300_GLSLDUMP` / `R300_DXTDUMP`: writes the draw's generated fragment shader and its
/// compressed textures to files, each distinct one once, up to a limit.
pub fn r300_dump_draw_assets(
    s: &PPCMacGPUFields,
    pkt: &R300DrawPacket,
    vram: crate::src::qemu::Vram,
) {
    r300_dump_glsl(s, pkt);
    r300_dump_dxt_textures(pkt, vram);
}

/// Writes `data` to `path`, reporting (not failing on) an error; `what` names the dump in it.
fn write_dump(what: &str, path: &str, data: impl AsRef<[u8]>) {
    if let Err(e) = std::fs::write(path, data) {
        crate::src::logs::err(format_args!("{}: cannot write {}: {}\n", what, path, e));
    }
}

/// `R300_GLSLDUMP=<prefix>`: writes each distinct fragment shader to `<prefix>.NN.frag`.
fn r300_dump_glsl(s: &PPCMacGPUFields, pkt: &R300DrawPacket) {
    static PREFIX: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    static COUNT: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
    static SEEN: crate::src::qemu::SeenSet = crate::src::qemu::SeenSet::new();
    let Some(prefix) = PREFIX.get_or_init(|| crate::src::qemu::env_string(c"R300_GLSLDUMP")) else {
        return;
    };
    if pkt.glsl.is_null() || COUNT.get() >= MAX_GLSL_DUMPS {
        return;
    }
    let glsl = crate::src::qemu::cstr(pkt.glsl);
    if !SEEN.insert(glsl.clone()) {
        return;
    }
    let n = COUNT.get();
    COUNT.set(n + 1);
    let state = s.r3_ref();
    let text = format!(
        "// draw {} blend {:08x}/{:08x}/{:08x} alpha-test {:08x}\n{}",
        state.draws,
        r300_reg(state, 0x4e04),
        r300_reg(state, 0x4e08),
        r300_reg(state, 0x4e0c),
        r300_reg(state, 0x4bd4),
        glsl
    );
    write_dump("R300_GLSLDUMP", &format!("{}.{:02}.frag", prefix, n), text);
}

/// `R300_DXTDUMP=<prefix>`: writes each distinct compressed texture to `<prefix>.NN_dxt....bin`.
fn r300_dump_dxt_textures(pkt: &R300DrawPacket, vram: crate::src::qemu::Vram) {
    static PREFIX: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    static COUNT: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
    static SEEN: crate::src::qemu::SeenSet = crate::src::qemu::SeenSet::new();
    let Some(prefix) = PREFIX.get_or_init(|| crate::src::qemu::env_string(c"R300_DXTDUMP")) else {
        return;
    };
    let dxt = pkt
        .tex
        .iter()
        .filter(|td| td.bound && td.kind >= R300_TEXK_DXT1 as uint32_t && td.size_bytes != 0);
    for td in dxt {
        if COUNT.get() >= MAX_DXT_DUMPS {
            break;
        }
        if !SEEN.insert((td.gpu_addr ^ td.width << 20 ^ td.height).to_string()) {
            continue;
        }
        let n = COUNT.get();
        COUNT.set(n + 1);
        let size = td.size_bytes as usize;
        let data = match crate::src::qemu::host_bytes(td.host_data, size) {
            Some(bytes) => Some(bytes.to_vec()),
            None => crate::src::qemu::try_alloc_bytes(size)
                .and_then(|mut buf| vram.read_bytes(td.gpu_addr as u64, &mut buf).then_some(buf)),
        };
        let path = format!(
            "{}.{:02}_dxt{}_{}x{}_l{}_p{}{}.bin",
            prefix,
            n,
            td.kind.wrapping_sub(R300_TEXK_DXT1 as uint32_t),
            td.width,
            td.height,
            td.levels,
            td.pitch_bytes,
            if td.host_data.is_null() { "" } else { "_gart" },
        );
        if let Some(data) = data {
            write_dump("R300_DXTDUMP", &path, data);
        }
    }
}

/// `R300_DUMP` prefix for the snapshot files; glibc prints "(null)" for an unset variable.
fn dump_prefix() -> String {
    crate::src::qemu::env_string(c"R300_DUMP").unwrap_or_else(|| "(null)".to_string())
}

/// Writes `len` bytes of VRAM starting at `addr` to `path`; bytes past the end of VRAM are zero.
fn dump_vram(path: &str, vram: crate::src::qemu::Vram, addr: uint64_t, len: usize) {
    let Some(mut buf) = crate::src::qemu::try_alloc_bytes(len) else {
        return;
    };
    if !vram.read_bytes(addr, &mut buf) {
        for (i, b) in buf.iter_mut().enumerate() {
            *b = vram.get8(addr.wrapping_add(i as uint64_t)).unwrap_or(0);
        }
    }
    write_dump("R300_DUMP", path, buf);
}

/// Debug aid (`r3_dump`): arms on a large texture-to-screen copy, then dumps the next 60
/// draws that target that surface (`R300_DUMP` prefix) together with their state.
pub fn r300_trace_armed_steps(
    s: &PPCMacGPUFields,
    pkt: &R300DrawPacket,
    vram: crate::src::qemu::Vram,
) {
            static arm_rt: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
            static armed_left: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
            static step: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
            if !s.r3_dump.is_null()
                && armed_left.get() == 0
                && s.r3_ref().draws > 5000 as uint64_t
                && pkt.rt_gpu_addr == 0 as uint32_t
                && pkt.tex[0 as ::core::ffi::c_int as usize].bound as ::core::ffi::c_int != 0
                && pkt.tex[0 as ::core::ffi::c_int as usize].gpu_addr != 0 as uint32_t
                && pkt.tex[0 as ::core::ffi::c_int as usize].height > 350 as uint32_t
                && pkt.tex[0 as ::core::ffi::c_int as usize].width > 500 as uint32_t
                && pkt.tex[0 as ::core::ffi::c_int as usize].pitch_bytes
                    != pkt.rt_pitch.wrapping_mul(pkt.rt_bpp)
                && step.get() == 0 as uint32_t
            {
                arm_rt.set(pkt.tex[0 as ::core::ffi::c_int as usize].gpu_addr);
                g_r300_arm_rt.set(arm_rt.get());
                armed_left.set(60 as uint32_t);
                crate::src::logs::err(format_args!("ARMED on {:06x}\n", ((arm_rt.get()) as u32)));
            } else if armed_left.get() != 0 && pkt.rt_gpu_addr == arm_rt.get() {
                crate::src::qemu::renderer_flush_sync(s);
                let path = format!(
                    "{}.step{:02}_p{}_h{}.bin",
                    dump_prefix(),
                    step.get(),
                    pkt.rt_pitch.wrapping_mul(pkt.rt_bpp),
                    pkt.rt_height,
                );
                let len = (pkt.rt_pitch as usize)
                    .wrapping_mul(pkt.rt_bpp as usize)
                    .wrapping_mul(pkt.rt_height as usize);
                dump_vram(&path, vram, pkt.rt_gpu_addr as uint64_t, len);
                crate::src::logs::err(format_args!("STEP {:02} tex {:06x} {}x{}/{} kind {} verts {} cls {} cblend {:08x} ablend {:08x} rule {:04x} clip0 {},{}-{},{} scissor {},{}-{},{}\n", ((step.get()) as u32), ((pkt.tex[0 as ::core::ffi::c_int as usize].gpu_addr) as u32), ((pkt.tex[0 as ::core::ffi::c_int as usize].width) as u32), ((pkt.tex[0 as ::core::ffi::c_int as usize].height) as u32), ((pkt.tex[0 as ::core::ffi::c_int as usize].pitch_bytes) as u32), ((pkt.tex[0 as ::core::ffi::c_int as usize].kind) as u32), ((pkt.num_verts) as u32), ((pkt.prim_class) as u32), ((pkt.uniforms.cblend) as u32), ((pkt.uniforms.ablend) as u32), ((pkt.uniforms.clip_rule) as u32), ((pkt
                        .uniforms
                        .cliprect[0 as ::core::ffi::c_int
                        as usize][0 as ::core::ffi::c_int as usize]) as i32), ((pkt
                        .uniforms
                        .cliprect[0 as ::core::ffi::c_int
                        as usize][1 as ::core::ffi::c_int as usize]) as i32), ((pkt
                        .uniforms
                        .cliprect[0 as ::core::ffi::c_int
                        as usize][2 as ::core::ffi::c_int as usize]) as i32), ((pkt
                        .uniforms
                        .cliprect[0 as ::core::ffi::c_int
                        as usize][3 as ::core::ffi::c_int as usize]) as i32), ((pkt.scissor[0 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[1 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[2 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[3 as ::core::ffi::c_int as usize]) as u32)));
                let mut v_3: uint32_t = 0 as uint32_t;
                while !pkt.verts.is_null() && v_3 < pkt.num_verts && v_3 < 6 as uint32_t {
                    let x_2: &R300Vertex = &pkt.verts()[v_3 as usize];
                    crate::src::logs::err(format_args!("   v{} pos {:.3} {:.3} {:.3} {:.3}  v0 {:.4} {:.4} {:.4} {:.4}  v1 {:.4} {:.4} {:.4} {:.4}\n", ((v_3) as u32), ((x_2.pos[0 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2.pos[1 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2.pos[2 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2.pos[3 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[0 as ::core::ffi::c_int
                            as usize][0 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[0 as ::core::ffi::c_int
                            as usize][1 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[0 as ::core::ffi::c_int
                            as usize][2 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[0 as ::core::ffi::c_int
                            as usize][3 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[1 as ::core::ffi::c_int
                            as usize][0 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[1 as ::core::ffi::c_int
                            as usize][1 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[1 as ::core::ffi::c_int
                            as usize][2 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64), ((x_2
                            .v[1 as ::core::ffi::c_int
                            as usize][3 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_double) as f64)));
                    v_3 = v_3.wrapping_add(1);
                }
                step.set(step.get().wrapping_add(1));
                armed_left.set(armed_left.get().wrapping_sub(1));
                if armed_left.get() == 0 as uint32_t {
                    g_r300_arm_rt.set(0 as uint32_t);
                }
            }
}

/// Debug aid (`r3_dump`): every 150th qualifying texture draw, snapshots the source texture
/// and the screen to `R300_DUMP` files.
pub fn r300_snapshot_textures(
    s: &PPCMacGPUFields,
    pkt: &R300DrawPacket,
    vram: crate::src::qemu::Vram,
) {
            static reads: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
            let en_0: uint32_t = r300_reg(s.r3_ref(), 0x4104 as uint32_t);
            if !s.r3_dump.is_null()
                && pkt.rt_gpu_addr == 0 as uint32_t
                && en_0 & 1 as uint32_t != 0
                && pkt.tex[0 as ::core::ffi::c_int as usize].bound as ::core::ffi::c_int != 0
                && pkt.tex[0 as ::core::ffi::c_int as usize].gpu_addr != 0 as uint32_t
                && pkt.tex[0 as ::core::ffi::c_int as usize].width < 1000 as uint32_t
                && pkt.tex[0 as ::core::ffi::c_int as usize].height > 100 as uint32_t
                && s.r3_ref().draws > 3000 as uint64_t
                && {
                    reads.set(reads.get() + 1);
                    reads.get() % 150 as ::core::ffi::c_int == 0 as ::core::ffi::c_int
                }
                && reads.get() <= 1500 as ::core::ffi::c_int
            {
                crate::src::qemu::renderer_flush_sync(s);
                let n = reads.get() / 150 as ::core::ffi::c_int;
                let tex = &pkt.tex[0 as ::core::ffi::c_int as usize];
                let tex_path = format!(
                    "{}.{:02}.tex_{:06x}_{}x{}_p{}.bin",
                    dump_prefix(),
                    n,
                    tex.gpu_addr,
                    tex.width,
                    tex.height,
                    tex.pitch_bytes,
                );
                let tex_len = (tex.pitch_bytes as usize).wrapping_mul(tex.height as usize);
                dump_vram(&tex_path, vram, tex.gpu_addr as uint64_t, tex_len);
                let screen_path = format!(
                    "{}.{:02}.screen_p{}_h{}.bin",
                    dump_prefix(),
                    n,
                    pkt.rt_pitch.wrapping_mul(pkt.rt_bpp),
                    pkt.rt_height,
                );
                let screen_len = (pkt.rt_pitch as usize)
                    .wrapping_mul(pkt.rt_bpp as usize)
                    .wrapping_mul(pkt.rt_height as usize);
                dump_vram(&screen_path, vram, 0, screen_len);
                crate::src::logs::err(format_args!("SNAP taken at draw {}\n", ((s.r3_ref().draws as ::core::ffi::c_ulonglong) as u64)));
            }
}

/// `ZPASS` tracing: logs each draw counted by the Z-pass query.
pub fn r300_trace_zpass(s: &PPCMacGPUFields, pkt: &R300DrawPacket) {
    if s.r3_zpass_active as ::core::ffi::c_int != 0
        && r300_zpass_log() as ::core::ffi::c_int != 0
    {
        let h_0: uint32_t = crate::src::qemu::c_bytes(pkt.glsl as *const ::core::ffi::c_char)
            .iter()
            .fold(2166136261 as uint32_t, |h, &b| {
                (h ^ b as uint32_t).wrapping_mul(16777619 as uint32_t)
            });
        crate::src::logs::err(format_args!("ZPASS draw counted: rt {}x{} scissor {},{}-{},{} cull {} nv {} prim {} zinfo {:x}/{:x}/{:x} cblend {:x} chanmask {:x} depth.attach {} fs {:08x}\n", ((pkt.rt_width) as u32), ((pkt.rt_height) as u32), ((pkt.scissor[0 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[1 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[2 as ::core::ffi::c_int as usize]) as u32), ((pkt.scissor[3 as ::core::ffi::c_int as usize]) as u32), ((pkt.cull) as u32), ((pkt.num_verts) as u32), ((pkt.prim_class) as u32), ((pkt.uniforms.zinfo[0 as ::core::ffi::c_int as usize]) as u32), ((pkt.uniforms.zinfo[1 as ::core::ffi::c_int as usize]) as u32), ((pkt.uniforms.zinfo[2 as ::core::ffi::c_int as usize]) as u32), ((pkt.uniforms.cblend) as u32), ((pkt.uniforms.chanmask) as u32), ((pkt.depth.attach as ::core::ffi::c_int) as i32), ((h_0) as u32)));
    }
}
