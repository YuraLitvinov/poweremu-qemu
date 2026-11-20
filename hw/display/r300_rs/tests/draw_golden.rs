//! The draw assembly against the C original: 150 random GPU states, each drawn
//! through `draw_build` or `draw_build_indexed` (see `tools/gen_draw.c`).

use std::mem::size_of;
use std::slice;

use std::ffi::{c_void, CStr};
use std::ptr;

use r300::capi::*;
use r300::draw::misc::{load_vbpntr, Arrays};
use r300::draw::{draw_build, draw_build_indexed, DrawPacket, Indices};
use r300::state::R300State;
use r300::types::R300TexDesc;

const GOLDEN: &str = include_str!("golden/draw.txt");

fn mem_dword(i: u32) -> u32 {
    (((i.wrapping_mul(2_654_435_761) >> 16) & 0xFF) as f32 / 64.0 - 2.0).to_bits()
}

fn read(addr: u32, dst: &mut [u8]) -> bool {
    if u64::from(addr) + dst.len() as u64 > 0x1000_0000 {
        return false;
    }
    for (k, b) in dst.iter_mut().enumerate() {
        let a = addr.wrapping_add(k as u32);
        *b = (mem_dword(a >> 2) >> (8 * (a & 3))) as u8;
    }
    true
}

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

const BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// The words of a plain struct made of 4-byte fields.
fn words<T>(x: &[T]) -> &[u32] {
    assert_eq!(size_of::<T>() % 4, 0);
    // SAFETY: only used on R300FSUniforms and R300Vertex, which are `repr(C)`
    // with 4-byte fields and no padding.
    unsafe { slice::from_raw_parts(x.as_ptr().cast(), std::mem::size_of_val(x) / 4) }
}

/// FNV over float words with -0.0 hashed as +0.0 (the sign of a zero out of
/// a `fmax`/`fmin` tie is unspecified in C).
fn fnv_f(w: &[u32]) -> u64 {
    w.iter().fold(BASIS, |h, &v| fnv(h, &(if v == 0x8000_0000 { 0 } else { v }).to_le_bytes()))
}

fn tex_hash(tex: &[R300TexDesc]) -> u64 {
    let mut t = BASIS;
    for d in tex {
        let w = [d.bound as u32, d.gpu_addr, d.width, d.height, d.endian, d.le as u32, d.tile, d.pitch_bytes];
        let m = [d.format, d.kind, d.view_bpp, d.filter0, d.filter1, d.dim, d.depth, d.levels, d.size_bytes];
        for v in w.iter().chain(&m).chain(&d.lvl_off).chain(&d.lvl_pitch).chain(&d.lvl_rows) {
            t = fnv(t, &v.to_le_bytes());
        }
    }
    t
}

fn digest(idx: &str, r: &Result<DrawPacket, &'static str>) -> Vec<String> {
    let p = match r {
        Err(e) => return vec![format!("P {idx} err {e}")],
        Ok(p) => p,
    };
    let mut v = vec![format!(
        "P {idx} ok rt {:x} {} {} {} {} {} {} {} 0 0 | sc {} {} {} {} | ncb {} dep {} {:x} {} {} {} cull {} aa {} warn {:x} cls {} nv {} nl {} gpu {} gw {} dmm {:08x} {:08x} {} {}",
        p.rt_gpu_addr, p.rt_pitch, p.rt_width, p.rt_height, p.rt_format, p.rt_bpp, p.rt_dummy as u32, p.rt_view as u32,
        p.scissor[0], p.scissor[1], p.scissor[2], p.scissor[3], p.num_cb, p.depth.attach as u32, p.depth.gpu_addr,
        p.depth.pitch, p.depth.bpp, p.depth.format, p.cull, p.aa_samples, p.warn, p.prim_class, p.num_verts,
        p.num_line_verts, p.gpu_vs as u32, p.gpu_data.len(), p.depth_min.to_bits(), p.depth_max.to_bits(),
        p.depth_clip as u32, p.depth_dx as u32
    )];
    for k in 1..4 {
        let c = &p.cb[k];
        v.push(format!("C {k} {:x} {} {} {} {}", c.gpu_addr, c.pitch, c.format, c.bpp, c.view));
    }
    let aa: Vec<u8> = p.aa_pos.iter().flatten().flat_map(|f| f.to_bits().to_le_bytes()).collect();
    v.push(format!("H aa {:016x}", fnv(BASIS, &aa)));
    v.push(format!("H uni {:016x}", fnv(BASIS, &words(slice::from_ref(&p.uniforms)).iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>())));
    v.push(format!("H tex {:016x}", tex_hash(&p.tex)));
    let glsl = if p.glsl.is_empty() { 0 } else { fnv(BASIS, p.glsl.as_bytes()) };
    v.push(format!("H glsl {} {glsl:016x}", p.glsl.len()));
    v.push(format!("H verts {} {:016x}", p.verts.len(), fnv_f(words(&p.verts))));
    let gpu = if p.gpu_data.is_empty() {
        0
    } else {
        fnv(BASIS, &p.gpu_data.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>())
    };
    v.push(format!("H gpu {gpu:016x}"));
    v
}

fn hex(f: &[&str]) -> Vec<u32> {
    f.iter().map(|x| u32::from_str_radix(x, 16).unwrap()).collect()
}

#[test]
fn draws_match_c() {
    let mut lines = GOLDEN.lines().peekable();
    let (mut ok, mut failed, mut gpu) = (0, 0, 0);
    while let Some(line) = lines.next() {
        let f: Vec<&str> = line.split(' ').collect();
        assert_eq!(f[0], "S", "scenario start: {line}");
        let idx = f[1];
        let mut st = R300State {
            win_immd: f[2] == "1",
            le_screen_h: f[3].parse().unwrap(),
            glsl_flags: f[4].parse().unwrap(),
            ..Default::default()
        };
        let (mut arr, mut b): (Arrays, Vec<&str>) = (Arrays::default(), vec![]);
        let mut vbp: Vec<u32> = vec![];
        let mut writes: Vec<&str> = vec![];
        let mut want: Vec<String> = vec![];
        while let Some(&next) = lines.peek() {
            if next.starts_with("S ") {
                break;
            }
            lines.next();
            let g: Vec<&str> = next.split(' ').collect();
            match g[0] {
                "V" => {
                    vbp = hex(&g[3..3 + g[2].parse::<usize>().unwrap()]);
                    arr = load_vbpntr(&vbp);
                }
                "B" => b = g,
                "W" => writes = g,
                _ => want.push(next.to_string()),
            }
        }
        let n: usize = writes[2].parse().unwrap();
        for i in 0..n {
            st.write(u32::from_str_radix(writes[3 + 2 * i], 16).unwrap(), u32::from_str_radix(writes[4 + 2 * i], 16).unwrap());
        }
        let ndw: usize = b[4].parse().unwrap();
        let d = hex(&b[5..5 + ndw]);
        let mut rd = read;
        let got = if b[2] == "0" {
            draw_build(&st, &arr, b[3].parse().unwrap(), &d, &mut rd)
        } else {
            draw_build_indexed(&st, &arr, b[3].parse().unwrap(), &Indices { dw: &d }, &mut rd)
        };
        let got_lines = digest(idx, &got);
        for (g, w) in got_lines.iter().zip(&want) {
            assert_eq!(g, w, "scenario {idx}");
        }
        assert_eq!(got_lines.len(), want.len(), "scenario {idx}");
        check_c_abi(&st, &vbp, &arr, &b, &d, &got);
        match got {
            Ok(p) => {
                ok += 1;
                gpu += usize::from(p.gpu_vs);
            }
            Err(_) => failed += 1,
        }
    }
    println!("{ok} packets, {failed} errors, {gpu} on the GPU vertex path");
    assert_eq!((ok, failed), (107, 43));
}

unsafe extern "C" fn c_read(_: *mut c_void, addr: u32, dst: *mut c_void, len: u32) -> bool {
    // SAFETY: the caller passes a writable buffer of `len` bytes.
    read(addr, unsafe { std::slice::from_raw_parts_mut(dst.cast(), len as usize) })
}

/// The C entry points give the same packet (and error) as the Rust API, and
/// `r300_draw_free` releases what they allocated.
fn check_c_abi(st: &R300State, vbp: &[u32], arr: &Arrays, b: &[&str], d: &[u32], want: &Result<DrawPacket, &'static str>) {
    // SAFETY: plain calls of the exported functions with valid pointers.
    unsafe {
        let mut carr = std::mem::zeroed::<CArrays>();
        r300_load_vbpntr(&mut carr, vbp.as_ptr(), vbp.len() as u32);
        assert_eq!(carr.count as usize, arr.count);
        let mut pkt = std::mem::zeroed::<CDrawPacket>();
        let mut err: *const std::ffi::c_char = ptr::null();
        let ok = if b[2] == "0" {
            r300_draw_build(st, &carr, b[3].parse().unwrap(), d.as_ptr(), d.len() as u32, c_read, ptr::null_mut(), &mut pkt, &mut err)
        } else {
            let idx = CIndices { dw: d.as_ptr(), ndw: d.len() as u32 };
            r300_draw_build_indexed(st, &carr, b[3].parse().unwrap(), &idx, c_read, ptr::null_mut(), &mut pkt, &mut err)
        };
        match want {
            Ok(w) => {
                assert!(ok);
                assert_eq!(CStr::from_ptr(pkt.glsl).to_str().unwrap(), w.glsl);
                assert_eq!((pkt.num_verts, pkt.num_line_verts, pkt.warn), (w.num_verts, w.num_line_verts, w.warn));
                assert_eq!(pkt.verts.is_null(), w.verts.is_empty());
                assert_eq!(pkt.gpu_words as usize, w.gpu_data.len());
                if !pkt.verts.is_null() {
                    let v = std::slice::from_raw_parts(pkt.verts, w.verts.len());
                    assert_eq!(v, &w.verts[..]);
                }
                r300_draw_free(&mut pkt);
                assert!(pkt.glsl.is_null() && pkt.verts.is_null() && pkt.gpu_data.is_null());
            }
            Err(e) => {
                assert!(!ok);
                assert_eq!(CStr::from_ptr(err).to_str().unwrap(), *e);
            }
        }
    }
}
