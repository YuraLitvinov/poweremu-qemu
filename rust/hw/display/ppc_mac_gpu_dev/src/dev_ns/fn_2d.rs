use super::*;
pub fn ppc_mac_gpu_2d_op(s: &mut PPCMacGPUFields, op: & Gpu2DOp) {
    let gmc: uint32_t = op.gmc;
    let doff: uint32_t = op.dst_off;
    let dpitch: uint32_t = op.dst_pitch;
    let soff: uint32_t = op.src_off;
    let spitch: uint32_t = op.src_pitch;
    let mut w: uint32_t = op.w;
    let mut h: uint32_t = op.h;
    let bpp: uint32_t = gpu_gmc_bpp(gmc);
    let rop: uint32_t = gmc >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
    let src_src: uint32_t = gmc >> 24 as ::core::ffi::c_int & 7 as uint32_t;
    let brush: uint32_t = gmc >> 4 as ::core::ffi::c_int & 0xf as uint32_t;
    let mut xl2r: bool = op.xl2r;
    let mut yt2b: bool = op.yt2b;
    let mut clip: bool = op.clip;
    let lsb: bool =
        gmc as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << 14 as ::core::ffi::c_int != 0;
    let mut dx: ::core::ffi::c_int = op.dx;
    let mut dy: ::core::ffi::c_int = op.dy;
    let mut sx: ::core::ffi::c_int = op.sx;
    let mut sy: ::core::ffi::c_int = op.sy;
    let mut need_src: bool =
        (rop ^ rop >> 2 as ::core::ffi::c_int) & 0x33 as uint32_t != 0 as uint32_t;
    let mask: uint32_t = if bpp == 4 as uint32_t {
        0xffffffff as uint32_t
    } else {
        (if bpp == 2 as uint32_t {
            0xffff as ::core::ffi::c_int
        } else {
            0xff as ::core::ffi::c_int
        }) as uint32_t
    };
    let vram = crate::src::qemu::Vram::of(s);
    if r300_ringdump() {
        crate::src::logs::err(format_args!("2D rop={:02x} brush={} srcsrc={} dst={:08x}/{} ({},{}) src={:08x}/{} ({},{}) {}x{} bpp={}\n", ((rop) as u32), ((brush) as u32), ((src_src) as u32), ((doff) as u32), ((dpitch) as u32), ((dx) as i32), ((dy) as i32), ((soff) as u32), ((spitch) as u32), ((sx) as i32), ((sy) as i32), ((w) as u32), ((h) as u32), ((bpp) as u32)));
    }
    if !crate::src::qemu::getenv(c"PPCGPU_LE2D_LOG").is_null() {
        static n: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
        let fresh43 = n.get();
        n.set(n.get() + 1);
        if fresh43 < 4000 as ::core::ffi::c_int {
            crate::src::logs::err(format_args!("LE2D rop={:02x} brush={} srcsrc={} dst={:08x}/{} ({},{}) src={:08x}/{} ({},{}) {}x{} bpp={}{}{}\n", ((rop) as u32), ((brush) as u32), ((src_src) as u32), ((doff) as u32), ((dpitch) as u32), ((dx) as i32), ((dy) as i32), ((soff) as u32), ((spitch) as u32), ((sx) as i32), ((sy) as i32), ((w) as u32), ((h) as u32), ((bpp) as u32), crate::src::qemu::cstr(if (doff as uint64_t)
                    .wrapping_add(
                        (dpitch as uint64_t)
                            .wrapping_mul(
                                (if dy > 0 as ::core::ffi::c_int {
                                    dy
                                } else {
                                    0 as ::core::ffi::c_int
                                }) as uint64_t,
                            ),
                    ) >= s.vram_size
                {
                    c" DST-OUTSIDE-VRAM".as_ptr()
                } else {
                    c"".as_ptr()
                }), crate::src::qemu::cstr(if need_src as ::core::ffi::c_int != 0
                    && (soff as uint64_t)
                        .wrapping_add(
                            (spitch as uint64_t)
                                .wrapping_mul(
                                    (if sy > 0 as ::core::ffi::c_int {
                                        sy
                                    } else {
                                        0 as ::core::ffi::c_int
                                    }) as uint64_t,
                                ),
                        ) >= s.vram_size
                {
                    c" SRC-OUTSIDE-VRAM".as_ptr()
                } else {
                    c"".as_ptr()
                })));
        }
    }
    if w == 0 || h == 0 || dpitch == 0 {
        return;
    }
    if src_src == 3 as uint32_t {
        s.host_data_offset = doff;
        s.host_data_pitch = dpitch;
        s.host_data_macro_tile = 0 as ::core::ffi::c_int != 0;
        s.host_data_dst_x = dx as uint32_t;
        s.host_data_dst_y = dy as uint32_t;
        s.host_data_w = w;
        s.host_data_h = h;
        s.host_data_cur_x = 0 as uint32_t;
        s.host_data_cur_y = 0 as uint32_t;
        s.host_data_bpp = bpp;
        s.host_data_active = 1 as ::core::ffi::c_int != 0;
        return;
    }
    if need_src as ::core::ffi::c_int != 0 && src_src != 2 as uint32_t {
        need_src = 0 as ::core::ffi::c_int != 0;
    }
    if op.auto_dir as ::core::ffi::c_int != 0
        && need_src as ::core::ffi::c_int != 0
        && soff == doff
        && spitch == dpitch
    {
        if dy > sy {
            yt2b = 0 as ::core::ffi::c_int != 0;
            dy += h as ::core::ffi::c_int - 1 as ::core::ffi::c_int;
            sy += h as ::core::ffi::c_int - 1 as ::core::ffi::c_int;
        } else if dy == sy && dx > sx {
            xl2r = 0 as ::core::ffi::c_int != 0;
            dx += w as ::core::ffi::c_int - 1 as ::core::ffi::c_int;
            sx += w as ::core::ffi::c_int - 1 as ::core::ffi::c_int;
        }
    }
    if clip {
        let xmin: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
            dx
        } else {
            dx - w as ::core::ffi::c_int + 1 as ::core::ffi::c_int
        };
        let ymin: ::core::ffi::c_int = if yt2b as ::core::ffi::c_int != 0 {
            dy
        } else {
            dy - h as ::core::ffi::c_int + 1 as ::core::ffi::c_int
        };
        let x0: ::core::ffi::c_int = {
            let mut _a97: ::core::ffi::c_int = xmin as ::core::ffi::c_int;
            let mut _b98: ::core::ffi::c_int = op.clip_x0 as ::core::ffi::c_int;
            if _a97 > _b98 {
                _a97 as ::core::ffi::c_int
            } else {
                _b98 as ::core::ffi::c_int
            }
        };
        let x1: ::core::ffi::c_int = {
            let mut _a99: ::core::ffi::c_int = xmin as ::core::ffi::c_int + w as ::core::ffi::c_int;
            let mut _b100: ::core::ffi::c_int = op.clip_x1 as ::core::ffi::c_int;
            if _a99 < _b100 {
                _a99 as ::core::ffi::c_int
            } else {
                _b100 as ::core::ffi::c_int
            }
        };
        let y0: ::core::ffi::c_int = {
            let mut _a101: ::core::ffi::c_int = ymin as ::core::ffi::c_int;
            let mut _b102: ::core::ffi::c_int = op.clip_y0 as ::core::ffi::c_int;
            if _a101 > _b102 {
                _a101 as ::core::ffi::c_int
            } else {
                _b102 as ::core::ffi::c_int
            }
        };
        let y1: ::core::ffi::c_int = {
            let mut _a103: ::core::ffi::c_int =
                ymin as ::core::ffi::c_int + h as ::core::ffi::c_int;
            let mut _b104: ::core::ffi::c_int = op.clip_y1 as ::core::ffi::c_int;
            if _a103 < _b104 {
                _a103 as ::core::ffi::c_int
            } else {
                _b104 as ::core::ffi::c_int
            }
        };
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        if xl2r {
            sx += x0 - xmin;
            dx = x0;
        } else {
            sx -= xmin + w as ::core::ffi::c_int - x1;
            dx = x1 - 1 as ::core::ffi::c_int;
        }
        if yt2b {
            sy += y0 - ymin;
            dy = y0;
        } else {
            sy -= ymin + h as ::core::ffi::c_int - y1;
            dy = y1 - 1 as ::core::ffi::c_int;
        }
        w = (x1 - x0) as uint32_t;
        h = (y1 - y0) as uint32_t;
        clip = 0 as ::core::ffi::c_int != 0;
    }
    if rop == 0xcc as uint32_t
        && src_src == 2 as uint32_t
        && soff >= 0x100000 as ::core::ffi::c_int as uint32_t
    {
        blit_region_record(
            soff,
            spitch,
            sx as uint32_t,
            sy as uint32_t,
            dx as uint32_t,
            dy as uint32_t,
            w,
            h,
        );
    }
    if brush != 13 as uint32_t && brush != 15 as uint32_t && brush > 1 as uint32_t {
        static warned: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
        if !warned.get() {
            warned.set(1 as ::core::ffi::c_int != 0);
            crate::src::logs::err(format_args!("ppc-mac-gpu: 2D brush type {} not supported\n", ((brush) as u32)));
        }
        return;
    }
    let dgtt: bool = doff as uint64_t >= s.vram_size;
    let sgtt: bool = need_src as ::core::ffi::c_int != 0 && soff as uint64_t >= s.vram_size;
    if !dgtt {
        gpu_vram_access(
            s,
            (doff as uint64_t).wrapping_add(
                ((if dy < 0 as ::core::ffi::c_int {
                    0 as ::core::ffi::c_int
                } else {
                    {
                        let mut _a105: ::core::ffi::c_int = dy as ::core::ffi::c_int;
                        let mut _b106: ::core::ffi::c_int = dy as ::core::ffi::c_int
                            + h as ::core::ffi::c_int
                            - 1 as ::core::ffi::c_int;
                        if _a105 < _b106 {
                            _a105 as ::core::ffi::c_int
                        } else {
                            _b106 as ::core::ffi::c_int
                        }
                    }
                }) as uint64_t)
                    .wrapping_mul(dpitch as uint64_t),
            ),
            (doff as uint64_t).wrapping_add(
                ((if dy < 0 as ::core::ffi::c_int {
                    0 as ::core::ffi::c_int
                } else {
                    {
                        let mut _a105: ::core::ffi::c_int = dy as ::core::ffi::c_int;
                        let mut _b106: ::core::ffi::c_int = dy as ::core::ffi::c_int
                            + h as ::core::ffi::c_int
                            - 1 as ::core::ffi::c_int;
                        if _a105 < _b106 {
                            _a105 as ::core::ffi::c_int
                        } else {
                            _b106 as ::core::ffi::c_int
                        }
                    }
                }) as uint64_t)
                    .wrapping_add(h as uint64_t)
                    .wrapping_mul(dpitch as uint64_t),
            ),
            1 as ::core::ffi::c_int != 0,
        );
    }
    if need_src as ::core::ffi::c_int != 0 && !sgtt {
        gpu_vram_access(
            s,
            (soff as uint64_t).wrapping_add(
                ((if sy < 0 as ::core::ffi::c_int {
                    0 as ::core::ffi::c_int
                } else {
                    {
                        let mut _a107: ::core::ffi::c_int = sy as ::core::ffi::c_int;
                        let mut _b108: ::core::ffi::c_int = sy as ::core::ffi::c_int
                            + h as ::core::ffi::c_int
                            - 1 as ::core::ffi::c_int;
                        if _a107 < _b108 {
                            _a107 as ::core::ffi::c_int
                        } else {
                            _b108 as ::core::ffi::c_int
                        }
                    }
                }) as uint64_t)
                    .wrapping_mul(spitch as uint64_t),
            ),
            (soff as uint64_t).wrapping_add(
                ((if sy < 0 as ::core::ffi::c_int {
                    0 as ::core::ffi::c_int
                } else {
                    {
                        let mut _a107: ::core::ffi::c_int = sy as ::core::ffi::c_int;
                        let mut _b108: ::core::ffi::c_int = sy as ::core::ffi::c_int
                            + h as ::core::ffi::c_int
                            - 1 as ::core::ffi::c_int;
                        if _a107 < _b108 {
                            _a107 as ::core::ffi::c_int
                        } else {
                            _b108 as ::core::ffi::c_int
                        }
                    }
                }) as uint64_t)
                    .wrapping_add(h as uint64_t)
                    .wrapping_mul(spitch as uint64_t),
            ),
            0 as ::core::ffi::c_int != 0,
        );
    }
    if dgtt as ::core::ffi::c_int != 0
        && need_src as ::core::ffi::c_int != 0
        && !sgtt
        && !crate::src::qemu::getenv(c"PPCGPU_TEX_LOG").is_null()
        && (w as uint64_t).wrapping_mul(h as uint64_t) >= 4096 as uint64_t
    {
        let mut nz: uint32_t = 0 as uint32_t;
        let mut tot: uint32_t = 0 as uint32_t;
        let mut j: uint32_t = 0 as uint32_t;
        while j < h
            && (soff as uint64_t).wrapping_add(
                ((sy as uint32_t).wrapping_add(j).wrapping_add(1 as uint32_t) as uint64_t)
                    .wrapping_mul(spitch as uint64_t),
            ) <= s.vram_size
        {
            let mut i: uint32_t = 0 as uint32_t;
            while i < w {
                let addr: uint64_t = (soff as uint64_t)
                    .wrapping_add(
                        ((sy as uint32_t).wrapping_add(j) as uint64_t).wrapping_mul(spitch as uint64_t),
                    )
                    .wrapping_add(((sx as uint32_t).wrapping_add(i) as uint64_t).wrapping_mul(4));
                // Out-of-range reads (guest-controlled) count as zero, never panic.
                nz = nz.wrapping_add((vram.get32(addr).unwrap_or(0) != 0) as uint32_t);
                tot = tot.wrapping_add(1);
                i = i.wrapping_add(1);
            }
            j = j.wrapping_add(1);
        }
        crate::src::logs::err(format_args!("DL2SYS src={:x}/{} {}x{} nonzero={}/{}\n", ((soff) as u32), ((spitch) as u32), ((w) as u32), ((h) as u32), ((nz) as u32), ((tot) as u32)));
    }
    let mut j_0: uint32_t = 0 as uint32_t;
    while j_0 < h {
        let y: ::core::ffi::c_int = if yt2b as ::core::ffi::c_int != 0 {
            dy + j_0 as ::core::ffi::c_int
        } else {
            dy - j_0 as ::core::ffi::c_int
        };
        let ys: ::core::ffi::c_int = if yt2b as ::core::ffi::c_int != 0 {
            sy + j_0 as ::core::ffi::c_int
        } else {
            sy - j_0 as ::core::ffi::c_int
        };
        if !(y < 0 as ::core::ffi::c_int
            || !dgtt
                && (doff as uint64_t)
                    .wrapping_add((y as uint64_t).wrapping_mul(dpitch as uint64_t))
                    >= s.vram_size)
        {
            if rop == 0xf0 as uint32_t && brush == 13 as uint32_t && !clip && !dgtt {
                let col: uint32_t = op.frgd & mask;
                let x0_0: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                    dx
                } else {
                    dx - w as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                };
                let mut i_0: uint32_t = 0 as uint32_t;
                while i_0 < w {
                    gpu_px_put(
                        s,
                        vram,
                        (doff as uint64_t)
                            .wrapping_add((y as uint64_t).wrapping_mul(dpitch as uint64_t))
                            .wrapping_add(
                                ((x0_0 as uint32_t).wrapping_add(i_0) as uint64_t)
                                    .wrapping_mul(bpp as uint64_t),
                            ),
                        col,
                        bpp,
                    );
                    i_0 = i_0.wrapping_add(1);
                }
            } else if rop == 0xcc as uint32_t
                && src_src == 2 as uint32_t
                && !clip
                && ys >= 0 as ::core::ffi::c_int
                && (dgtt as ::core::ffi::c_int != 0 || sgtt as ::core::ffi::c_int != 0)
            {
                let x0_1: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                    dx
                } else {
                    dx - w as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                };
                let xs0: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                    sx
                } else {
                    sx - w as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                };
                gpu_mc_copy(
                    s,
                    vram,
                    (doff as uint64_t)
                        .wrapping_add((y as uint64_t).wrapping_mul(dpitch as uint64_t))
                        .wrapping_add((x0_1 as uint64_t).wrapping_mul(bpp as uint64_t)),
                    (soff as uint64_t)
                        .wrapping_add((ys as uint64_t).wrapping_mul(spitch as uint64_t))
                        .wrapping_add((xs0 as uint64_t).wrapping_mul(bpp as uint64_t)),
                    w.wrapping_mul(bpp),
                );
            } else if rop == 0xcc as uint32_t
                && src_src == 2 as uint32_t
                && !clip
                && ys >= 0 as ::core::ffi::c_int
            {
                let x0_2: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                    dx
                } else {
                    dx - w as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                };
                let xs0_0: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                    sx
                } else {
                    sx - w as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                };
                let da: uint64_t = (doff as uint64_t)
                    .wrapping_add((y as uint64_t).wrapping_mul(dpitch as uint64_t))
                    .wrapping_add((x0_2 as uint64_t).wrapping_mul(bpp as uint64_t));
                let sa: uint64_t = (soff as uint64_t)
                    .wrapping_add((ys as uint64_t).wrapping_mul(spitch as uint64_t))
                    .wrapping_add((xs0_0 as uint64_t).wrapping_mul(bpp as uint64_t));
                if da.wrapping_add((w as uint64_t).wrapping_mul(bpp as uint64_t)) <= s.vram_size
                    && sa.wrapping_add((w as uint64_t).wrapping_mul(bpp as uint64_t))
                        <= s.vram_size
                {
                    vram.copy_within(((sa) as u64), ((da) as u64), ((w as size_t).wrapping_mul(bpp as size_t)) as usize);
                }
            } else {
                let mut current_block_102: u64;
                let mut i_1: uint32_t = 0 as uint32_t;
                while i_1 < w {
                    let x: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                        dx + i_1 as ::core::ffi::c_int
                    } else {
                        dx - i_1 as ::core::ffi::c_int
                    };
                    let xs: ::core::ffi::c_int = if xl2r as ::core::ffi::c_int != 0 {
                        sx + i_1 as ::core::ffi::c_int
                    } else {
                        sx - i_1 as ::core::ffi::c_int
                    };
                    let mut pat: uint32_t = 0 as uint32_t;
                    let mut src: uint32_t = 0 as uint32_t;
                    let mut bit: uint32_t = 1 as uint32_t;
                    if !(x < 0 as ::core::ffi::c_int) {
                        if brush == 13 as uint32_t {
                            pat = op.frgd;
                            current_block_102 = 17372050596571538954;
                        } else if brush <= 1 as uint32_t {
                            let bx: ::core::ffi::c_int = ((x as uint32_t)
                                .wrapping_add(op.brush_yx & 0xffff as uint32_t)
                                & 7 as uint32_t)
                                as ::core::ffi::c_int;
                            let by: ::core::ffi::c_int = ((y as uint32_t)
                                .wrapping_add(op.brush_yx >> 16 as ::core::ffi::c_int)
                                & 7 as uint32_t)
                                as ::core::ffi::c_int;
                            // Every Gpu2DOp is built with brush_data = &s.regs.brush_data, so
                            // read the words from the device state; by is 0..=7, index 0 or 1.
                            let word: uint32_t = s
                                .regs
                                .brush_data
                                .get((by >> 2 as ::core::ffi::c_int) as usize)
                                .copied()
                                .unwrap_or(0);
                            let row: uint32_t = word
                                >> (by & 3 as ::core::ffi::c_int) * 8 as ::core::ffi::c_int
                                & 0xff as uint32_t;
                            bit = if lsb as ::core::ffi::c_int != 0 {
                                row >> bx & 1 as uint32_t
                            } else {
                                row >> 7 as ::core::ffi::c_int - bx & 1 as uint32_t
                            };
                            if bit == 0 && brush == 1 as uint32_t {
                                current_block_102 = 17769492591016358583;
                            } else {
                                pat = if bit != 0 { op.frgd } else { op.bkgd };
                                current_block_102 = 17372050596571538954;
                            }
                        } else {
                            current_block_102 = 17372050596571538954;
                        }
                        match current_block_102 {
                            17769492591016358583 => {}
                            _ => {
                                if need_src {
                                    src = gpu_px_get(
                                        s,
                                        vram,
                                        (soff as uint64_t)
                                            .wrapping_add(
                                                (ys as uint64_t).wrapping_mul(spitch as uint64_t),
                                            )
                                            .wrapping_add(
                                                (xs as uint64_t).wrapping_mul(bpp as uint64_t),
                                            ),
                                        bpp,
                                    );
                                }
                                let da_0: uint64_t = (doff as uint64_t)
                                    .wrapping_add((y as uint64_t).wrapping_mul(dpitch as uint64_t))
                                    .wrapping_add((x as uint64_t).wrapping_mul(bpp as uint64_t));
                                let cur_px = gpu_px_get(s, vram, da_0, bpp);
                                gpu_px_put(s, vram, da_0, gpu_rop3(rop, pat, src, cur_px) & mask, bpp);
                            }
                        }
                    }
                    i_1 = i_1.wrapping_add(1);
                }
            }
        }
        j_0 = j_0.wrapping_add(1);
    }
    if !dgtt {
        let mut ylo: ::core::ffi::c_int = if yt2b as ::core::ffi::c_int != 0 {
            dy
        } else {
            dy - h as ::core::ffi::c_int + 1 as ::core::ffi::c_int
        };
        if ylo < 0 as ::core::ffi::c_int {
            ylo = 0 as ::core::ffi::c_int;
        }
        gpu_vram_dirty(
            s,
            (doff as uint64_t).wrapping_add((ylo as uint64_t).wrapping_mul(dpitch as uint64_t)),
            (h as uint64_t).wrapping_mul(dpitch as uint64_t),
        );
    }
}
pub fn ppc_mac_gpu_2d(
    s: &mut PPCMacGPUFields,
    w: uint32_t,
    h: uint32_t,
    legacy: bool,
) {
    let gmc: uint32_t = s.regs.dp_gui_master_cntl;
    let dpo: uint32_t =
        if gmc & ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t != 0 {
            s.regs.dst_pitch_offset
        } else {
            s.regs.default_pitch_offset
        };
    let spo: uint32_t =
        if gmc & ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint32_t != 0 {
            s.regs.src_pitch_offset
        } else {
            s.regs.default_pitch_offset
        };
    let mut op: Gpu2DOp = Gpu2DOp {
        gmc: gmc,
        dst_off: gpu_po_offset(s, dpo),
        dst_pitch: (dpo & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int,
        src_off: gpu_po_offset(s, spo),
        src_pitch: (spo & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int,
        dx: (s.regs.dst_y_x & 0xffff as uint32_t) as ::core::ffi::c_int,
        dy: (s.regs.dst_y_x >> 16 as ::core::ffi::c_int) as ::core::ffi::c_int,
        sx: (s.regs.src_y_x & 0xffff as uint32_t) as ::core::ffi::c_int,
        sy: (s.regs.src_y_x >> 16 as ::core::ffi::c_int) as ::core::ffi::c_int,
        w: w,
        h: h,
        xl2r: s.regs.dp_cntl & 1 as uint32_t != 0,
        yt2b: s.regs.dp_cntl & 2 as uint32_t != 0,
        auto_dir: false,
        clip: gmc & ((1 as ::core::ffi::c_int) << 3 as ::core::ffi::c_int) as uint32_t != 0,
        clip_x0: (s.regs.sc_top_left & 0xffff as uint32_t) as int16_t as ::core::ffi::c_int,
        clip_y0: (s.regs.sc_top_left >> 16 as ::core::ffi::c_int) as int16_t
            as ::core::ffi::c_int,
        clip_x1: (s.regs.sc_bottom_right & 0xffff as uint32_t) as int16_t as ::core::ffi::c_int,
        clip_y1: (s.regs.sc_bottom_right >> 16 as ::core::ffi::c_int) as int16_t
            as ::core::ffi::c_int,
        frgd: s.regs.dp_brush_frgd_clr,
        bkgd: s.regs.dp_brush_bkgd_clr,
        brush_yx: s.regs.brush_y_x,
        brush_data: &raw mut s.regs.brush_data as *mut uint32_t,
    };
    if legacy {
        op.dst_off = s.regs.dst_offset;
        op.dst_pitch = s.regs.dst_pitch;
        op.src_off = s.regs.src_offset;
        op.src_pitch = if s.regs.src_pitch != 0 {
            s.regs.src_pitch
        } else {
            op.dst_pitch
        };
    }
    ppc_mac_gpu_2d_op(s, &op);
}
pub fn ppc_mac_gpu_2d_line(s: &mut PPCMacGPUFields, end: uint32_t) {
    let gmc: uint32_t = s.regs.dp_gui_master_cntl;
    let po: uint32_t =
        if gmc & ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t != 0 {
            s.regs.dst_pitch_offset
        } else {
            s.regs.default_pitch_offset
        };
    let off: uint32_t = gpu_po_offset(s, po);
    let pitch: uint32_t =
        (po & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int;
    let bpp: uint32_t = gpu_gmc_bpp(gmc);
    let color: uint32_t = s.regs.dp_brush_frgd_clr;
    let mut x0: ::core::ffi::c_int =
        (s.regs.dst_line_start & 0xffff as uint32_t) as ::core::ffi::c_int;
    let mut y0: ::core::ffi::c_int =
        (s.regs.dst_line_start >> 16 as ::core::ffi::c_int) as ::core::ffi::c_int;
    let x1: ::core::ffi::c_int = (end & 0xffff as uint32_t) as ::core::ffi::c_int;
    let y1: ::core::ffi::c_int = (end >> 16 as ::core::ffi::c_int) as ::core::ffi::c_int;
    let dx: ::core::ffi::c_int = (x1 - x0).abs();
    let dy: ::core::ffi::c_int = -(y1 - y0).abs();
    let sx: ::core::ffi::c_int = if x0 < x1 {
        1 as ::core::ffi::c_int
    } else {
        -(1 as ::core::ffi::c_int)
    };
    let sy: ::core::ffi::c_int = if y0 < y1 {
        1 as ::core::ffi::c_int
    } else {
        -(1 as ::core::ffi::c_int)
    };
    let mut err: ::core::ffi::c_int = dx + dy;
    let clip: bool =
        gmc as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << 3 as ::core::ffi::c_int != 0;
    let vram = crate::src::qemu::Vram::of(s);
    let ymin: ::core::ffi::c_int = {
        let mut _a109: ::core::ffi::c_int = y0 as ::core::ffi::c_int;
        let mut _b110: ::core::ffi::c_int = y1 as ::core::ffi::c_int;
        if _a109 < _b110 {
            _a109 as ::core::ffi::c_int
        } else {
            _b110 as ::core::ffi::c_int
        }
    };
    let ymax: ::core::ffi::c_int = {
        let mut _a111: ::core::ffi::c_int = y0 as ::core::ffi::c_int;
        let mut _b112: ::core::ffi::c_int = y1 as ::core::ffi::c_int;
        if _a111 > _b112 {
            _a111 as ::core::ffi::c_int
        } else {
            _b112 as ::core::ffi::c_int
        }
    };
    if pitch == 0 {
        return;
    }
    gpu_vram_access(
        s,
        (off as uint64_t).wrapping_add((ymin as uint64_t).wrapping_mul(pitch as uint64_t)),
        (off as uint64_t).wrapping_add(
            (ymin as uint64_t)
                .wrapping_add((ymax - ymin + 1 as ::core::ffi::c_int) as uint64_t)
                .wrapping_mul(pitch as uint64_t),
        ),
        1 as ::core::ffi::c_int != 0,
    );
    loop {
        let in_0: bool = !clip
            || x0 >= (s.regs.sc_top_left & 0xffff as uint32_t) as ::core::ffi::c_int
                && y0 >= (s.regs.sc_top_left >> 16 as ::core::ffi::c_int) as ::core::ffi::c_int
                && x0 <= (s.regs.sc_bottom_right & 0xffff as uint32_t) as ::core::ffi::c_int
                && y0
                    <= (s.regs.sc_bottom_right >> 16 as ::core::ffi::c_int)
                        as ::core::ffi::c_int;
        if in_0 {
            crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_2d_line", 5178 as ::core::ffi::c_int);
            fill_vram_px(
                s,
                vram,
                (off as uint64_t)
                    .wrapping_add((y0 as uint64_t).wrapping_mul(pitch as uint64_t))
                    .wrapping_add((x0 as uint64_t).wrapping_mul(bpp as uint64_t)),
                color,
                bpp,
            );
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2: ::core::ffi::c_int = 2 as ::core::ffi::c_int * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    gpu_vram_dirty(
        s,
        (off as uint64_t).wrapping_add((ymin as uint64_t).wrapping_mul(pitch as uint64_t)),
        ((ymax - ymin + 1 as ::core::ffi::c_int) as uint64_t).wrapping_mul(pitch as uint64_t),
    );
}
