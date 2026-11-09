//! VRAM coherency and the image cache.
//!
//! Every VRAM page has a write generation.  CPU writes (the device's dirty
//! log) and GPU writes (a draw into an image) stamp the pages they touch;
//! an image whose range holds a newer stamp than its own is stale and is
//! copied from VRAM before use, after any image holding newer contents of
//! that range is written back.  What a batch renders goes back to VRAM at
//! the end of the batch, and only the rectangles draws touched (their
//! scissors): the CPU may write other parts of the same buffer meanwhile.

use ash::vk;

use crate::state::*;

fn no_cache() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        let on = std::env::var("PPCGPU_NO_CACHE").map_or(false, |v| v == "1");
        if on {
            eprintln!("ppc-mac-gpu vulkan: PPCGPU_NO_CACHE=1, re-reading every render target on every draw");
        }
        on
    })
}

impl Vk {
    pub fn pg(addr: u64) -> u64 {
        addr / PAGE
    }

    /// Is a GPU write of the page still pending (open or unfinished batch)?
    pub fn page_pending_write(&self, p: u64) -> bool {
        let s = self.page_wseq[p as usize];
        if s == 0 {
            return false;
        }
        match self.cur {
            Some(i) if self.batch[i].seq == s => true,
            _ => !self.seq_done(s),
        }
    }

    /// Take the CPU's writes from the device's dirty log (which, with this
    /// backend, does not include what draws render).  An image with
    /// rendering not yet in VRAM over a page the CPU wrote gets that
    /// rendering written back before it is reloaded (img_get), so both
    /// survive.
    pub fn harvest(&mut self) {
        let Some(f) = self.dirty_fn else {
            return;
        };
        for w in self.dirty_bm.iter_mut() {
            *w = 0;
        }
        unsafe { f(self.dirty_arg, self.dirty_bm.as_mut_ptr() as *mut _, self.npages) };
        let mut any = false;
        for wi in 0..self.dirty_bm.len() {
            let mut w = self.dirty_bm[wi];
            while w != 0 {
                let bit = w.trailing_zeros() as u64;
                w &= w - 1;
                let p = wi as u64 * 64 + bit;
                if p >= self.npages {
                    break;
                }
                if !any {
                    any = true;
                    self.gen += 1;
                }
                self.page_gen[p as usize] = self.gen;
            }
        }
    }

    pub fn range_gen(&self, lo: u64, hi: u64) -> u64 {
        let mut g = 0;
        let mut p = Self::pg(lo);
        while p <= Self::pg(hi - 1) && p < self.npages {
            g = g.max(self.page_gen[p as usize]);
            p += 1;
        }
        g
    }

    pub fn note_read(&mut self, lo: u64, hi: u64) {
        let s = {
            let i = self.batch();
            self.batch[i].seq
        };
        let mut p = Self::pg(lo);
        while p <= Self::pg(hi - 1) && p < self.npages {
            self.page_rseq[p as usize] = s;
            p += 1;
        }
    }

    pub fn range_pending(&self, lo: u64, hi: u64) -> bool {
        let mut p = Self::pg(lo);
        while p <= Self::pg(hi - 1) && p < self.npages {
            if self.page_pending_write(p) {
                return true;
            }
            p += 1;
        }
        false
    }

    fn fb_forget(&mut self, view: vk::ImageView) {
        for i in 0..MAX_FB {
            let hit = match &self.fbs[i] {
                Some(f) => f.view[..f.n].contains(&view),
                None => false,
            };
            if hit {
                let f = self.fbs[i].take().unwrap();
                self.trash(Trash::Fb(f.fb));
            }
        }
    }

    /// The copy of image rows/columns [x0, x1) x [y0, y1) to or from VRAM.
    /// Buffer offsets must be 4-byte aligned: x0 is rounded down to suit,
    /// and a pitch that is not a multiple of 4 falls back to the image's
    /// first row.
    fn region(im: &Img, mut x0: u32, mut y0: u32, x1: u32, mut y1: u32) -> vk::BufferImageCopy {
        if im.pitch % 4 != 0 {
            y1 = if y1 > y0 { y1 } else { im.height };
            y0 = 0;
        }
        if im.bpp < 4 {
            x0 &= !(4 / im.bpp - 1);
        }
        vk::BufferImageCopy {
            buffer_offset: im.addr as u64 + y0 as u64 * im.pitch as u64 + x0 as u64 * im.bpp as u64,
            buffer_row_length: im.pitch / im.bpp,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: x0 as i32, y: y0 as i32, z: 0 },
            image_extent: vk::Extent3D { width: x1 - x0, height: y1 - y0, depth: 1 },
        }
    }

    /// Write the rectangles rendered since the last write-back to VRAM.
    pub fn copy_to_vram(&mut self, i: usize) {
        let cb = self.cb();
        let (regions, img): (Vec<vk::BufferImageCopy>, vk::Image) = {
            let im = &self.img[i];
            (
                (0..im.ndr).map(|k| Self::region(im, im.dr[k][0], im.dr[k][1], im.dr[k][2], im.dr[k][3])).collect(),
                im.img,
            )
        };
        if !regions.is_empty() {
            self.end_pass();
            self.full_barrier(cb);
            unsafe {
                self.dev.cmd_copy_image_to_buffer(cb, img, vk::ImageLayout::GENERAL, self.vram_buf, &regions);
            }
            self.full_barrier(cb);
            self.stat_writebacks += 1;
        }
        self.img[i].dirty = false;
        self.img[i].ndr = 0;
    }

    /// Write back every image with rendering newer than VRAM in [lo, hi).
    pub fn writeback_range(&mut self, lo: u64, hi: u64) {
        for i in 0..MAX_IMG {
            let im = &self.img[i];
            if im.live && im.dirty && lo < im.hi() && (im.addr as u64) < hi {
                self.copy_to_vram(i);
            }
        }
    }

    fn img_free(&mut self, i: usize) {
        if self.img[i].dirty {
            self.copy_to_vram(i);
        }
        if self.in_pass && self.pass_img[..self.pass_n].contains(&i) {
            self.end_pass();
        }
        let (view, img, mem) = (self.img[i].view, self.img[i].img, self.img[i].mem);
        self.fb_forget(view);
        self.trash(Trash::View(view));
        self.trash(Trash::Image(img));
        self.trash(Trash::Mem(mem));
        self.img[i] = Img::empty();
    }

    pub fn layout_general(&self, cb: vk::CommandBuffer, img: vk::Image, levels: u32, layers: u32) {
        let ib = [vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(img)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: levels,
                base_array_layer: 0,
                layer_count: layers,
            })
            .build()];
        unsafe {
            self.dev.cmd_pipeline_barrier(
                cb,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &ib,
            );
        }
    }

    /// The image for a VRAM range: w x h texels of bpp bytes at addr, rows
    /// pitch bytes apart.  Its contents are current: copied from VRAM when
    /// a CPU write or another image's rendering made them stale (after
    /// writing such rendering back).
    pub fn img_get(
        &mut self,
        cls: ImgCls,
        addr: u32,
        w: u32,
        h: u32,
        pitch: u32,
        fmt: vk::Format,
        bpp: u32,
    ) -> Option<usize> {
        let maxdim = self.props.limits.max_image_dimension2_d;
        if w == 0
            || h == 0
            || bpp == 0
            || pitch < w * bpp
            || addr % 4 != 0
            || addr % bpp != 0
            || pitch % bpp != 0
            || addr as u64 + pitch as u64 * (h as u64 - 1) + w as u64 * bpp as u64 > self.vram_size
            || w > maxdim
            || h > maxdim
        {
            return None;
        }
        let mut found = (0..MAX_IMG).find(|&i| {
            let c = &self.img[i];
            c.live && c.cls == cls && c.addr == addr && c.width == w && c.height == h && c.pitch == pitch && c.fmt == fmt
        });
        if found.is_none() {
            // a free slot, else the least recently used not in the open pass
            let mut lru: Option<usize> = None;
            for i in 0..MAX_IMG {
                if !self.img[i].live {
                    lru = Some(i);
                    break;
                }
                let busy = self.in_pass && self.pass_img[..self.pass_n].contains(&i);
                if !busy && lru.map_or(true, |l| self.img[i].used < self.img[l].used) {
                    lru = Some(i);
                }
            }
            let i = lru?;
            if self.img[i].live {
                self.img_free(i);
            }
            let usage = vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | if cls == ImgCls::Rt {
                    vk::ImageUsageFlags::COLOR_ATTACHMENT
                        | vk::ImageUsageFlags::INPUT_ATTACHMENT
                        | if self.fb_image { vk::ImageUsageFlags::STORAGE } else { vk::ImageUsageFlags::empty() }
                } else {
                    vk::ImageUsageFlags::SAMPLED
                };
            let (img, mem, view) =
                match self.image(vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D, fmt, w, h, 1, 1, 1, usage) {
                    Ok(x) => x,
                    Err(e) => {
                        crate::state::fail(&e);
                        return None;
                    }
                };
            self.img[i] = Img {
                live: true,
                cls,
                addr,
                width: w,
                height: h,
                pitch,
                fmt,
                bpp,
                img,
                mem,
                view,
                ..Img::empty()
            };
            self.end_pass();
            let cb = self.cb();
            self.layout_general(cb, img, 1, 1);
            found = Some(i);
        }
        let i = found.unwrap();
        self.clock += 1;
        self.img[i].used = self.clock;
        // PPCGPU_NO_CACHE=1: distrust every cached image and re-read it from VRAM
        // on every use (slow, but correct whatever the coherency tracking misses).
        // A way to tell a cache bug from a rendering bug.
        if no_cache() {
            if self.img[i].dirty {
                self.copy_to_vram(i);
            }
            self.img[i].valid = false;
        }

        let hi = self.img[i].hi();
        let gen = self.range_gen(addr as u64, hi);
        if !self.img[i].valid || gen > self.img[i].gen {
            // The rows holding pages newer than the image (all, if new).
            let (mut y0, mut y1) = (0u32, h);
            if self.img[i].valid {
                let (mut lo_p, mut hi_p) = (u64::MAX, 0u64);
                let mut p = Self::pg(addr as u64);
                while p <= Self::pg(hi - 1) && p < self.npages {
                    if self.page_gen[p as usize] > self.img[i].gen {
                        lo_p = lo_p.min(p);
                        hi_p = p;
                    }
                    p += 1;
                }
                let blo = (lo_p * PAGE).max(addr as u64);
                let bhi = ((hi_p + 1) * PAGE).min(hi);
                y0 = ((blo - addr as u64) / pitch as u64) as u32;
                y1 = (((bhi - addr as u64 + pitch as u64 - 1) / pitch as u64) as u32).min(h);
            }
            // Rendering over those rows that VRAM lacks goes there first.
            self.writeback_range(addr as u64 + y0 as u64 * pitch as u64, addr as u64 + y1 as u64 * pitch as u64);
            let cb = self.cb();
            let r = Self::region(&self.img[i], 0, y0, w, y1);
            self.end_pass();
            self.full_barrier(cb);
            unsafe {
                self.dev.cmd_copy_buffer_to_image(cb, self.vram_buf, self.img[i].img, vk::ImageLayout::GENERAL, &[r]);
            }
            self.full_barrier(cb);
            self.note_read(addr as u64 + y0 as u64 * pitch as u64, addr as u64 + y1 as u64 * pitch as u64);
            self.img[i].valid = true;
            self.img[i].gen = gen;
            self.stat_uploads += 1;
        }
        Some(i)
    }

    /// A draw renders into rectangle [x0, x1) x [y0, y1) of image i: the
    /// pages of those rows get a new generation that the image holds, and
    /// the rectangle is written back with the batch.
    pub fn img_written(&mut self, i: usize, x0: u32, y0: u32, x1: u32, y1: u32) {
        let s = {
            let b = self.batch();
            self.batch[b].seq
        };
        let fresh = self.img[i].gen == self.gen; // nothing else happened since
        let x1 = x1.min(self.img[i].width);
        let y1 = y1.min(self.img[i].height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        // Keep the rectangle: inside one we have, merged into the one it
        // grows least, or new.
        let mut best: Option<usize> = None;
        let mut grow = u64::MAX;
        {
            let im = &self.img[i];
            for k in 0..im.ndr {
                let d = &im.dr[k];
                let a0 = (d[2] - d[0]) as u64 * (d[3] - d[1]) as u64;
                let a1 = (d[2].max(x1) - d[0].min(x0)) as u64 * (d[3].max(y1) - d[1].min(y0)) as u64;
                if a1 - a0 < grow {
                    grow = a1 - a0;
                    best = Some(k);
                }
            }
        }
        {
            let im = &mut self.img[i];
            match best {
                Some(k) if grow == 0 || im.ndr == MAX_DR => {
                    let d = &mut im.dr[k];
                    d[0] = d[0].min(x0);
                    d[1] = d[1].min(y0);
                    d[2] = d[2].max(x1);
                    d[3] = d[3].max(y1);
                }
                _ => {
                    im.dr[im.ndr] = [x0, y0, x1, y1];
                    im.ndr += 1;
                }
            }
            im.dirty = true;
            im.valid = true;
        }
        if grow == 0 && fresh {
            return; // those pages are already stamped
        }
        let (lo, hi) = {
            let im = &self.img[i];
            (
                im.addr as u64 + y0 as u64 * im.pitch as u64 + x0 as u64 * im.bpp as u64,
                im.addr as u64 + (y1 as u64 - 1) * im.pitch as u64 + x1 as u64 * im.bpp as u64,
            )
        };
        self.gen += 1;
        let g = self.gen;
        let mut p = Self::pg(lo);
        while p <= Self::pg(hi - 1) && p < self.npages {
            self.page_gen[p as usize] = g;
            self.page_wseq[p as usize] = s;
            p += 1;
        }
        self.img[i].gen = g;
    }

    pub fn range_busy(&self, lo: u64, hi: u64, write_access: bool) -> bool {
        if self.vram.is_null() || hi <= lo {
            return false;
        }
        let hi = hi.min(self.vram_size);
        let mut p = Self::pg(lo);
        while p <= Self::pg(hi - 1) && p < self.npages {
            if self.page_pending_write(p) {
                return true;
            }
            if write_access {
                let s = self.page_rseq[p as usize];
                if s != 0 {
                    let cur = matches!(self.cur, Some(i) if self.batch[i].seq == s);
                    if cur || !self.seq_done(s) {
                        return true;
                    }
                }
            }
            p += 1;
        }
        false
    }
}
