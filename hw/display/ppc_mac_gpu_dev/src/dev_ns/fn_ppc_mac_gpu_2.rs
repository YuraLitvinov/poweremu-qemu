use super::*;
pub fn ppc_mac_gpu_hostdata_stream(
    s: &mut PPCMacGPUState,
    hdr: &[uint32_t],
    pos: uint32_t,
    size: uint32_t,
    bits: &[uint32_t],
    nbits_dw: uint32_t,
) {
    let gmc: uint32_t = dw(hdr, 0);
    let po: uint32_t = dw(hdr, 1);
    let src_dt: uint32_t = gmc >> 12 & 3;
    let off: uint32_t = gpu_po_offset(s, po);
    let pitch: uint32_t = (po & 0x3fc0_0000) >> 16;
    let bpp: uint32_t = gpu_gmc_bpp(gmc);
    let w: uint32_t = size & 0xffff;
    let h: uint32_t = size >> 16;
    let dx: uint32_t = pos & 0xffff;
    let dy: uint32_t = pos >> 16;
    let lsb: bool = gmc & (1 << 14) != 0;
    let clip: bool = gmc & (1 << 3) != 0;
    let vram = crate::src::qemu::Vram::of(s);
    if g_seq_log_enabled.get() > 0 {
        let mut words = String::new();
        for i in 0..nbits_dw.min(24) {
            words.push_str(&format!(" {:08x}", dw(bits, i as usize)));
        }
        crate::src::logs::seq(format_args!(
            "GLYPH w={} h={} dw={}{}",
            w, h, nbits_dw, words
        ));
    }
    if w == 0
        || h == 0
        || pitch == 0
        || (w as uint64_t).wrapping_mul(h as uint64_t)
            > (nbits_dw as uint64_t).wrapping_mul(32 as uint64_t)
    {
        return;
    }
    gpu_vram_access(
        s,
        (off as uint64_t).wrapping_add((dy as uint64_t).wrapping_mul(pitch as uint64_t)),
        (off as uint64_t).wrapping_add(
            (dy as uint64_t)
                .wrapping_add(h as uint64_t)
                .wrapping_mul(pitch as uint64_t),
        ),
        1 as ::core::ffi::c_int != 0,
    );
    let mut y: uint32_t = 0 as uint32_t;
    while y < h {
        let mut x: uint32_t = 0 as uint32_t;
        while x < w {
            let k: uint32_t = y.wrapping_mul(w).wrapping_add(x);
            let px: uint32_t = dx.wrapping_add(x);
            let py: uint32_t = dy.wrapping_add(y);
            let word: uint32_t = dw(bits, (k.wrapping_div(32 as uint32_t)) as usize);
            let sh: uint32_t = (k >> 3 as ::core::ffi::c_int & 3 as uint32_t)
                .wrapping_mul(8 as uint32_t)
                .wrapping_add(
                    if lsb as ::core::ffi::c_int != 0 {
                        k & 7 as uint32_t
                    } else {
                        (7 as uint32_t).wrapping_sub(k & 7 as uint32_t)
                    },
                );
            let bit: bool = word >> sh & 1 as uint32_t != 0;
            if !(clip as ::core::ffi::c_int != 0
                && (px < dw(hdr, (2 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t
                    || py
                        < dw(hdr, (2 as ::core::ffi::c_int) as usize)
                            >> 16 as ::core::ffi::c_int
                    || px >= dw(hdr, (3 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t
                    || py
                        >= dw(hdr, (3 as ::core::ffi::c_int) as usize)
                            >> 16 as ::core::ffi::c_int))
            {
                if bit as ::core::ffi::c_int != 0 || src_dt == 0 as uint32_t {
                    crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_hostdata_stream", 4841);
                    fill_vram_px(
                        s,
                        vram,
                        (off as uint64_t)
                            .wrapping_add((py as uint64_t).wrapping_mul(pitch as uint64_t))
                            .wrapping_add((px as uint64_t).wrapping_mul(bpp as uint64_t)),
                        if bit as ::core::ffi::c_int != 0 {
                            dw(hdr, (4 as ::core::ffi::c_int) as usize)
                        } else {
                            dw(hdr, (5 as ::core::ffi::c_int) as usize)
                        },
                        bpp,
                    );
                }
            }
            x = x.wrapping_add(1);
        }
        y = y.wrapping_add(1);
    }
    gpu_vram_dirty(
        s,
        (off as uint64_t).wrapping_add((dy as uint64_t).wrapping_mul(pitch as uint64_t)),
        (h as uint64_t).wrapping_mul(pitch as uint64_t),
    );
}
pub fn ppc_mac_gpu_pm4_2d(
    s: &mut PPCMacGPUState,
    opcode: uint32_t,
    d: &[uint32_t],
    n: uint32_t,
    bbm_hdr: &mut [uint32_t; 6],
    bbm_hdr_n: &mut uint32_t,
) -> bool {
    let mut op: Gpu2DOp = Gpu2DOp {
        gmc: 0,
        dst_off: 0,
        dst_pitch: 0,
        src_off: 0,
        src_pitch: 0,
        dx: 0,
        dy: 0,
        sx: 0,
        sy: 0,
        w: 0,
        h: 0,
        xl2r: false,
        yt2b: false,
        auto_dir: false,
        clip: false,
        clip_x0: 0,
        clip_y0: 0,
        clip_x1: 0,
        clip_y1: 0,
        frgd: s.regs.dp_brush_frgd_clr,
        bkgd: s.regs.dp_brush_bkgd_clr,
        brush_yx: s.regs.brush_y_x,
        brush_data: &raw mut s.regs.brush_data as *mut uint32_t,
    };
    static hd_hdr: std::sync::Mutex<[uint32_t; 9]> = std::sync::Mutex::new([0; 9]);
    static hd_valid: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
    static skip: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(2 as ::core::ffi::c_int));
    if skip.get() == -(2 as ::core::ffi::c_int) {
        // strtoul(v, NULL, 16): optional sign, optional 0x, then the hex-digit prefix.
        skip.set(match crate::src::qemu::env_string(c"PPCGPU_SKIP_OP") {
            Some(v) => {
                let t = v.trim_start();
                let (neg, t) = match t.as_bytes().first() {
                    Some(b'-') => (true, &t[1..]),
                    Some(b'+') => (false, &t[1..]),
                    _ => (false, t),
                };
                let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
                let mag = t
                    .chars()
                    .map_while(|c| c.to_digit(16))
                    .fold(0u64, |acc, dig| acc.wrapping_mul(16).wrapping_add(dig as u64));
                let mag = if neg { mag.wrapping_neg() } else { mag };
                mag as ::core::ffi::c_int
            }
            None => -(1 as ::core::ffi::c_int),
        });
    }
    if skip.get() >= 0 as ::core::ffi::c_int && opcode == skip.get() as uint32_t {
        return true;
    }
    if opcode == 0x94 as uint32_t
        && n == 9
        && dw(d, 7) & 0xffff as uint32_t == 0
    {
        if let Some(src) = d.get(..9) {
            hd_hdr.lock().unwrap_or_else(|e| e.into_inner()).copy_from_slice(src);
        }
        hd_valid.set(dw(d, 0) >> 12 as ::core::ffi::c_int
            & 3
            <= 1);
        crate::src::logs::seq(format_args!("HDHDR {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", ((dw(d, 0)) as u32), ((dw(d, 1)) as u32), ((dw(d, 2)) as u32), ((dw(d, 3)) as u32), ((dw(d, 4)) as u32), ((dw(d, 5)) as u32), ((dw(d, 6)) as u32), ((dw(d, 7)) as u32), ((dw(d, 8)) as u32)));
        return true;
    }
    if opcode == 0x19 as uint32_t && hd_valid.get() && n >= 3 {
        let hdr = *hd_hdr.lock().unwrap_or_else(|e| e.into_inner());
        ppc_mac_gpu_hostdata_stream(
            s,
            &hdr,
            dw(d, 0),
            dw(d, 1),
            d.get(2..).unwrap_or(&[]),
            n.wrapping_sub(2),
        );
        return true;
    }
    static win_pal: std::sync::Mutex<[uint32_t; 256]> = std::sync::Mutex::new([0; 256]);
    if opcode == 0x2c as uint32_t && n == 257 {
        if let Some(src) = d.get(1..257) {
            win_pal.lock().unwrap_or_else(|e| e.into_inner()).copy_from_slice(src);
        }
        return true;
    }
    if opcode == 0x94 as uint32_t
        && n > 9
        && dw(d, 0) >> 12 as ::core::ffi::c_int & 3
            == 1
        && dw(d, 6) == n.wrapping_sub(7)
        && dw(d, 5) & 0xffff as uint32_t != 0
        && dw(d, 5) >> 16 as ::core::ffi::c_int != 0
        && dw(d, 6) as uint64_t
            == ((dw(d, 5) & 0xffff as uint32_t) as uint64_t)
                .wrapping_mul(
                    (dw(d, 5) >> 16 as ::core::ffi::c_int)
                        as uint64_t,
                )
                .wrapping_add(3)
                .wrapping_div(4)
        && gpu_gmc_bpp(dw(d, 0)) == 4
    {
        let off: uint32_t = gpu_po_offset(s, dw(d, 1));
        let pitch: uint32_t = (dw(d, 1)
            & 0x3fc00000 as ::core::ffi::c_int as uint32_t)
            >> 16 as ::core::ffi::c_int;
        let dx: uint32_t = dw(d, 4) & 0xffff as uint32_t;
        let dy: uint32_t =
            dw(d, 4) >> 16 as ::core::ffi::c_int;
        let w: uint32_t = dw(d, 5) & 0xffff as uint32_t;
        let h: uint32_t =
            dw(d, 5) >> 16 as ::core::ffi::c_int;
        let vram = crate::src::qemu::Vram::of(s);
        let pal: [uint32_t; 256] = *win_pal.lock().unwrap_or_else(|e| e.into_inner());
        // Byte idx of the packed 8-bit pixel data that starts at dword 7 (native byte order).
        let px_byte = |idx: usize| d.get(7 + idx / 4).map_or(0, |v| v.to_ne_bytes()[idx % 4]);
        if pitch != 0
            && (off as uint64_t)
                .wrapping_add((dy.wrapping_add(h) as uint64_t).wrapping_mul(pitch as uint64_t))
                <= s.vram_size
        {
            gpu_vram_access(
                s,
                (off as uint64_t).wrapping_add((dy as uint64_t).wrapping_mul(pitch as uint64_t)),
                (off as uint64_t).wrapping_add(
                    (dy as uint64_t)
                        .wrapping_add(h as uint64_t)
                        .wrapping_mul(pitch as uint64_t),
                ),
                true,
            );
            let mut y: uint32_t = 0;
            while y < h {
                let mut x: uint32_t = 0;
                while x < w {
                    gpu_px_put(
                        s,
                        vram,
                        (off as uint64_t)
                            .wrapping_add(
                                (dy.wrapping_add(y) as uint64_t).wrapping_mul(pitch as uint64_t),
                            )
                            .wrapping_add(
                                (dx.wrapping_add(x) as uint64_t).wrapping_mul(4),
                            ),
                        pal[px_byte(y.wrapping_mul(w).wrapping_add(x) as usize) as usize],
                        4,
                    );
                    x = x.wrapping_add(1);
                }
                y = y.wrapping_add(1);
            }
            gpu_vram_dirty(
                s,
                (off as uint64_t).wrapping_add((dy as uint64_t).wrapping_mul(pitch as uint64_t)),
                (h as uint64_t).wrapping_mul(pitch as uint64_t),
            );
        }
        return true;
    }
    if opcode == 0x9c as uint32_t && g_seq_log_enabled.get() > 0 as ::core::ffi::c_int {
        let b: String = (0..n.min(20)).map(|i| format!(" {:08x}", dw(d, i as usize))).collect();
        crate::src::logs::seq(format_args!("TRANS9C n={}{}", ((n) as u32), b));
    }
    if opcode == 0x94 as uint32_t
        && n > 9
        && dw(d, 0) >> 12 as ::core::ffi::c_int & 3
            == 3
        && dw(d, 6) == n.wrapping_sub(7)
        && dw(d, 5) & 0xffff as uint32_t != 0
        && dw(d, 5) >> 16 as ::core::ffi::c_int != 0
    {
        // Rebuild the packet in the layout hostdata_mono expects: 9 header dwords, then the
        // n - 7 data dwords that follow the 7-dword header of the 0x94 packet.
        let mut t: Vec<uint32_t> = vec![0; n as usize + 2];
        t[0] = dw(d, 0);
        t[1] = dw(d, 1);
        t[2] = dw(d, 2);
        t[3] = dw(d, 3);
        t[6] = dw(d, 4);
        t[7] = dw(d, 5);
        for k in 0..n.wrapping_sub(7) as usize {
            t[9 + k] = dw(d, 7 + k);
        }
        return ppc_mac_gpu_hostdata_mono(s, &t, n.wrapping_add(2));
    }
    if opcode == 0x94 as uint32_t && n >= 9 {
        crate::src::logs::seq(format_args!("HD94 n={} d: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", ((n) as u32), ((dw(d, 0)) as u32), ((dw(d, 1)) as u32), ((dw(d, 2)) as u32), ((dw(d, 3)) as u32), ((dw(d, 4)) as u32), ((dw(d, 5)) as u32), ((dw(d, 6)) as u32), ((dw(d, 7)) as u32), ((dw(d, 8)) as u32), ((if n > 9 {
                dw(d, 9)
            } else {
                0
            }) as u32), ((if n > 10 {
                dw(d, 10)
            } else {
                0
            }) as u32)));
        return ppc_mac_gpu_hostdata_mono(s, d, n);
    }
    if opcode == 0x9b as uint32_t && g_seq_log_enabled.get() > 0 as ::core::ffi::c_int {
        let b: String = (0..n.min(14)).map(|i| format!(" {:08x}", dw(d, i as usize))).collect();
        crate::src::logs::seq(format_args!("BB9B n={}{}", ((n) as u32), b));
    }
    if opcode == 0x9b as uint32_t && n >= 4 {
        let gmc: uint32_t = dw(d, 0);
        let mut hdr: uint32_t = 1;
        let spo: uint32_t = if gmc & ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint32_t != 0 {
            let fresh39 = hdr;
            hdr = hdr.wrapping_add(1);
            dw(d, fresh39 as usize)
        } else {
            s.regs.default_pitch_offset
        };
        let dpo: uint32_t = if gmc & ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t != 0 {
            let fresh40 = hdr;
            hdr = hdr.wrapping_add(1);
            dw(d, fresh40 as usize)
        } else {
            s.regs.default_pitch_offset
        };
        if gmc & ((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as uint32_t != 0 {
            hdr = hdr.wrapping_add(1);
        }
        if gmc & ((1 as ::core::ffi::c_int) << 3 as ::core::ffi::c_int) as uint32_t != 0 {
            op.clip = true;
            op.clip_x0 = (dw(d, hdr as usize) & 0x3fff as uint32_t) as ::core::ffi::c_int;
            op.clip_y0 = (dw(d, hdr as usize) >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t)
                as ::core::ffi::c_int;
            op.clip_x1 = (dw(d, (hdr.wrapping_add(1)) as usize) & 0x3fff as uint32_t)
                as ::core::ffi::c_int;
            op.clip_y1 = (dw(d, (hdr.wrapping_add(1)) as usize)
                >> 16 as ::core::ffi::c_int
                & 0x3fff as uint32_t) as ::core::ffi::c_int;
            hdr = hdr.wrapping_add(2);
        }
        if hdr <= 6 {
            // hdr <= 6 here; a short packet leaves the saved header untouched.
            if let Some(src) = d.get(..hdr as usize) {
                bbm_hdr[..hdr as usize].copy_from_slice(src);
            }
            *bbm_hdr_n = hdr;
        }
        if n < hdr.wrapping_add(3) {
            return true;
        }
        op.gmc = (gmc as ::core::ffi::c_uint
            & !((7 as ::core::ffi::c_uint) << 24 as ::core::ffi::c_int)
            | (2 as ::core::ffi::c_uint) << 24 as ::core::ffi::c_int) as uint32_t;
        op.dst_off = gpu_po_offset(s, dpo);
        op.dst_pitch =
            (dpo & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int;
        op.src_off = gpu_po_offset(s, spo);
        op.src_pitch =
            (spo & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int;
        op.auto_dir = true;
        op.xl2r = true;
        op.yt2b = true;
        if op.dst_pitch == 0 || op.src_pitch == 0 {
            return true;
        }
        let mut i_1: uint32_t = hdr;
        while i_1.wrapping_add(3) <= n {
            op.sx = (dw(d, i_1 as usize) >> 16 as ::core::ffi::c_int & 0xffff as uint32_t)
                as ::core::ffi::c_int;
            op.sy = (dw(d, i_1 as usize) & 0xffff as uint32_t) as ::core::ffi::c_int;
            op.dx = (dw(d, (i_1.wrapping_add(1)) as usize)
                >> 16 as ::core::ffi::c_int
                & 0xffff as uint32_t) as ::core::ffi::c_int;
            op.dy = (dw(d, (i_1.wrapping_add(1)) as usize) & 0xffff as uint32_t)
                as ::core::ffi::c_int;
            op.w = dw(d, (i_1.wrapping_add(2)) as usize) >> 16 as ::core::ffi::c_int
                & 0x3fff as uint32_t;
            op.h = dw(d, (i_1.wrapping_add(2)) as usize) & 0x3fff as uint32_t;
            ppc_mac_gpu_2d_op(s, &op);
            i_1 = i_1.wrapping_add(3);
        }
        return true;
    }
    if opcode == 0x9a as uint32_t && n >= 3 {
        let gmc_0: uint32_t = dw(d, 0);
        let mut idx: uint32_t = 1;
        let mut dpo_0: uint32_t = s.regs.default_pitch_offset;
        let brush: uint32_t = gmc_0 >> 4 as ::core::ffi::c_int & 0xf as uint32_t;
        let rop: uint32_t = gmc_0 >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
        let bpp: uint32_t = gpu_gmc_bpp(gmc_0);
        if gmc_0 & ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t != 0 {
            let fresh41 = idx;
            idx = idx.wrapping_add(1);
            dpo_0 = dw(d, fresh41 as usize);
        }
        if gmc_0 & ((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as uint32_t != 0 {
            idx = idx.wrapping_add(1);
        }
        if gmc_0 & ((1 as ::core::ffi::c_int) << 3 as ::core::ffi::c_int) as uint32_t != 0
            && idx.wrapping_add(2) <= n
        {
            op.clip = true;
            op.clip_x0 = (dw(d, idx as usize) & 0x3fff as uint32_t) as ::core::ffi::c_int;
            op.clip_y0 = (dw(d, idx as usize) >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t)
                as ::core::ffi::c_int;
            op.clip_x1 = (dw(d, (idx.wrapping_add(1)) as usize) & 0x3fff as uint32_t)
                as ::core::ffi::c_int;
            op.clip_y1 = (dw(d, (idx.wrapping_add(1)) as usize)
                >> 16 as ::core::ffi::c_int
                & 0x3fff as uint32_t) as ::core::ffi::c_int;
            idx = idx.wrapping_add(2);
        }
        let color: uint32_t = if brush == 0xd as uint32_t {
            let fresh42 = idx;
            idx = idx.wrapping_add(1);
            dw(d, fresh42 as usize)
        } else {
            s.regs.dp_brush_frgd_clr
        };
        op.frgd = color;
        op.dst_off = gpu_po_offset(s, dpo_0);
        op.dst_pitch =
            (dpo_0 & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int;
        op.xl2r = true;
        op.yt2b = true;
        op.gmc = (if rop == 0xcc as uint32_t && brush == 0xd as uint32_t {
            gmc_0 as ::core::ffi::c_uint
                & !((0xff as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int)
                | (0xf0 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
        } else {
            gmc_0 as ::core::ffi::c_uint
        }) as uint32_t;
        if op.dst_pitch == 0 {
            return true;
        }
        let mut i_2: uint32_t = idx;
        while i_2.wrapping_add(2) <= n {
            op.dx = (dw(d, i_2 as usize) >> 16 as ::core::ffi::c_int & 0xffff as uint32_t)
                as ::core::ffi::c_int;
            op.dy = (dw(d, i_2 as usize) & 0xffff as uint32_t) as ::core::ffi::c_int;
            op.w = dw(d, (i_2.wrapping_add(1)) as usize) >> 16 as ::core::ffi::c_int
                & 0x3fff as uint32_t;
            op.h = dw(d, (i_2.wrapping_add(1)) as usize) & 0x3fff as uint32_t;
            ppc_mac_gpu_2d_op(s, &op);
            if brush == 0xd as uint32_t && (rop == 0xf0 as uint32_t || rop == 0xcc as uint32_t) {
                r200_fill_notify(
                    s,
                    op.dst_off,
                    op.dst_pitch,
                    op.dx as uint32_t,
                    op.dy as uint32_t,
                    op.w,
                    op.h,
                    bpp,
                    color,
                );
            }
            i_2 = i_2.wrapping_add(2);
        }
        return true;
    }
    return false;
}
/// The VRAM watch word at 0x353000, or 0 when the watch is off or the word is out of range.
fn vram_watch_word(s: &PPCMacGPUState) -> uint32_t {
    if s.vram_watch_active && (0x353000 + 4) as uint64_t <= s.vram_size {
        crate::src::qemu::Vram::of(s).get32(0x353000).unwrap_or(0)
    } else {
        0
    }
}

/// State carried between BBM packets: the saved header dwords and their count.
struct BbmState {
    hdr: [uint32_t; 6],
    hdr_n: uint32_t,
}

static BBM_STATE: std::sync::Mutex<BbmState> =
    std::sync::Mutex::new(BbmState { hdr: [0; 6], hdr_n: 0 });

fn bbm_state_load() -> ([uint32_t; 6], uint32_t) {
    let st = BBM_STATE.lock().unwrap_or_else(|e| e.into_inner());
    (st.hdr, st.hdr_n)
}

fn bbm_state_store(hdr: [uint32_t; 6], hdr_n: uint32_t) {
    let mut st = BBM_STATE.lock().unwrap_or_else(|e| e.into_inner());
    st.hdr = hdr;
    st.hdr_n = hdr_n;
}

pub fn ppc_mac_gpu_process_pm4(s: &mut PPCMacGPUState, pm4: &[uint32_t]) {
    g_in_pm4.set(g_in_pm4.get() + 1);
    let size_dw: uint32_t = pm4.len() as uint32_t;
    let mut i: uint32_t = 0 as uint32_t;
    while i < size_dw {
        let hdr: uint32_t = dw(pm4, (i) as usize);
        let type_0: uint32_t = hdr >> 30 as ::core::ffi::c_int & 3 as uint32_t;
        if r300_ringdump() {
            let n: uint32_t = if type_0 == 2 as uint32_t {
                0 as uint32_t
            } else {
                (hdr >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t).wrapping_add(1 as uint32_t)
            };
            crate::src::qemu::ringdump_write(
                format!(
                    "{}{} {:08x}:",
                    b"0123"[(type_0 & 3) as usize] as char,
                    g_in_pm4.get(),
                    hdr,
                )
                .as_bytes(),
            );
            let mut k: uint32_t = 0 as uint32_t;
            while k < n
                && i.wrapping_add(1 as uint32_t).wrapping_add(k) < size_dw
                && k < 512 as uint32_t
            {
                crate::src::logs::err(format_args!(" {:08x}", ((dw(pm4, (i.wrapping_add(1 as uint32_t).wrapping_add(k)) as usize)) as u32)));
                k = k.wrapping_add(1);
            }
            crate::src::qemu::ringdump_write(b"\n");
        }
        i = i.wrapping_add(1);
        if type_0 == 0 as uint32_t {
            let mut pkt_count: uint32_t =
                (hdr >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t).wrapping_add(1 as uint32_t);
            let reg_base: uint32_t = hdr & 0x7fff as uint32_t;
            let one_reg_wr: bool = hdr >> 15 as ::core::ffi::c_int & 1 as uint32_t != 0;
            let vw_before: uint32_t = vram_watch_word(s);
            if i.wrapping_add(pkt_count) > size_dw {
                pkt_count = size_dw.wrapping_sub(i);
            }
            ppc_mac_gpu_pm4_process_type0(
                s,
                reg_base,
                one_reg_wr,
                pkt_count,
                &pm4[(i as usize).min(pm4.len())..],
            );
            let vw_after: uint32_t = vram_watch_word(s);
            if s.vram_watch_active && vw_after != vw_before {
                crate::src::logs::err(format_args!("[VRAM_CLR_FOUND] Type0 reg=0x{:04x} count={} CHANGED vram[0x353000] from 0x{:08x} to 0x{:08x}\n", ((reg_base.wrapping_mul(4 as uint32_t)) as u32), ((pkt_count) as u32), ((vw_before) as u32), ((vw_after) as u32)));
            }
            i = i.wrapping_add(pkt_count);
        } else {
            if type_0 == 2 as uint32_t {
                continue;
            }
            if type_0 == 3 as uint32_t {
                let pkt_count_0: uint32_t = (hdr >> 16 as ::core::ffi::c_int
                    & 0x3fff as uint32_t)
                    .wrapping_add(2 as uint32_t);
                let mut opcode: uint32_t = hdr >> 8 as ::core::ffi::c_int & 0xff as uint32_t;
                let mut body_dw: uint32_t = pkt_count_0.wrapping_sub(1 as uint32_t);
                let body_adv: uint32_t = body_dw;
                let mut d: &[uint32_t] = pm4.get(i as usize..).unwrap_or(&[]);
                if !s.r3.is_null()
                    && ppc_mac_gpu_r300_packet3(s, opcode, d, body_dw) as ::core::ffi::c_int != 0
                {
                    opcode = 0x10 as uint32_t;
                }
                if opcode != 0x34 as uint32_t
                    && opcode != 0x35 as uint32_t
                    && opcode != 0x36 as uint32_t
                {
                    crate::src::logs::seq(format_args!("T3   op={:02x} n={} d0={:08x} d1={:08x} d2={:08x}", ((opcode) as u32), ((body_dw) as u32), ((if body_dw > 0 as uint32_t {
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
                let t3_vw_before: uint32_t = vram_watch_word(s);
                let (mut bbm_hdr, mut bbm_hdr_n) = bbm_state_load();
                let mut bbm_cont: [uint32_t; 256] = [0; 256];
                if opcode == 0x1b as uint32_t
                    && bbm_hdr_n != 0
                    && body_dw >= 3 as uint32_t
                    && bbm_hdr_n.wrapping_add(body_dw) as usize <= bbm_cont.len()
                {
                    let n = bbm_hdr_n as usize;
                    let m = body_dw as usize;
                    if let (Some(dst), Some(hdr)) = (bbm_cont.get_mut(..n), bbm_hdr.get(..n)) {
                        dst.copy_from_slice(hdr);
                    }
                    // A packet that runs past the end of the buffer copies what is there.
                    let avail = d.len().min(m);
                    if let Some(dst) = bbm_cont.get_mut(n..n + avail) {
                        dst.copy_from_slice(&d[..avail]);
                    }
                    d = &bbm_cont[..];
                    body_dw = body_dw.wrapping_add(bbm_hdr_n);
                    opcode = 0x9b as uint32_t;
                }
                let handled_2d = gpu_unified_2d() && {
                    let r = ppc_mac_gpu_pm4_2d(
                        s,
                        opcode,
                        d,
                        body_dw,
                        &mut bbm_hdr,
                        &mut bbm_hdr_n,
                    );
                    bbm_state_store(bbm_hdr, bbm_hdr_n);
                    r
                };
                if !handled_2d {
                    if opcode == 0x9b as uint32_t && body_dw >= 4 as uint32_t {
                        let gmc: uint32_t = dw(d, (0 as ::core::ffi::c_int) as usize);
                        let rop3: uint32_t = gmc >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
                        let mut hdr_dw: uint32_t = 1 as uint32_t;
                        let mut src_po: uint32_t = 0;
                        let mut dst_po: uint32_t = 0;
                        if gmc & ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint32_t
                            != 0
                        {
                            let fresh30 = hdr_dw;
                            hdr_dw = hdr_dw.wrapping_add(1);
                            src_po = dw(d, (fresh30) as usize);
                        } else {
                            src_po = s.regs.default_pitch_offset;
                        }
                        if gmc & ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t
                            != 0
                        {
                            let fresh31 = hdr_dw;
                            hdr_dw = hdr_dw.wrapping_add(1);
                            dst_po = dw(d, (fresh31) as usize);
                        } else {
                            dst_po = s.regs.default_pitch_offset;
                        }
                        let mut bbm_clip_tl: uint32_t = 0 as uint32_t;
                        let mut bbm_clip_br: uint32_t =
                            0x3fff3fff as ::core::ffi::c_int as uint32_t;
                        if gmc & ((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as uint32_t
                            != 0
                        {
                            hdr_dw = hdr_dw.wrapping_add(1);
                        }
                        if gmc & ((1 as ::core::ffi::c_int) << 3 as ::core::ffi::c_int) as uint32_t
                            != 0
                        {
                            let fresh32 = hdr_dw;
                            hdr_dw = hdr_dw.wrapping_add(1);
                            bbm_clip_tl = dw(d, (fresh32) as usize);
                            let fresh33 = hdr_dw;
                            hdr_dw = hdr_dw.wrapping_add(1);
                            bbm_clip_br = dw(d, (fresh33) as usize);
                        }
                        if let (Some(dst), Some(src)) = (
                            bbm_hdr.get_mut(..hdr_dw as usize),
                            d.get(..hdr_dw as usize),
                        ) {
                            dst.copy_from_slice(src);
                            bbm_hdr_n = hdr_dw;
                            bbm_state_store(bbm_hdr, bbm_hdr_n);
                        }
                        if body_dw < hdr_dw.wrapping_add(3 as uint32_t) {
                            i = i.wrapping_add(body_adv);
                            continue;
                        } else {
                            let dst_offset: uint32_t = gpu_po_offset(s, dst_po);
                            let dst_pitch: uint32_t = (dst_po
                                & 0x3fc00000 as ::core::ffi::c_int as uint32_t)
                                >> 16 as ::core::ffi::c_int;
                            let src_offset: uint32_t = gpu_po_offset(s, src_po);
                            let src_pitch: uint32_t = (src_po
                                & 0x3fc00000 as ::core::ffi::c_int as uint32_t)
                                >> 16 as ::core::ffi::c_int;
                            if dst_po >> 30 as ::core::ffi::c_int & 1 as uint32_t != 0 {
                                mc_register_tiled_surface(
                                    s,
                                    dst_offset,
                                    dst_pitch,
                                    c"BBM_dst".as_ptr(),
                                );
                            }
                            if src_po >> 30 as ::core::ffi::c_int & 1 as uint32_t != 0 {
                                mc_register_tiled_surface(
                                    s,
                                    src_offset,
                                    src_pitch,
                                    c"BBM_src".as_ptr(),
                                );
                            }
                            let bpp: uint32_t = gmc_dst_bpp(gmc);
                            if dst_pitch == 0 as uint32_t || src_pitch == 0 as uint32_t {
                                i = i.wrapping_add(body_adv);
                                continue;
                            } else {
                                let vram = crate::src::qemu::Vram::of(s);
                                let num_blits: uint32_t =
                                    body_dw.wrapping_sub(hdr_dw).wrapping_div(3 as uint32_t);
                                crate::src::logs::seq(format_args!("BBMRAW gmc={:08x} clip_tl={:08x} clip_br={:08x} n={} t0={:08x} {:08x} {:08x}", ((gmc) as u32), ((bbm_clip_tl) as u32), ((bbm_clip_br) as u32), ((num_blits) as u32), ((dw(d, (hdr_dw) as usize)) as u32), ((dw(d, (hdr_dw.wrapping_add(1 as uint32_t)) as usize)) as u32), ((dw(d, (hdr_dw.wrapping_add(2 as uint32_t)) as usize)) as u32)));
                                let use_scissors: bool = gmc
                                    & ((1 as ::core::ffi::c_int) << 3 as ::core::ffi::c_int)
                                        as uint32_t
                                    != 0 as uint32_t;
                                let mut sc_left: uint32_t = 0 as uint32_t;
                                let mut sc_top: uint32_t = 0 as uint32_t;
                                let mut sc_right: uint32_t = 0x3fff as uint32_t;
                                let mut sc_bottom: uint32_t = 0x3fff as uint32_t;
                                if use_scissors {
                                    sc_left = bbm_clip_tl & 0x3fff as uint32_t;
                                    sc_top = bbm_clip_tl >> 16 as ::core::ffi::c_int
                                        & 0x3fff as uint32_t;
                                    sc_right = bbm_clip_br & 0x3fff as uint32_t;
                                    sc_bottom = bbm_clip_br >> 16 as ::core::ffi::c_int
                                        & 0x3fff as uint32_t;
                                }
                                static bbm_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
                                if bbm_log.get() < 200 as ::core::ffi::c_int {
                                    {
 let mut out = String::new();
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\n=== BBM[{}] ===\n", ((bbm_log.get()) as i32)));
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("raw[{}]:", ((body_dw) as u32)));
                                        let mut dd: uint32_t = 0 as uint32_t;
                                        while dd < body_dw && dd < 30 as uint32_t {
                                            let _ = std::fmt::Write::write_fmt(&mut out, format_args!(" {:08x}", ((dw(d, (dd) as usize)) as u32)));
                                            dd = dd.wrapping_add(1);
                                        }
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\n"));
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("gmc=0x{:08x} rop=0x{:02x} src_type={} dst_clipping={}\n", ((gmc) as u32), ((rop3) as u32), ((gmc >> 24 as ::core::ffi::c_int & 7 as uint32_t) as u32), ((use_scissors as ::core::ffi::c_int) as u32)));
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("hdr_dw={} src_po={} dst_po={}\n", ((hdr_dw) as u32), crate::src::qemu::cstr(if gmc
                                                & ((1 as ::core::ffi::c_int)
                                                    << 0 as ::core::ffi::c_int)
                                                    as uint32_t
                                                != 0
                                            {
                                                c"packet".as_ptr()
                                            } else {
                                                c"default".as_ptr()
                                            }), crate::src::qemu::cstr(if gmc
                                                & ((1 as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    as uint32_t
                                                != 0
                                            {
                                                c"packet".as_ptr()
                                            } else {
                                                c"default".as_ptr()
                                            })));
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("src_po=0x{:08x}: off=0x{:06x} pitch={}({}px) tile={}\n", ((src_po) as u32), ((src_offset) as u32), ((src_pitch) as u32), ((src_pitch.wrapping_div(4 as uint32_t)) as u32), ((src_po >> 30 as ::core::ffi::c_int & 3 as uint32_t) as u32)));
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("dst_po=0x{:08x}: off=0x{:06x} pitch={}({}px) tile={}\n", ((dst_po) as u32), ((dst_offset) as u32), ((dst_pitch) as u32), ((dst_pitch.wrapping_div(4 as uint32_t)) as u32), ((dst_po >> 30 as ::core::ffi::c_int & 3 as uint32_t) as u32)));
                                        if use_scissors {
                                            let _ = std::fmt::Write::write_fmt(&mut out, format_args!("scissors: ({},{})-({},{})\n", ((sc_left) as u32), ((sc_top) as u32), ((sc_right) as u32), ((sc_bottom) as u32)));
                                        }
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("num_blits={}\n", ((num_blits) as u32)));
                                        let mut bb: uint32_t = 0 as uint32_t;
                                        while bb < num_blits && bb < 10 as uint32_t {
                                            let sy: uint32_t = dw(d, (hdr_dw
                                                    .wrapping_add(bb.wrapping_mul(3 as uint32_t))
                                                    .wrapping_add(0 as uint32_t)) as usize);
                                            let dy: uint32_t = dw(d, (hdr_dw
                                                    .wrapping_add(bb.wrapping_mul(3 as uint32_t))
                                                    .wrapping_add(1 as uint32_t)) as usize);
                                            let wh: uint32_t = dw(d, (hdr_dw
                                                    .wrapping_add(bb.wrapping_mul(3 as uint32_t))
                                                    .wrapping_add(2 as uint32_t)) as usize);
                                            let _ = std::fmt::Write::write_fmt(&mut out, format_args!("  blit{} raw: SYX=0x{:08x} DYX=0x{:08x} WHH=0x{:08x}\n", ((bb) as u32), ((sy) as u32), ((dy) as u32), ((wh) as u32)));
                                            let _ = std::fmt::Write::write_fmt(&mut out, format_args!("    if Y_X,H_W: src=({},{}) dst=({},{}) {}x{}\n", ((sy & 0xffff as uint32_t) as u32), ((sy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((dy & 0xffff as uint32_t) as u32), ((dy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((wh & 0x3fff as uint32_t) as u32), ((wh >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t) as u32)));
                                            let _ = std::fmt::Write::write_fmt(&mut out, format_args!("    if X_Y,W_H: src=({},{}) dst=({},{}) {}x{}\n", ((sy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((sy & 0xffff as uint32_t) as u32), ((dy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((dy & 0xffff as uint32_t) as u32), ((wh >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t) as u32), ((wh & 0x3fff as uint32_t) as u32)));
                                            let yx_dx: uint32_t = dy & 0xffff as uint32_t;
                                            let yx_dy: uint32_t =
                                                dy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t;
                                            let yx_w: uint32_t = wh & 0x3fff as uint32_t;
                                            let yx_h: uint32_t =
                                                wh >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                                            let xy_dx: uint32_t =
                                                dy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t;
                                            let xy_dy: uint32_t = dy & 0xffff as uint32_t;
                                            let xy_w: uint32_t =
                                                wh >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                                            let xy_h: uint32_t = wh & 0x3fff as uint32_t;
                                            let _ = std::fmt::Write::write_fmt(&mut out, format_args!("    Y_X dst_end=({},{}) X_Y dst_end=({},{})\n", ((yx_dx.wrapping_add(yx_w)) as u32), ((yx_dy.wrapping_add(yx_h)) as u32), ((xy_dx.wrapping_add(xy_w)) as u32), ((xy_dy.wrapping_add(xy_h)) as u32)));
                                            bb = bb.wrapping_add(1);
                                        }
                                        crate::src::qemu::append_file("/tmp/gpu_blit_diag.txt", &out);
                                    }
                                    bbm_log.set(bbm_log.get() + 1);
                                }
                                g_blit_stats_mut().bbm_count = g_blit_stats_mut().bbm_count.wrapping_add(1);
                                let raw_src_mt: bool =
                                    src_po >> 30 as ::core::ffi::c_int & 1 as uint32_t != 0;
                                let raw_dst_mt: bool =
                                    dst_po >> 30 as ::core::ffi::c_int & 1 as uint32_t != 0;
                                static bbm_logged: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0 as uint64_t);
                                if bbm_logged.get() < 500 as uint64_t {
                                    let mut dst_region: *const ::core::ffi::c_char =
                                        c"offscreen".as_ptr();
                                    if dst_offset == 0 as uint32_t
                                        && (dst_pitch == 4096 as uint32_t
                                            || dst_pitch == 3200 as uint32_t)
                                    {
                                        dst_region = c"FRAMEBUFFER".as_ptr();
                                    } else if dst_offset
                                        >= 0x200000 as ::core::ffi::c_int as uint32_t
                                    {
                                        dst_region =
                                            c"backbuf".as_ptr();
                                    }
                                    crate::src::logs::blit_path("BBM", format_args!("rop=0x{:02x} n={}  src_po=0x{:08x}[off=0x{:06x} pitch={} mt={}]  dst_po=0x{:08x}[off=0x{:06x} pitch={} mt={}]  dst={}", ((rop3) as u32), ((num_blits) as u32), ((src_po) as u32), ((src_offset) as u32), ((src_pitch) as u32), ((raw_src_mt as ::core::ffi::c_int) as i32), ((dst_po) as u32), ((dst_offset) as u32), ((dst_pitch) as u32), ((raw_dst_mt as ::core::ffi::c_int) as i32), crate::src::qemu::cstr(dst_region)));
                                    bbm_logged.set(bbm_logged.get().wrapping_add(1));
                                }
                                let mut current_block_190: u64;
                                let mut b: uint32_t = 0 as uint32_t;
                                while b < num_blits {
                                    let src_xy: uint32_t = dw(d, (hdr_dw
                                            .wrapping_add(b.wrapping_mul(3 as uint32_t))
                                            .wrapping_add(0 as uint32_t)) as usize);
                                    let dst_xy: uint32_t = dw(d, (hdr_dw
                                            .wrapping_add(b.wrapping_mul(3 as uint32_t))
                                            .wrapping_add(1 as uint32_t)) as usize);
                                    let dwh: uint32_t = dw(d, (hdr_dw
                                            .wrapping_add(b.wrapping_mul(3 as uint32_t))
                                            .wrapping_add(2 as uint32_t)) as usize);
                                    let mut src_x: uint32_t =
                                        src_xy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t;
                                    let mut src_y: uint32_t = src_xy & 0xffff as uint32_t;
                                    let mut dst_x: uint32_t =
                                        dst_xy >> 16 as ::core::ffi::c_int & 0xffff as uint32_t;
                                    let mut dst_y: uint32_t = dst_xy & 0xffff as uint32_t;
                                    let mut blit_w: uint32_t =
                                        dwh >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                                    let mut blit_h: uint32_t = dwh & 0x3fff as uint32_t;
                                    if !(blit_w == 0 as uint32_t || blit_h == 0 as uint32_t) {
                                        if use_scissors {
                                            let dst_x2: uint32_t = dst_x.wrapping_add(blit_w);
                                            let dst_y2: uint32_t = dst_y.wrapping_add(blit_h);
                                            if dst_x2 <= sc_left
                                                || dst_x >= sc_right
                                                || dst_y2 <= sc_top
                                                || dst_y >= sc_bottom
                                            {
                                                current_block_190 = 5248622017361056354;
                                            } else {
                                                if dst_x < sc_left {
                                                    let clip: uint32_t =
                                                        sc_left.wrapping_sub(dst_x);
                                                    src_x = src_x.wrapping_add(clip);
                                                    dst_x = sc_left;
                                                    blit_w = blit_w.wrapping_sub(clip);
                                                }
                                                if dst_y < sc_top {
                                                    let clip_0: uint32_t =
                                                        sc_top.wrapping_sub(dst_y);
                                                    src_y = src_y.wrapping_add(clip_0);
                                                    dst_y = sc_top;
                                                    blit_h = blit_h.wrapping_sub(clip_0);
                                                }
                                                if dst_x.wrapping_add(blit_w) > sc_right {
                                                    blit_w = sc_right.wrapping_sub(dst_x);
                                                }
                                                if dst_y.wrapping_add(blit_h) > sc_bottom {
                                                    blit_h = sc_bottom.wrapping_sub(dst_y);
                                                }
                                                if blit_w == 0 as uint32_t
                                                    || blit_h == 0 as uint32_t
                                                {
                                                    current_block_190 = 5248622017361056354;
                                                } else {
                                                    current_block_190 = 13174377073168946860;
                                                }
                                            }
                                        } else {
                                            current_block_190 = 13174377073168946860;
                                        }
                                        match current_block_190 {
                                            5248622017361056354 => {}
                                            _ => {
                                                gpu_vram_access(
                                                    s,
                                                    (dst_offset as uint64_t).wrapping_add(
                                                        (dst_y as uint64_t)
                                                            .wrapping_mul(dst_pitch as uint64_t),
                                                    ),
                                                    (dst_offset as uint64_t).wrapping_add(
                                                        (dst_y as uint64_t)
                                                            .wrapping_add(blit_h as uint64_t)
                                                            .wrapping_mul(dst_pitch as uint64_t),
                                                    ),
                                                    1 as ::core::ffi::c_int != 0,
                                                );
                                                gpu_vram_access(
                                                    s,
                                                    (src_offset as uint64_t).wrapping_add(
                                                        (src_y as uint64_t)
                                                            .wrapping_mul(src_pitch as uint64_t),
                                                    ),
                                                    (src_offset as uint64_t).wrapping_add(
                                                        (src_y as uint64_t)
                                                            .wrapping_add(blit_h as uint64_t)
                                                            .wrapping_mul(src_pitch as uint64_t),
                                                    ),
                                                    0 as ::core::ffi::c_int != 0,
                                                );
                                                crate::src::logs::seq(format_args!("BBM  rop={:02x} src={:06x}/{} ({},{}) dst={:06x}/{} ({},{}) {}x{}", ((rop3) as u32), ((src_offset) as u32), ((src_pitch) as u32), ((src_x) as u32), ((src_y) as u32), ((dst_offset) as u32), ((dst_pitch) as u32), ((dst_x) as u32), ((dst_y) as u32), ((blit_w) as u32), ((blit_h) as u32)));
                                                if rop3 == 0xcc as uint32_t
                                                    && src_offset
                                                        >= 0x100000 as ::core::ffi::c_int
                                                            as uint32_t
                                                {
                                                    blit_region_record(
                                                        src_offset, src_pitch, src_x, src_y, dst_x,
                                                        dst_y, blit_w, blit_h,
                                                    );
                                                }
                                                if rop3 == 0xcc as uint32_t {
                                                    let handled_by_renderer: bool =
                                                        0 as ::core::ffi::c_int != 0;
                                                    if dst_offset
                                                        <= 0x10000 as ::core::ffi::c_int as uint32_t
                                                        && blit_w >= 4 as uint32_t
                                                        && blit_h >= 4 as uint32_t
                                                    {
                                                        let is_gart_src: bool =
                                                            s.regs.aic_ctrl & 1 as uint32_t != 0
                                                                && src_offset
                                                                    >= s.regs.aic_lo_addr
                                                                && src_offset
                                                                    <= s.regs.aic_hi_addr;
                                                        let mut test_phys: hwaddr = 0;
                                                        let is_agp_src: bool = !is_gart_src
                                                            && ppc_mac_gpu_agp_translate(
                                                                s,
                                                                src_offset,
                                                                &mut test_phys,
                                                            )
                                                                as ::core::ffi::c_int
                                                                != 0;
                                                        fb_write_watch(
                                                            c"bbm_direct".as_ptr(),
                                                            if is_gart_src as ::core::ffi::c_int
                                                                != 0
                                                            {
                                                                c"gart".as_ptr()
                                                            } else if is_agp_src
                                                                as ::core::ffi::c_int
                                                                != 0
                                                            {
                                                                c"agp".as_ptr()
                                                            } else {
                                                                c"vram".as_ptr()
                                                            },
                                                            src_offset,
                                                            dst_offset,
                                                            dst_x,
                                                            dst_y,
                                                            blit_w,
                                                            blit_h,
                                                            0 as ::core::ffi::c_int != 0,
                                                            1 as ::core::ffi::c_int != 0,
                                                        );
                                                    }
                                                    let bbm_src_gart: bool =
                                                        s.regs.aic_ctrl & 1 as uint32_t != 0
                                                            && src_offset >= s.regs.aic_lo_addr
                                                            && src_offset <= s.regs.aic_hi_addr;
                                                    let mut bbm_src_agp: bool =
                                                        0 as ::core::ffi::c_int != 0;
                                                    if !bbm_src_gart {
                                                        let mut test_phys_0: hwaddr = 0;
                                                        bbm_src_agp = ppc_mac_gpu_agp_translate(
                                                            s,
                                                            src_offset,
                                                            &mut test_phys_0,
                                                        );
                                                    }
                                                    if bbm_src_gart as ::core::ffi::c_int != 0
                                                        || bbm_src_agp as ::core::ffi::c_int != 0
                                                    {
                                                        let mut row: uint32_t = 0 as uint32_t;
                                                        while row < blit_h {
                                                            let mut col: uint32_t = 0 as uint32_t;
                                                            while col < blit_w {
                                                                let gpu_addr: uint32_t =
                                                                    src_offset
                                                                        .wrapping_add(
                                                                            src_y
                                                                                .wrapping_add(row)
                                                                                .wrapping_mul(
                                                                                    src_pitch,
                                                                                ),
                                                                        )
                                                                        .wrapping_add(
                                                                            src_x
                                                                                .wrapping_add(col)
                                                                                .wrapping_mul(bpp),
                                                                        );
                                                                let mut phys: hwaddr = 0;
                                                                let mut pixel: uint32_t =
                                                                    0 as uint32_t;
                                                                if ppc_mac_gpu_agp_translate(
                                                                    s,
                                                                    gpu_addr,
                                                                    &mut phys,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    != 0
                                                                    || ppc_mac_gpu_gart_translate(
                                                                        s,
                                                                        gpu_addr,
                                                                        &mut phys,
                                                                    )
                                                                        as ::core::ffi::c_int
                                                                        != 0
                                                                {
                                                                    crate::src::qemu::guest_read32(phys as u64, &mut pixel);
                                                                }
                                                                let d_linear: uint64_t = (dst_offset as uint64_t)
                                                                    .wrapping_add(
                                                                        (dst_y.wrapping_add(row) as uint64_t)
                                                                            .wrapping_mul(dst_pitch as uint64_t),
                                                                    )
                                                                    .wrapping_add(
                                                                        (dst_x.wrapping_add(col) as uint64_t)
                                                                            .wrapping_mul(bpp as uint64_t),
                                                                    );
                                                                crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_process_pm4", 6856);
                                                                mc_vram_write32(
                                                                    s, vram, d_linear, pixel,
                                                                );
                                                                col = col.wrapping_add(1);
                                                            }
                                                            row = row.wrapping_add(1);
                                                        }
                                                    } else {
                                                        let mut rev_rows: bool =
                                                            0 as ::core::ffi::c_int != 0;
                                                        let mut rev_cols: bool =
                                                            0 as ::core::ffi::c_int != 0;
                                                        if blit_overlaps(
                                                            src_offset, src_pitch, src_x, src_y,
                                                            dst_offset, dst_pitch, dst_x, dst_y,
                                                            blit_w, blit_h, bpp,
                                                        ) {
                                                            rev_rows = dst_y > src_y;
                                                            rev_cols =
                                                                dst_y == src_y && dst_x > src_x;
                                                        }
                                                        if !blit_rect_untiled(
                                                            s, vram, src_offset, src_pitch, src_x,
                                                            src_y, dst_offset, dst_pitch, dst_x,
                                                            dst_y, blit_w, blit_h, bpp,
                                                        ) {
                                                            let mut r: uint32_t = 0 as uint32_t;
                                                            while r < blit_h {
                                                                let row_0: uint32_t =
                                                                    if rev_rows
                                                                        as ::core::ffi::c_int
                                                                        != 0
                                                                    {
                                                                        blit_h
                                                                            .wrapping_sub(
                                                                                1 as uint32_t,
                                                                            )
                                                                            .wrapping_sub(r)
                                                                    } else {
                                                                        r
                                                                    };
                                                                let mut c: uint32_t = 0 as uint32_t;
                                                                while c < blit_w {
                                                                    let col_0: uint32_t =
                                                                        if rev_cols
                                                                            as ::core::ffi::c_int
                                                                            != 0
                                                                        {
                                                                            blit_w
                                                                                .wrapping_sub(
                                                                                    1 as uint32_t,
                                                                                )
                                                                                .wrapping_sub(c)
                                                                        } else {
                                                                            c
                                                                        };
                                                                    let s_linear: uint64_t = (src_offset as uint64_t)
                                                                        .wrapping_add(
                                                                            (src_y.wrapping_add(row_0) as uint64_t)
                                                                                .wrapping_mul(src_pitch as uint64_t),
                                                                        )
                                                                        .wrapping_add(
                                                                            (src_x.wrapping_add(col_0) as uint64_t)
                                                                                .wrapping_mul(bpp as uint64_t),
                                                                        );
                                                                    let d_linear_0: uint64_t = (dst_offset as uint64_t)
                                                                        .wrapping_add(
                                                                            (dst_y.wrapping_add(row_0) as uint64_t)
                                                                                .wrapping_mul(dst_pitch as uint64_t),
                                                                        )
                                                                        .wrapping_add(
                                                                            (dst_x.wrapping_add(col_0) as uint64_t)
                                                                                .wrapping_mul(bpp as uint64_t),
                                                                        );
                                                                    let pixel_0: uint32_t =
                                                                        mc_vram_read32(
                                                                            s, vram, s_linear,
                                                                        );
                                                                    crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_process_pm4", 6902);
                                                                    mc_vram_write32(
                                                                        s, vram, d_linear_0,
                                                                        pixel_0,
                                                                    );
                                                                    c = c.wrapping_add(1);
                                                                }
                                                                r = r.wrapping_add(1);
                                                            }
                                                        }
                                                    }
                                                    let dirty_start: uint64_t = (dst_offset
                                                        as uint64_t)
                                                        .wrapping_add(
                                                            (dst_y as uint64_t).wrapping_mul(
                                                                dst_pitch as uint64_t,
                                                            ),
                                                        )
                                                        .wrapping_add(
                                                            (dst_x as uint64_t)
                                                                .wrapping_mul(bpp as uint64_t),
                                                        );
                                                    let dirty_len: uint64_t = if blit_h
                                                        > 1 as uint32_t
                                                    {
                                                        (blit_h.wrapping_sub(1 as uint32_t)
                                                            as uint64_t)
                                                            .wrapping_mul(dst_pitch as uint64_t)
                                                            .wrapping_add(
                                                                (blit_w as uint64_t)
                                                                    .wrapping_mul(bpp as uint64_t),
                                                            )
                                                    } else {
                                                        (blit_w as uint64_t)
                                                            .wrapping_mul(bpp as uint64_t)
                                                    };
                                                    if dirty_start.wrapping_add(dirty_len)
                                                        <= s.vram_size
                                                    {
                                                        gpu_vram_dirty(s, dirty_start, dirty_len);
                                                    }
                                                    if !handled_by_renderer {
                                                        if dst_offset == s.regs.crtc_offset {
                                                            frame_tracker_record_2d_event(
                                                                PASS_EVENT_FALLBACK_2D,
                                                                src_offset,
                                                                src_pitch,
                                                                src_offset,
                                                                dst_offset,
                                                                src_x,
                                                                src_y,
                                                                dst_x,
                                                                dst_y,
                                                                blit_w,
                                                                blit_h,
                                                                0 as ::core::ffi::c_int != 0,
                                                            );
                                                        } else {
                                                            frame_tracker_record_2d_event(
                                                                PASS_EVENT_BLIT_UPLOAD,
                                                                dst_offset,
                                                                dst_pitch,
                                                                src_offset,
                                                                dst_offset,
                                                                src_x,
                                                                src_y,
                                                                dst_x,
                                                                dst_y,
                                                                blit_w,
                                                                blit_h,
                                                                0 as ::core::ffi::c_int != 0,
                                                            );
                                                        }
                                                    }
                                                    if dst_offset == s.regs.crtc_offset
                                                        && rop3 == 0xcc as uint32_t
                                                        && blit_w >= 256 as uint32_t
                                                        && blit_h >= 256 as uint32_t
                                                    {
                                                        let mut src_rendered: bool =
                                                            0 as ::core::ffi::c_int != 0;
                                                        let mut rt: ::core::ffi::c_int =
                                                            0 as ::core::ffi::c_int;
                                                        while rt < s.metal_rt_count {
                                                            if s.metal_rt_offsets[rt as usize]
                                                                == src_offset
                                                            {
                                                                src_rendered =
                                                                    1 as ::core::ffi::c_int != 0;
                                                                break;
                                                            } else {
                                                                rt += 1;
                                                            }
                                                        }
                                                        static present_blit_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
                                                        if present_blit_log.get()
                                                            < 50 as ::core::ffi::c_int
                                                        {
                                                            crate::src::logs::err(format_args!("[PRESENT_BLIT] PM4_BBM: src=0x{:x}+{} dst=0x{:x}+{} sxy={},{} dxy={},{} {}x{} crtc_stride={}\n", ((src_offset) as u32), ((src_pitch) as u32), ((dst_offset) as u32), ((dst_pitch) as u32), ((src_x) as u32), ((src_y) as u32), ((dst_x) as u32), ((dst_y) as u32), ((blit_w) as u32), ((blit_h) as u32), ((s.disp.stride) as u32)));
                                                            crate::src::logs::err(format_args!("[PRESENT_SOURCE] src_off=0x{:x} dst_off=0x{:x} size=({},{}) rendered_this_frame={} source_kind={} matches_latest_rt={} note=PM4_BBM_visible_blit\n", ((src_offset) as u32), ((dst_offset) as u32), ((blit_w) as u32), ((blit_h) as u32), crate::src::qemu::cstr(if src_rendered as ::core::ffi::c_int != 0 {
                                                                    c"yes".as_ptr()
                                                                } else {
                                                                    c"no".as_ptr()
                                                                }), crate::src::qemu::cstr(if src_rendered as ::core::ffi::c_int != 0 {
                                                                    c"metal_result".as_ptr()
                                                                } else {
                                                                    c"raw_vram".as_ptr()
                                                                }), crate::src::qemu::cstr(if src_rendered as ::core::ffi::c_int != 0 {
                                                                    c"yes".as_ptr()
                                                                } else {
                                                                    c"no".as_ptr()
                                                                })));
                                                            present_blit_log.set(present_blit_log.get() + 1);
                                                        }
                                                        if dst_pitch > 0 as uint32_t
                                                            && s.disp.bpp == 32 as uint32_t
                                                        {
                                                            if !s.disp_stride_override_active
                                                                || s.disp_stride_override_value
                                                                    != dst_pitch
                                                            {
                                                                static so_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
                                                                if so_log.get() < 20 as ::core::ffi::c_int
                                                                {
                                                                    crate::src::logs::err(format_args!("[STRIDE_CHANGE] PM4_BBM: override {} {} -> {}\n", crate::src::qemu::cstr(if s.disp_stride_override_active as ::core::ffi::c_int
                                                                            != 0
                                                                        {
                                                                            c"update".as_ptr()
                                                                        } else {
                                                                            c"activate".as_ptr()
                                                                        }), ((if s.disp_stride_override_active as ::core::ffi::c_int
                                                                            != 0
                                                                        {
                                                                            s.disp_stride_override_value
                                                                        } else {
                                                                            s.disp.stride
                                                                        }) as u32), ((dst_pitch) as u32)));
                                                                    so_log.set(so_log.get() + 1);
                                                                }
                                                            }
                                                            gpu_set_present_pitch(s, dst_pitch);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    b = b.wrapping_add(1);
                                }
                            }
                        }
                    } else if opcode == 0x9a as uint32_t && body_dw >= 3 as uint32_t {
                        let gmc_0: uint32_t = dw(d, (0 as ::core::ffi::c_int) as usize);
                        let rop3_0: uint32_t =
                            gmc_0 >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
                        let has_po: bool =
                            gmc_0 >> 1 as ::core::ffi::c_int & 1 as uint32_t != 0;
                        let bpp_0: uint32_t = gmc_dst_bpp(gmc_0);
                        let mut idx: uint32_t = 1 as uint32_t;
                        let mut offset: uint32_t = 0;
                        let mut pitch: uint32_t = 0;
                        if has_po {
                            let fresh34 = idx;
                            idx = idx.wrapping_add(1);
                            let po: uint32_t = dw(d, (fresh34) as usize);
                            offset = gpu_po_offset(s, po);
                            pitch = (po & 0x3fc00000 as ::core::ffi::c_int as uint32_t)
                                >> 16 as ::core::ffi::c_int;
                            if po >> 30 as ::core::ffi::c_int & 1 as uint32_t != 0 {
                                mc_register_tiled_surface(
                                    s,
                                    offset,
                                    pitch,
                                    c"PAINT_PO".as_ptr(),
                                );
                            }
                        } else {
                            let po_0: uint32_t = s.regs.default_pitch_offset;
                            offset = gpu_po_offset(s, po_0);
                            pitch = (po_0 & 0x3fc00000 as ::core::ffi::c_int as uint32_t)
                                >> 16 as ::core::ffi::c_int;
                        }
                        let mut pm_clip_x0: uint32_t = 0 as uint32_t;
                        let mut pm_clip_y0: uint32_t = 0 as uint32_t;
                        let mut pm_clip_x1: uint32_t = 0x3fff as uint32_t;
                        let mut pm_clip_y1: uint32_t = 0x3fff as uint32_t;
                        if gmc_0
                            & ((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as uint32_t
                            != 0
                        {
                            idx = idx.wrapping_add(1);
                        }
                        if gmc_0
                            & ((1 as ::core::ffi::c_int) << 3 as ::core::ffi::c_int) as uint32_t
                            != 0
                            && idx.wrapping_add(2 as uint32_t) <= body_dw
                        {
                            let fresh35 = idx;
                            idx = idx.wrapping_add(1);
                            let tl: uint32_t = dw(d, (fresh35) as usize);
                            let fresh36 = idx;
                            idx = idx.wrapping_add(1);
                            let br: uint32_t = dw(d, (fresh36) as usize);
                            pm_clip_x0 = tl & 0x3fff as uint32_t;
                            pm_clip_y0 = tl >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                            pm_clip_x1 = br & 0x3fff as uint32_t;
                            pm_clip_y1 = br >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                        }
                        let brush_type: uint32_t =
                            gmc_0 >> 4 as ::core::ffi::c_int & 0xf as uint32_t;
                        let mut color: uint32_t = 0;
                        if brush_type == 0xd as uint32_t {
                            let fresh37 = idx;
                            idx = idx.wrapping_add(1);
                            color = dw(d, (fresh37) as usize);
                        } else {
                            color = s.regs.dp_brush_frgd_clr;
                        }
                        if pitch == 0 as uint32_t {
                            i = i.wrapping_add(body_adv);
                            continue;
                        } else {
                            let vram_0 = crate::src::qemu::Vram::of(s);
                            let num_fills: uint32_t =
                                body_dw.wrapping_sub(idx).wrapping_div(2 as uint32_t);
                            g_blit_stats_mut().paint_count = g_blit_stats_mut().paint_count.wrapping_add(1);
                            static paint_logged: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0 as uint64_t);
                            if paint_logged.get() < 500 as uint64_t {
                                let mut dst_region_0: *const ::core::ffi::c_char =
                                    c"offscreen".as_ptr();
                                if offset == 0 as uint32_t
                                    && (pitch == 4096 as uint32_t || pitch == 3200 as uint32_t)
                                {
                                    dst_region_0 =
                                        c"FRAMEBUFFER".as_ptr();
                                } else if offset >= 0x200000 as ::core::ffi::c_int as uint32_t {
                                    dst_region_0 =
                                        c"backbuf".as_ptr();
                                }
                                crate::src::logs::blit_path("PAINT", format_args!("rop=0x{:02x} color=0x{:08x}  off=0x{:06x} pitch={} has_po={}  fills={}  dst={}", ((rop3_0) as u32), ((color) as u32), ((offset) as u32), ((pitch) as u32), ((has_po as ::core::ffi::c_int) as i32), ((num_fills) as u32), crate::src::qemu::cstr(dst_region_0)));
                                paint_logged.set(paint_logged.get().wrapping_add(1));
                            }
                            static paint_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
                            if paint_log.get() < 50 as ::core::ffi::c_int {
                                {
 let mut out = String::new();
                                    let _ = std::fmt::Write::write_fmt(&mut out, format_args!("\n=== PAINT[{}] gmc=0x{:08x} rop=0x{:02x} has_po={} brush=0x{:x} off=0x{:x} pitch={} color=0x{:08x} fills={} ===\n", ((paint_log.get()) as i32), ((gmc_0) as u32), ((rop3_0) as u32), ((has_po as ::core::ffi::c_int) as i32), ((brush_type) as u32), ((offset) as u32), ((pitch) as u32), ((color) as u32), ((num_fills) as u32)));
                                    let mut ff: uint32_t = 0 as uint32_t;
                                    while ff < num_fills && ff < 5 as uint32_t {
                                        let yx: uint32_t = dw(d, (idx.wrapping_add(ff.wrapping_mul(2 as uint32_t))
                                                .wrapping_add(0 as uint32_t)) as usize);
                                        let wh2: uint32_t = dw(d, (idx.wrapping_add(ff.wrapping_mul(2 as uint32_t))
                                                .wrapping_add(1 as uint32_t)) as usize);
                                        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("  fill{}: YX=0x{:08x} WH=0x{:08x} X_Y=({},{}) {}x{}\n", ((ff) as u32), ((yx) as u32), ((wh2) as u32), ((yx >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((yx & 0xffff as uint32_t) as u32), ((wh2 >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t) as u32), ((wh2 & 0x3fff as uint32_t) as u32)));
                                        ff = ff.wrapping_add(1);
                                    }
                                    crate::src::qemu::append_file("/tmp/gpu_blit_diag.txt", &out);
                                }
                                paint_log.set(paint_log.get() + 1);
                            }
                            let mut f_0: uint32_t = 0 as uint32_t;
                            while f_0 < num_fills {
                                let dst_yx: uint32_t = dw(d, (idx.wrapping_add(f_0.wrapping_mul(2 as uint32_t))
                                        .wrapping_add(0 as uint32_t)) as usize);
                                let dwh_0: uint32_t = dw(d, (idx.wrapping_add(f_0.wrapping_mul(2 as uint32_t))
                                        .wrapping_add(1 as uint32_t)) as usize);
                                let mut dst_x_0: uint32_t =
                                    dst_yx >> 16 as ::core::ffi::c_int & 0xffff as uint32_t;
                                let mut dst_y_0: uint32_t = dst_yx & 0xffff as uint32_t;
                                let mut blit_w_0: uint32_t =
                                    dwh_0 >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                                let mut blit_h_0: uint32_t = dwh_0 & 0x3fff as uint32_t;
                                if !(blit_w_0 == 0 as uint32_t || blit_h_0 == 0 as uint32_t) {
                                    let cx0: uint32_t = {
                                        let mut _a113: uint32_t = dst_x_0 as uint32_t;
                                        let mut _b114: uint32_t = pm_clip_x0 as uint32_t;
                                        if _a113 > _b114 {
                                            _a113 as uint32_t
                                        } else {
                                            _b114 as uint32_t
                                        }
                                    };
                                    let cy0: uint32_t = {
                                        let mut _a115: uint32_t = dst_y_0 as uint32_t;
                                        let mut _b116: uint32_t = pm_clip_y0 as uint32_t;
                                        if _a115 > _b116 {
                                            _a115 as uint32_t
                                        } else {
                                            _b116 as uint32_t
                                        }
                                    };
                                    let cx1: uint32_t = {
                                        let mut _a119: uint32_t = (dst_x_0 as uint32_t)
                                            .wrapping_add(blit_w_0 as uint32_t);
                                        let mut _b120: uint32_t = {
                                            let mut _a117: uint32_t = pm_clip_x1 as uint32_t;
                                            let mut _b118: uint32_t =
                                                (pitch as uint32_t).wrapping_div(bpp_0 as uint32_t);
                                            if _a117 < _b118 {
                                                _a117 as uint32_t
                                            } else {
                                                _b118 as uint32_t
                                            }
                                        };
                                        if _a119 < _b120 {
                                            _a119 as uint32_t
                                        } else {
                                            _b120 as uint32_t
                                        }
                                    };
                                    let cy1: uint32_t = {
                                        let mut _a121: uint32_t = (dst_y_0 as uint32_t)
                                            .wrapping_add(blit_h_0 as uint32_t);
                                        let mut _b122: uint32_t = pm_clip_y1 as uint32_t;
                                        if _a121 < _b122 {
                                            _a121 as uint32_t
                                        } else {
                                            _b122 as uint32_t
                                        }
                                    };
                                    if !(cx0 >= cx1 || cy0 >= cy1) {
                                        dst_x_0 = cx0;
                                        dst_y_0 = cy0;
                                        blit_w_0 = cx1.wrapping_sub(cx0);
                                        blit_h_0 = cy1.wrapping_sub(cy0);
                                        if !((offset as uint64_t).wrapping_add(
                                            (dst_y_0.wrapping_add(blit_h_0) as uint64_t)
                                                .wrapping_mul(pitch as uint64_t),
                                        ) > s.vram_size)
                                        {
                                            gpu_vram_access(
                                                s,
                                                (offset as uint64_t).wrapping_add(
                                                    (dst_y_0 as uint64_t)
                                                        .wrapping_mul(pitch as uint64_t),
                                                ),
                                                (offset as uint64_t).wrapping_add(
                                                    (dst_y_0 as uint64_t)
                                                        .wrapping_add(blit_h_0 as uint64_t)
                                                        .wrapping_mul(pitch as uint64_t),
                                                ),
                                                1 as ::core::ffi::c_int != 0,
                                            );
                                            crate::src::logs::seq(format_args!("FILL rop={:02x} dst={:06x}/{} ({},{}) {}x{} color={:08x}", ((rop3_0) as u32), ((offset) as u32), ((pitch) as u32), ((dst_x_0) as u32), ((dst_y_0) as u32), ((blit_w_0) as u32), ((blit_h_0) as u32), ((color) as u32)));
                                            if rop3_0 == 0xf0 as uint32_t
                                                || rop3_0 == 0xcc as uint32_t
                                            {
                                                if offset
                                                    <= 0x10000 as ::core::ffi::c_int as uint32_t
                                                {
                                                    fb_write_watch(
                                                        c"paint_fill".as_ptr(),
                                                        c"solid".as_ptr(),
                                                        0 as uint32_t,
                                                        offset,
                                                        dst_x_0,
                                                        dst_y_0,
                                                        blit_w_0,
                                                        blit_h_0,
                                                        0 as ::core::ffi::c_int != 0,
                                                        color != 0 as uint32_t,
                                                    );
                                                }
                                                let mut row_1: uint32_t = 0 as uint32_t;
                                                while row_1 < blit_h_0 {
                                                    let mut col_1: uint32_t = 0 as uint32_t;
                                                    while col_1 < blit_w_0 {
                                                        let d_linear_1: uint64_t = (offset
                                                            as uint64_t)
                                                            .wrapping_add(
                                                                (dst_y_0.wrapping_add(row_1)
                                                                    as uint64_t)
                                                                    .wrapping_mul(
                                                                        pitch as uint64_t,
                                                                    ),
                                                            )
                                                            .wrapping_add(
                                                                (dst_x_0.wrapping_add(col_1)
                                                                    as uint64_t)
                                                                    .wrapping_mul(
                                                                        bpp_0 as uint64_t,
                                                                    ),
                                                            );
                                                        crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_process_pm4", 7182);
                                                        fill_vram_px(
                                                            s, vram_0, d_linear_1, color, bpp_0,
                                                        );
                                                        col_1 = col_1.wrapping_add(1);
                                                    }
                                                    row_1 = row_1.wrapping_add(1);
                                                }
                                                let dirty_start_0: uint64_t = (offset
                                                    as uint64_t)
                                                    .wrapping_add(
                                                        (dst_y_0 as uint64_t)
                                                            .wrapping_mul(pitch as uint64_t),
                                                    )
                                                    .wrapping_add(
                                                        (dst_x_0 as uint64_t)
                                                            .wrapping_mul(bpp_0 as uint64_t),
                                                    );
                                                let dirty_len_0: uint64_t = if blit_h_0
                                                    > 1 as uint32_t
                                                {
                                                    (blit_h_0.wrapping_sub(1 as uint32_t)
                                                        as uint64_t)
                                                        .wrapping_mul(pitch as uint64_t)
                                                        .wrapping_add(
                                                            (blit_w_0 as uint64_t)
                                                                .wrapping_mul(bpp_0 as uint64_t),
                                                        )
                                                } else {
                                                    (blit_w_0 as uint64_t)
                                                        .wrapping_mul(bpp_0 as uint64_t)
                                                };
                                                if dirty_start_0.wrapping_add(dirty_len_0)
                                                    <= s.vram_size
                                                {
                                                    gpu_vram_dirty(s, dirty_start_0, dirty_len_0);
                                                }
                                                r200_fill_notify(
                                                    s, offset, pitch, dst_x_0, dst_y_0, blit_w_0,
                                                    blit_h_0, bpp_0, color,
                                                );
                                            }
                                        }
                                    }
                                }
                                f_0 = f_0.wrapping_add(1);
                            }
                        }
                    } else if !(opcode == 0x94 as uint32_t
                        && body_dw >= 9 as uint32_t
                        && ppc_mac_gpu_hostdata_mono(s, d, body_dw) as ::core::ffi::c_int != 0)
                    {
                        if opcode == 0x9c as uint32_t && body_dw >= 4 as uint32_t {
                            let gmc_1: uint32_t = dw(d, (0 as ::core::ffi::c_int) as usize);
                            let po_1: uint32_t = dw(d, (1 as ::core::ffi::c_int) as usize);
                            let dst_offset_0: uint32_t = gpu_po_offset(s, po_1);
                            let dst_pitch_0: uint32_t = (po_1
                                & 0x3fc00000 as ::core::ffi::c_int as uint32_t)
                                >> 16 as ::core::ffi::c_int;
                            if po_1 >> 30 as ::core::ffi::c_int & 1 as uint32_t != 0 {
                                mc_register_tiled_surface(
                                    s,
                                    dst_offset_0,
                                    dst_pitch_0,
                                    c"HD_PO".as_ptr(),
                                );
                            }
                            let dst_yx_0: uint32_t =
                                dw(d, (2 as ::core::ffi::c_int) as usize);
                            let dst_x_1: uint32_t = dst_yx_0 & 0xffff as uint32_t;
                            let dst_y_1: uint32_t =
                                dst_yx_0 >> 16 as ::core::ffi::c_int & 0xffff as uint32_t;
                            let dwh_1: uint32_t = dw(d, (3 as ::core::ffi::c_int) as usize);
                            let blit_w_1: uint32_t = dwh_1 & 0x3fff as uint32_t;
                            let blit_h_1: uint32_t =
                                dwh_1 >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t;
                            let ymax: uint32_t = {
                                let mut _a123: uint32_t = dst_yx_0 as uint32_t & 0xffff as uint32_t;
                                let mut _b124: uint32_t = dst_yx_0 as uint32_t
                                    >> 16 as ::core::ffi::c_int
                                    & 0xffff as uint32_t;
                                if _a123 > _b124 {
                                    _a123 as uint32_t
                                } else {
                                    _b124 as uint32_t
                                }
                            };
                            let hmax: uint32_t = {
                                let mut _a125: uint32_t = dwh_1 as uint32_t & 0x3fff as uint32_t;
                                let mut _b126: uint32_t = dwh_1 as uint32_t
                                    >> 16 as ::core::ffi::c_int
                                    & 0x3fff as uint32_t;
                                if _a125 > _b126 {
                                    _a125 as uint32_t
                                } else {
                                    _b126 as uint32_t
                                }
                            };
                            gpu_vram_access(
                                s,
                                (dst_offset_0 as uint64_t).wrapping_add(
                                    (0 as ::core::ffi::c_int as uint64_t)
                                        .wrapping_mul(dst_pitch_0 as uint64_t),
                                ),
                                (dst_offset_0 as uint64_t).wrapping_add(
                                    (0 as ::core::ffi::c_int as uint64_t)
                                        .wrapping_add(ymax.wrapping_add(hmax) as uint64_t)
                                        .wrapping_mul(dst_pitch_0 as uint64_t),
                                ),
                                1 as ::core::ffi::c_int != 0,
                            );
                            crate::src::logs::seq(format_args!("HOST dst={:06x}/{} raw_yx={:08x} raw_wh={:08x} asYX_HW=({},{}) {}x{}  asXY_WH=({},{}) {}x{} data={} dw", ((dst_offset_0) as u32), ((dst_pitch_0) as u32), ((dst_yx_0) as u32), ((dwh_1) as u32), ((dst_yx_0 & 0xffff as uint32_t) as u32), ((dst_yx_0 >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((dwh_1 & 0x3fff as uint32_t) as u32), ((dwh_1 >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t) as u32), ((dst_yx_0 >> 16 as ::core::ffi::c_int & 0xffff as uint32_t) as u32), ((dst_yx_0 & 0xffff as uint32_t) as u32), ((dwh_1 >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t) as u32), ((dwh_1 & 0x3fff as uint32_t) as u32), ((body_dw.wrapping_sub(4 as uint32_t)) as u32)));
                            g_blit_stats_mut().hostdata_count =
                                g_blit_stats_mut().hostdata_count.wrapping_add(1);
                            static hd_logged: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0 as uint64_t);
                            if hd_logged.get() < 500 as uint64_t {
                                let mut dst_region_1: *const ::core::ffi::c_char =
                                    c"offscreen".as_ptr();
                                if dst_offset_0 == 0 as uint32_t
                                    && (dst_pitch_0 == 4096 as uint32_t
                                        || dst_pitch_0 == 3200 as uint32_t)
                                {
                                    dst_region_1 =
                                        c"FRAMEBUFFER".as_ptr();
                                } else if dst_offset_0 >= 0x200000 as ::core::ffi::c_int as uint32_t
                                {
                                    dst_region_1 =
                                        c"backbuf".as_ptr();
                                }
                                crate::src::logs::blit_path("HD", format_args!("off=0x{:06x} pitch={}  xy=({},{}) {}x{}  data={} dw  dst={}", ((dst_offset_0) as u32), ((dst_pitch_0) as u32), ((dst_x_1) as u32), ((dst_y_1) as u32), ((blit_w_1) as u32), ((blit_h_1) as u32), ((body_dw.wrapping_sub(4 as uint32_t)) as u32), crate::src::qemu::cstr(dst_region_1)));
                                hd_logged.set(hd_logged.get().wrapping_add(1));
                            }
                            if blit_w_1 > 0 as uint32_t
                                && blit_h_1 > 0 as uint32_t
                                && dst_pitch_0 > 0 as uint32_t
                            {
                                let vram_1 = crate::src::qemu::Vram::of(s);
                                let bpp_1: uint32_t = gmc_dst_bpp(gmc_1);
                                let mut pix_idx: uint32_t = 0 as uint32_t;
                                let mut data_dw: uint32_t = body_dw.wrapping_sub(4 as uint32_t);
                                if bpp_1 < 4 as uint32_t {
                                    let mut cx: uint32_t = 0 as uint32_t;
                                    let mut cy: uint32_t = 0 as uint32_t;
                                    let mut dw: uint32_t = 0 as uint32_t;
                                    while dw < data_dw {
                                        if !host_data_put_narrow(
                                            s,
                                            vram_1,
                                            super::dw(d, ((4 as uint32_t).wrapping_add(dw)) as usize),
                                            bpp_1,
                                            dst_offset_0 as uint64_t,
                                            dst_pitch_0,
                                            dst_x_1,
                                            dst_y_1,
                                            blit_w_1,
                                            blit_h_1,
                                            &mut cx,
                                            &mut cy,
                                        ) {
                                            break;
                                        }
                                        dw = dw.wrapping_add(1);
                                    }
                                    pix_idx = if cy >= blit_h_1 {
                                        blit_w_1.wrapping_mul(blit_h_1)
                                    } else {
                                        cy.wrapping_mul(blit_w_1).wrapping_add(cx)
                                    };
                                    data_dw = 0 as uint32_t;
                                }
                                let mut dw_0: uint32_t = 0 as uint32_t;
                                while dw_0 < data_dw {
                                    let cx_0: uint32_t = pix_idx.wrapping_rem(blit_w_1);
                                    let cy_0: uint32_t = pix_idx.wrapping_div(blit_w_1);
                                    if cy_0 >= blit_h_1 {
                                        break;
                                    }
                                    let d_linear_2: uint64_t = (dst_offset_0 as uint64_t)
                                        .wrapping_add(
                                            (dst_y_1.wrapping_add(cy_0) as uint64_t)
                                                .wrapping_mul(dst_pitch_0 as uint64_t),
                                        )
                                        .wrapping_add(
                                            (dst_x_1.wrapping_add(cx_0) as uint64_t)
                                                .wrapping_mul(bpp_1 as uint64_t),
                                        );
                                    crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_process_pm4", 7299);
                                    mc_vram_write32(
                                        s,
                                        vram_1,
                                        d_linear_2,
                                        dw(d, ((4 as uint32_t).wrapping_add(dw_0)) as usize),
                                    );
                                    pix_idx = pix_idx.wrapping_add(1);
                                    dw_0 = dw_0.wrapping_add(1);
                                }
                                if pix_idx < blit_w_1.wrapping_mul(blit_h_1) {
                                    s.host_data_offset = dst_offset_0;
                                    s.host_data_pitch = dst_pitch_0;
                                    s.host_data_macro_tile = 0 as ::core::ffi::c_int != 0;
                                    s.host_data_dst_x = dst_x_1;
                                    s.host_data_dst_y = dst_y_1;
                                    s.host_data_w = blit_w_1;
                                    s.host_data_h = blit_h_1;
                                    s.host_data_cur_x = pix_idx.wrapping_rem(blit_w_1);
                                    s.host_data_cur_y = pix_idx.wrapping_div(blit_w_1);
                                    s.host_data_bpp = bpp_1;
                                    s.host_data_active = 1 as ::core::ffi::c_int != 0;
                                } else {
                                    let dirty_start_1: uint64_t = (dst_offset_0 as uint64_t)
                                        .wrapping_add(
                                            (dst_y_1 as uint64_t)
                                                .wrapping_mul(dst_pitch_0 as uint64_t),
                                        )
                                        .wrapping_add(
                                            (dst_x_1 as uint64_t).wrapping_mul(bpp_1 as uint64_t),
                                        );
                                    let dirty_len_1: uint64_t = if blit_h_1 > 1 as uint32_t {
                                        (blit_h_1.wrapping_sub(1 as uint32_t) as uint64_t)
                                            .wrapping_mul(dst_pitch_0 as uint64_t)
                                            .wrapping_add(
                                                (blit_w_1 as uint64_t)
                                                    .wrapping_mul(bpp_1 as uint64_t),
                                            )
                                    } else {
                                        (blit_w_1 as uint64_t).wrapping_mul(bpp_1 as uint64_t)
                                    };
                                    if dirty_start_1.wrapping_add(dirty_len_1) <= s.vram_size {
                                        gpu_vram_dirty(s, dirty_start_1, dirty_len_1);
                                    }
                                    s.display_invalid = 1 as ::core::ffi::c_int != 0;
                                }
                            }
                        } else {
                            crate::src::qemu::trace_3d_cmd(opcode, body_dw);
                            let log_count: uint32_t = if body_dw < 8 as uint32_t {
                                body_dw
                            } else {
                                8 as uint32_t
                            };
                            let mut l: uint32_t = 0 as uint32_t;
                            while l < log_count {
                                crate::src::logs::debug(format_args!("  3D_CMD op=0x{:02x} [{}]=0x{:08x}", ((opcode) as u32), ((l) as u32), ((dw(d, (l) as usize)) as u32)));
                                l = l.wrapping_add(1);
                            }
                        }
                    }
                }
                let t3_vw_after: uint32_t = vram_watch_word(s);
                if s.vram_watch_active && t3_vw_after != t3_vw_before {
                    crate::src::logs::err(format_args!("[VRAM_CLR_FOUND] Type3 op=0x{:02x} body={} CHANGED vram[0x353000] from 0x{:08x} to 0x{:08x}\n", ((opcode) as u32), ((body_dw) as u32), ((t3_vw_before) as u32), ((t3_vw_after) as u32)));
                }
                i = i.wrapping_add(body_adv);
            } else {
                let pkt_count_1: uint32_t = (hdr >> 16 as ::core::ffi::c_int
                    & 0x3fff as uint32_t)
                    .wrapping_add(1 as uint32_t);
                i = i.wrapping_add(pkt_count_1);
            }
        }
    }
    g_in_pm4.set(g_in_pm4.get() - 1);
}
pub fn ppc_mac_gpu_execute_ib(
    s: &mut PPCMacGPUState,
    ib_base: uint32_t,
    ib_size_dw: uint32_t,
) {
    if r300_ringdump() {
        crate::src::logs::err(format_args!("IB base={:08x} size={}\n", ((ib_base) as u32), ((ib_size_dw) as u32)));
    }
    if ib_size_dw == 0 as uint32_t || ib_size_dw > 0x100000 as ::core::ffi::c_int as uint32_t {
        s.regs.stall_ib_lost = s.regs.stall_ib_lost.wrapping_add(1);
        crate::src::logs::qemu(format_args!("ppc-mac-gpu: IB lost: base={:08x} size={}\n", ((ib_base) as u32), ((ib_size_dw) as u32)));
        return;
    }
    s.kick_epoch = s.kick_epoch.wrapping_add(1);
    crate::src::logs::debug(format_args!("IB_EXEC base=0x{:x} size={} dwords", ((ib_base) as u32), ((ib_size_dw) as u32)));
    let mut ib_data: Option<Vec<uint32_t>> = ppc_mac_gpu_read_ib_via_gart(s, ib_base, ib_size_dw);
    if ib_data.is_none() {
        let ib_end: uint64_t = (ib_base as uint64_t)
            .wrapping_add((ib_size_dw as uint64_t).wrapping_mul(4 as uint64_t));
        if ib_end > s.vram_size {
            crate::src::logs::debug(format_args!("IB_EXEC: neither GART nor VRAM (base=0x{:x})", ((ib_base) as u32)));
            crate::src::logs::qemu(format_args!("ppc-mac-gpu: IB lost (not in GART or VRAM): base={:08x} size={}\n", ((ib_base) as u32), ((ib_size_dw) as u32)));
            s.regs.stall_ib_lost = s.regs.stall_ib_lost.wrapping_add(1);
            return;
        }
        let vram = crate::src::qemu::Vram::of(s);
        let mut bytes = vec![0u8; (ib_size_dw as usize) * 4];
        if !vram.read_bytes(ib_base as u64, &mut bytes) {
            s.regs.stall_ib_lost = s.regs.stall_ib_lost.wrapping_add(1);
            return;
        }
        ib_data = Some(
            bytes
                .chunks_exact(4)
                .map(|c| u32::from_be_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        );
    }
    s.regs.stall_ib_done = s.regs.stall_ib_done.wrapping_add(1);
    s.regs.stall_ib_dwords = s
        .regs
        .stall_ib_dwords
        .wrapping_add(ib_size_dw as uint64_t);
    if let Some(ib) = ib_data {
        ppc_mac_gpu_process_pm4(s, &ib);
    }
}
pub fn ppc_mac_gpu_process_ring_buffer(
    s: &mut PPCMacGPUState,
    old_rptr: uint32_t,
    new_wptr: uint32_t,
) {
    let rb_bufsz: uint32_t = s.regs.cp_rb_cntl & 0x3f as uint32_t;
    if rb_bufsz == 0 as uint32_t || rb_bufsz > 25 as uint32_t {
        return;
    }
    let ring_size_dw: uint32_t = (2 as uint32_t) << rb_bufsz;
    let ring_mask: uint32_t = ring_size_dw.wrapping_sub(1 as uint32_t);
    let count: uint32_t = new_wptr.wrapping_sub(old_rptr) & ring_mask;
    if count == 0 as uint32_t || count > ring_size_dw {
        return;
    }
    if count > 65536 as ::core::ffi::c_int as uint32_t {
        crate::src::logs::debug(format_args!("RING: skipping oversized batch ({} dw)", ((count) as u32)));
        return;
    }
    // `count` is bounded above (65536 dwords), so this allocation is small.
    let mut rb_data: Vec<uint32_t> = Vec::with_capacity(count as usize);
    for i in 0..count {
        let offset_dw: uint32_t = old_rptr.wrapping_add(i) & ring_mask;
        let gpu_addr: uint32_t = s
            .regs
            .cp_rb_base
            .wrapping_add(offset_dw.wrapping_mul(4 as uint32_t));
        let mut phys: hwaddr = 0;
        if !ppc_mac_gpu_gart_translate(s, gpu_addr, &mut phys)
            && !ppc_mac_gpu_agp_translate(s, gpu_addr, &mut phys)
        {
            crate::src::logs::debug(format_args!("RING: GART translate failed at gpu_addr=0x{:x} (rb_base=0x{:x} offset_dw={})", ((gpu_addr) as u32), ((s.regs.cp_rb_base) as u32), ((offset_dw) as u32)));
            return;
        }
        let mut raw: uint32_t = 0 as uint32_t;
        let r: MemTxResult = crate::src::qemu::guest_read32(phys as u64, &mut raw);
        if r != 0 as MemTxResult {
            crate::src::logs::debug(format_args!("RING: read failed at phys=0x{:x}", ((phys) as u64)));
            return;
        }
        rb_data.push(gpu_guest_dw(s, raw));
    }
    s.kick_epoch = s.kick_epoch.wrapping_add(1);
    crate::src::logs::debug(format_args!("RING: processing {} dwords (rptr={} wptr={} rb_bufsz={})", ((count) as u32), ((old_rptr) as u32), ((new_wptr) as u32), ((rb_bufsz) as u32)));
    ppc_mac_gpu_process_pm4(s, &rb_data);
}
