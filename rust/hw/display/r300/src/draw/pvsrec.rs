//! `PPCGPU_PVSREC=file`: append the vertex program, its constants and the
//! input and output vertices of draws of exactly `PPCGPU_PVSREC_N` vertices (up
//! to `PPCGPU_PVSREC_MAX` of them, default 60), to find out what a program does
//! to a given mesh.  Binary, little endian:
//!
//! `"PVS1", u32 n, u32 first_inst, u32 last_inst, u32 max_const, u32 fc_opc,
//! u32 fc_addrs[16], u32 fc_loop[16], u32 code[256*4], f32 consts[256*4],
//! f32 in[n][32][4], f32 out[n][32][4]`

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

use crate::pvs::{Inputs, Outputs, Program, Vec4, MAX_INSTS};

struct Config {
    path: Option<String>,
    n: u32,
    max: u32,
}

struct Rec {
    config: Config,
    done: u32,
    seen: Vec<u64>,
}

fn rec() -> &'static Mutex<Rec> {
    static REC: OnceLock<Mutex<Rec>> = OnceLock::new();
    REC.get_or_init(|| {
        let var = |k: &str| std::env::var(k).ok();
        let parse = |s: &str| {
            let s = s.trim();
            match s.strip_prefix("0x") {
                Some(h) => u32::from_str_radix(h, 16).unwrap_or(0),
                None => s.parse().unwrap_or(0),
            }
        };
        let (p, v, m) = (var("PPCGPU_PVSREC"), var("PPCGPU_PVSREC_N"), var("PPCGPU_PVSREC_MAX"));
        let path = p.filter(|p| !p.is_empty() && v.is_some());
        let n = v.as_deref().map_or(0, parse);
        let max = m.as_deref().map_or(60, parse);
        if let Some(path) = &path {
            eprintln!(
                "ppc-mac-gpu r300: PPCGPU_PVSREC={path}: recording vertex programs of {n}-vertex draws (up to {max})"
            );
        }
        Mutex::new(Rec { config: Config { path, n, max }, done: 0, seen: Vec::new() })
    })
}

/// Whether a draw of `n` vertices is to be recorded.
pub fn wanted(n: u32) -> bool {
    let r = rec().lock().unwrap_or_else(|p| p.into_inner());
    // PPCGPU_PVSREC_N=0: every distinct program once, its first 64 vertices
    r.config.path.is_some() && n != 0 && (r.config.n == 0 || n == r.config.n) && r.done < r.config.max
}

/// Has this program (code, flow control, constants) been recorded already?
/// Only used when `PVSREC_N` is 0.
fn seen(prog: &Program, consts: &[Vec4]) -> bool {
    const P: u64 = 1_099_511_628_211;
    let mut h: u64 = 1_469_598_103_934_665_603;
    let d = &prog.desc;
    for pc in d.first_inst..=d.last_inst.min(MAX_INSTS - 1) {
        for k in 0..4 {
            h = (h ^ u64::from(d.code.get(pc * 4 + k).copied().unwrap_or(0))).wrapping_mul(P);
        }
    }
    h = (h ^ u64::from(d.flow.opc)).wrapping_mul(P);
    for (i, c) in consts.iter().enumerate().take(256) {
        if i as i32 > d.max_const {
            break;
        }
        for v in c {
            h = (h ^ u64::from(v.to_bits())).wrapping_mul(P);
        }
    }
    let mut r = rec().lock().unwrap_or_else(|p| p.into_inner());
    if r.seen.contains(&h) {
        return true;
    }
    if r.seen.len() < 1024 {
        r.seen.push(h);
    }
    false
}

/// Records a draw if configured.  `n` is the vertex count of the draw.
pub fn record(prog: &Program, consts: &[Vec4], n: u32, cin: &[Inputs], outs: &[Outputs]) {
    let (path, rec_n) = {
        let r = rec().lock().unwrap_or_else(|p| p.into_inner());
        (r.config.path.clone(), r.config.n)
    };
    let Some(path) = path else {
        return;
    };
    if rec_n == 0 && seen(prog, consts) {
        return;
    }
    let count = if rec_n == 0 && n > 64 { 64 } else { n } as usize;
    let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let d = &prog.desc;
    let mut buf: Vec<u8> = b"PVS1".to_vec();
    let mut put = |v: u32| buf.extend_from_slice(&v.to_le_bytes());
    for v in [count as u32, d.first_inst as u32, d.last_inst as u32, d.max_const as u32, d.flow.opc] {
        put(v);
    }
    for v in d.flow.addrs.iter().chain(&d.flow.loops) {
        put(*v);
    }
    for i in 0..MAX_INSTS * 4 {
        put(d.code.get(i).copied().unwrap_or(0));
    }
    for i in 0..256 {
        for k in 0..4 {
            put(consts.get(i).map_or(0, |c| c[k].to_bits()));
        }
    }
    for v in cin.iter().take(count).flatten().flatten() {
        put(v.to_bits());
    }
    for v in outs.iter().take(count).flatten().flatten() {
        put(v.to_bits());
    }
    if f.write_all(&buf).is_ok() {
        rec().lock().unwrap_or_else(|p| p.into_inner()).done += 1;
    }
}
