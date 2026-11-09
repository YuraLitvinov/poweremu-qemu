//! R300 draws: bind and draw the packets r300/r300_draw.c assembles, with
//! the GLSL r300/r300_us.c generates.
//!
//! The fragment shader reads the buffers it blends into as input
//! attachments that are also its colour attachments (a feedback loop, all
//! images in GENERAL layout).  Draws are ordered by a by-region
//! self-dependency barrier between them.

use std::ffi::CStr;

use ash::vk;

use crate::state::*;
use crate::sys::*;

fn rt_format(view: u32) -> vk::Format {
    match view {
        R300_RTV_R8U => vk::Format::R8_UINT,
        R300_RTV_R16U => vk::Format::R16_UINT,
        R300_RTV_R32U => vk::Format::R32_UINT,
        R300_RTV_RG32U => vk::Format::R32G32_UINT,
        R300_RTV_RGBA32U => vk::Format::R32G32B32A32_UINT,
        _ => vk::Format::R8G8B8A8_UNORM,
    }
}

fn round_up(v: u64, a: u64) -> u64 {
    v.div_ceil(a) * a
}

impl Vk {
    pub fn draw_r300(&mut self, vram_ptr: *mut u8, pkt: &R300DrawPacket) -> i32 {
        let ncb = pkt.num_cb.max(1) as usize;
        let (sx0, sy0) = (pkt.scissor[0], pkt.scissor[1]);
        let sx1 = pkt.scissor[2].min(pkt.rt_width);
        let sy1 = pkt.scissor[3].min(pkt.rt_height);
        let ns = pkt.aa_samples.clamp(1, 6);

        if self.vram.is_null() || vram_ptr != self.vram {
            return -1;
        }
        if sx0 >= sx1 || sy0 >= sy1 || pkt.num_verts + pkt.num_line_verts == 0 {
            return 0;
        }
        if pkt.cull & (R300_CULL_FRONT | R300_CULL_BACK) == (R300_CULL_FRONT | R300_CULL_BACK) && pkt.num_line_verts == 0 {
            return 0; // every polygon culled
        }
        if ns > 1 {
            let mut same_pitch = !pkt.depth.attach || pkt.depth.pitch == pkt.rt_pitch;
            for k in 1..ncb {
                same_pitch &= pkt.cb[k].pitch == pkt.rt_pitch;
            }
            if !same_pitch || ns as u64 * pkt.rt_pitch as u64 > self.props.limits.max_image_dimension2_d as u64 {
                self.warn(4, "multisampled buffers of different pitches");
                return -1;
            }
        }
        // The framebuffer: sample k of row y is row y * ns + k (see Metal).
        let fbw = if ns > 1 { ns * pkt.rt_pitch } else { pkt.rt_width };
        let mut fmt = [vk::Format::UNDEFINED; MAX_ATT];
        let mut addr = [0u32; MAX_ATT];
        let mut bpr = [0u32; MAX_ATT];
        let mut bpp = [0u32; MAX_ATT];
        for k in 0..ncb {
            let view = if k > 0 { pkt.cb[k].view } else { pkt.rt_view };
            addr[k] = if k > 0 { pkt.cb[k].gpu_addr } else { pkt.rt_gpu_addr };
            bpp[k] = if k > 0 { pkt.cb[k].bpp } else { pkt.rt_bpp };
            bpr[k] = ns * (if k > 0 { pkt.cb[k].pitch } else { pkt.rt_pitch }) * bpp[k];
            fmt[k] = rt_format(view);
        }
        let zd = &pkt.depth;
        let mut want_z = zd.attach;
        if want_z && (zd.pitch < pkt.rt_width || zd.gpu_addr % 4 != 0) {
            self.warn(5, "depth buffer not usable");
            want_z = false;
        }

        self.harvest();
        {
            let uu = pkt.uniforms;
            self.dump_draw(pkt, &uu, ns, want_z);
            self.record_draw(self.dump_draws, pkt);
        }

        // Textures whose bytes the CPU reads (rebuilt ones, and anything in
        // VRAM a batch still renders into) first: that may need a flush,
        // which must not come in the middle of recording this draw.
        let mut u: R300FSUniforms = pkt.uniforms;
        let mut need_flush = false;
        for t in 0..NTEX {
            let td = &pkt.tex[t];
            let direct = td.levels <= 1
                && td.dim == R300_TEXDIM_2D
                && (td.kind == R300_TEXK_RGBA8 || td.kind == R300_TEXK_R8 || td.kind == R300_TEXK_RG8);
            if td.bound
                && td.host_data.is_null()
                && td.kind != R300_TEXK_RAW
                && !direct
                && td.gpu_addr as u64 + td.size_bytes as u64 <= self.vram_size
                && self.range_pending(td.gpu_addr as u64, td.gpu_addr as u64 + td.size_bytes as u64)
            {
                need_flush = true;
            }
        }
        if need_flush {
            self.flush();
        }
        if !self.dummies() {
            return -1;
        }

        let mut tii = [vk::DescriptorImageInfo::default(); NTEX];
        let mut aux_bytes: u32 = 0;
        for t in 0..NTEX {
            let td = &pkt.tex[t];
            let raw = td.kind == R300_TEXK_RAW;
            let sampler = self.sampler(td.filter0, td.levels);
            tii[t] = vk::DescriptorImageInfo {
                sampler,
                image_view: self.dummy[if raw { 0 } else { td.dim.min(2) as usize }].2,
                image_layout: vk::ImageLayout::GENERAL,
            };
            u.tex_addr[t][0] = 0;
            u.tex_addr[t][1] = 0;
            if !td.bound {
                u.tex_info[t][0] = 0;
                continue;
            }
            if td.host_data.is_null() && td.gpu_addr as u64 + td.size_bytes as u64 > self.vram_size {
                self.warn(6, "texture outside VRAM");
                u.tex_info[t][0] = 0;
                continue;
            }
            if raw {
                if !td.host_data.is_null() {
                    u.tex_addr[t][0] = aux_bytes;
                    u.tex_addr[t][1] = 1;
                    aux_bytes += round_up(td.size_bytes as u64, 16) as u32;
                } else {
                    // The shader reads VRAM: rendering to it must be there.
                    u.tex_addr[t][0] = td.gpu_addr;
                    let (lo, hi) = (td.gpu_addr as u64, td.gpu_addr as u64 + td.size_bytes as u64);
                    self.writeback_range(lo, hi);
                    self.note_read(lo, hi);
                }
                continue;
            }
            let direct = td.host_data.is_null()
                && td.levels <= 1
                && td.dim == R300_TEXDIM_2D
                && (td.kind == R300_TEXK_RGBA8 || td.kind == R300_TEXK_R8 || td.kind == R300_TEXK_RG8);
            let mut view = vk::ImageView::null();
            if direct {
                let b: u32 = if td.kind == R300_TEXK_RGBA8 {
                    4
                } else if td.kind == R300_TEXK_RG8 {
                    2
                } else {
                    1
                };
                let (mut w, mut h) = (td.width, td.height);
                // like r200_view: shrink what does not fit the pitch or VRAM
                if w as u64 * b as u64 > td.pitch_bytes as u64 {
                    w = td.pitch_bytes / b;
                }
                if w != 0
                    && td.gpu_addr as u64 + (h as u64 - 1) * td.pitch_bytes as u64 + w as u64 * b as u64 > self.vram_size
                {
                    h = ((self.vram_size - td.gpu_addr as u64 - w as u64 * b as u64) / td.pitch_bytes as u64 + 1) as u32;
                }
                match self.img_get(ImgCls::Tex, td.gpu_addr, w, h, td.pitch_bytes, Self::tex_format(td.kind), b) {
                    Some(i) => view = self.img[i].view,
                    None => {
                        if !self.range_pending(td.gpu_addr as u64, td.gpu_addr as u64 + td.size_bytes as u64) {
                            // odd pitch: repacked on the CPU
                            view = self.texture_full(td).unwrap_or(vk::ImageView::null());
                        }
                    }
                }
            } else {
                view = self.texture_full(td).unwrap_or(vk::ImageView::null());
            }
            if view == vk::ImageView::null() {
                u.tex_info[t][0] = 0;
                continue;
            }
            tii[t].image_view = view;
        }

        // Shader-decoded texels copied out of the GART.
        let Some((auxb, auxo, auxp)) = self.arena(aux_bytes.max(16) as u64) else {
            return -1;
        };
        if aux_bytes != 0 {
            for t in 0..NTEX {
                let td = &pkt.tex[t];
                if td.bound && td.kind == R300_TEXK_RAW && !td.host_data.is_null() {
                    unsafe {
                        let dw = auxp.add(u.tex_addr[t][0] as usize) as *mut u32;
                        std::ptr::copy_nonoverlapping(td.host_data, dw as *mut u8, td.size_bytes as usize);
                        // GART dwords reversed to VRAM's side (r300_tex_raw_bpp)
                        for i in 0..(td.size_bytes as usize).div_ceil(4) {
                            *dw.add(i) = (*dw.add(i)).swap_bytes();
                        }
                    }
                }
            }
        }

        // Colour buffers and the depth buffer.
        let mut att = [0usize; MAX_ATT];
        for k in 0..ncb {
            match self.img_get(ImgCls::Rt, addr[k], fbw, pkt.rt_height, bpr[k], fmt[k], bpp[k]) {
                Some(i) => att[k] = i,
                None => {
                    self.warn(7, if k == 0 { "colour buffer unusable" } else { "render target B-D unusable" });
                    return -1;
                }
            }
        }
        let mut natt = ncb;
        let mut z16 = zd.bpp == 2;
        if want_z {
            let zf = if z16 { vk::Format::R16_UINT } else { vk::Format::R32_UINT };
            let zw = if ns > 1 { ns * zd.pitch } else { pkt.rt_width };
            let zi = self.img_get(ImgCls::Rt, zd.gpu_addr, zw, pkt.rt_height, ns * zd.pitch * zd.bpp, zf, zd.bpp);
            match zi {
                Some(i) if zw == fbw => {
                    att[ncb] = i;
                    fmt[ncb] = zf;
                    natt += 1;
                }
                _ => {
                    self.warn(9, "depth buffer not usable in place");
                    want_z = false;
                }
            }
        }
        // Keep the open pass's depth buffer bound for depth-less draws.
        if !want_z && self.in_pass && self.pass_z && self.pass_n == ncb + 1 {
            let same = (0..ncb).all(|k| self.pass_img[k] == att[k]);
            if same {
                att[ncb] = self.pass_img[ncb];
                fmt[ncb] = self.img[att[ncb]].fmt;
                natt += 1;
                z16 = self.pass_z16;
                u.zinfo[0] = 0;
                u.zinfo[3] = if z16 { R300_ZFMT_Z16 } else { 0 };
                u.poly_en = 0;
            }
        }
        let pass_z = natt > ncb;

        let Some(rp) = self.render_pass(&fmt[..natt]) else {
            return -1;
        };
        let topo = if pkt.prim_class == 1 { vk::PrimitiveTopology::LINE_LIST } else { vk::PrimitiveTopology::TRIANGLE_LIST };
        let pk = PipeKey {
            rp,
            ncb: ncb as u32,
            z: pass_z,
            topo: topo.as_raw(),
            cull: if pkt.cull & R300_CULL_FRONT != 0 {
                vk::CullModeFlags::FRONT.as_raw()
            } else if pkt.cull & R300_CULL_BACK != 0 {
                vk::CullModeFlags::BACK.as_raw()
            } else {
                vk::CullModeFlags::NONE.as_raw()
            },
            front: if pkt.cull & R300_FACE_CW == 0 {
                vk::FrontFace::COUNTER_CLOCKWISE.as_raw()
            } else {
                vk::FrontFace::CLOCKWISE.as_raw()
            },
            z16: pass_z && z16,
        };
        let glsl: &CStr = unsafe { CStr::from_ptr(pkt.glsl) };
        let key = ProgKey::Text(glsl.to_string_lossy().into_owned());
        let mut pipe = vk::Pipeline::null();
        let mut lpipe = vk::Pipeline::null();
        if pkt.num_verts != 0 {
            match self.pipeline(&key, glsl, &pk) {
                Some(p) => pipe = p,
                None => return -1,
            }
        }
        if pkt.num_line_verts != 0 {
            // polygon-mode edges: never culled
            let lk = PipeKey { topo: vk::PrimitiveTopology::LINE_LIST.as_raw(), cull: 0, ..pk };
            match self.pipeline(&key, glsl, &lk) {
                Some(p) => lpipe = p,
                None => return -1,
            }
        }

        // Per-draw data: vertices, and each sample's uniforms and shift.
        let nv = (pkt.num_verts + pkt.num_line_verts) as usize;
        let vsize = std::mem::size_of::<R300Vertex>() as u64;
        let usize_ = std::mem::size_of::<R300FSUniforms>() as u64;
        let Some((vb, vo, vp)) = self.arena(nv as u64 * vsize) else {
            return -1;
        };
        let ustride = round_up(usize_, self.align);
        let mstride = self.align;
        let Some((ub, uo, up)) = self.arena((ustride + mstride) * ns as u64) else {
            return -1;
        };
        unsafe {
            std::ptr::copy_nonoverlapping(pkt.verts, vp as *mut R300Vertex, nv);
            for k in 0..ns as u64 {
                let uk = up.add((k * ustride) as usize) as *mut R300FSUniforms;
                let ms = up.add((ns as u64 * ustride + k * mstride) as usize) as *mut f32;
                *uk = u;
                std::ptr::write_bytes(ms as *mut u8, 0, 16);
                if ns > 1 {
                    let cx = k as u32 * pkt.rt_pitch;
                    *ms = -2.0 * pkt.aa_pos[k as usize][0] / pkt.rt_width as f32;
                    *ms.add(1) = 2.0 * pkt.aa_pos[k as usize][1] / pkt.rt_height as f32;
                    for i in 0..4 {
                        (*uk).cliprect[i][0] += cx as i32;
                        (*uk).cliprect[i][2] += cx as i32;
                    }
                }
            }
        }

        // The descriptor set.
        let bi = self.batch();
        let set = loop {
            if self.batch[bi].pool_cur == self.batch[bi].pools.len() {
                let ps = [
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, descriptor_count: 2 * SETS_PER_POOL },
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::STORAGE_BUFFER, descriptor_count: 4 * SETS_PER_POOL },
                    vk::DescriptorPoolSize {
                        ty: if self.fb_image { vk::DescriptorType::STORAGE_IMAGE } else { vk::DescriptorType::INPUT_ATTACHMENT },
                        descriptor_count: MAX_ATT as u32 * SETS_PER_POOL,
                    },
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: NTEX as u32 * SETS_PER_POOL,
                    },
                ];
                let pci = vk::DescriptorPoolCreateInfo::builder().max_sets(SETS_PER_POOL).pool_sizes(&ps);
                match unsafe { self.dev.create_descriptor_pool(&pci, None) } {
                    Ok(p) => self.batch[bi].pools.push(p),
                    Err(_) => return -1,
                }
            }
            let pool = self.batch[bi].pools[self.batch[bi].pool_cur];
            let layouts = [self.dsl];
            let dai = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(pool).set_layouts(&layouts);
            match unsafe { self.dev.allocate_descriptor_sets(&dai) } {
                Ok(s) => break s[0],
                Err(_) => self.batch[bi].pool_cur += 1,
            }
        };
        let bufi = [
            vk::DescriptorBufferInfo { buffer: ub, offset: uo, range: usize_ },
            vk::DescriptorBufferInfo { buffer: self.zpass_buf, offset: 0, range: 16 },
            vk::DescriptorBufferInfo { buffer: vb, offset: vo, range: nv as u64 * vsize },
            vk::DescriptorBufferInfo { buffer: ub, offset: uo + ns as u64 * ustride, range: 16 },
            vk::DescriptorBufferInfo { buffer: self.vram_buf, offset: 0, range: vk::WHOLE_SIZE },
            vk::DescriptorBufferInfo { buffer: auxb, offset: auxo, range: aux_bytes.max(16) as u64 },
        ];
        let bt = [
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::DescriptorType::STORAGE_BUFFER,
        ];
        let mut ai = [vk::DescriptorImageInfo::default(); MAX_ATT];
        for k in 0..natt {
            ai[k] = vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: self.img[att[k]].view,
                image_layout: vk::ImageLayout::GENERAL,
            };
        }
        let mut wr: Vec<vk::WriteDescriptorSet> = Vec::with_capacity(8);
        for i in 0..6 {
            wr.push(
                vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(i as u32)
                    .descriptor_type(bt[i])
                    .buffer_info(std::slice::from_ref(&bufi[i]))
                    .build(),
            );
        }
        wr.push(
            vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(R300_BIND_FB0)
                .descriptor_type(if self.fb_image { vk::DescriptorType::STORAGE_IMAGE } else { vk::DescriptorType::INPUT_ATTACHMENT })
                .image_info(&ai[..natt])
                .build(),
        );
        wr.push(
            vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(R300_BIND_TEX0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&tii)
                .build(),
        );
        unsafe { self.dev.update_descriptor_sets(&wr, &[]) };

        // The render pass: reuse the open one if it has these attachments.
        let cb = self.batch[bi].cb;
        let same = self.in_pass && self.pass_rp == rp && self.pass_n == natt && (0..natt).all(|k| self.pass_img[k] == att[k]);
        if !same {
            self.end_pass();
            let Some(fb) = self.framebuffer(rp, &att[..natt], fbw, pkt.rt_height) else {
                return -1;
            };
            let rbi = vk::RenderPassBeginInfo::builder().render_pass(rp).framebuffer(fb).render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D { width: fbw, height: pkt.rt_height },
            });
            unsafe { self.dev.cmd_begin_render_pass(cb, &rbi, vk::SubpassContents::INLINE) };
            self.in_pass = true;
            self.pass_rp = rp;
            self.pass_n = natt;
            self.pass_img[..natt].copy_from_slice(&att[..natt]);
            self.pass_z = pass_z;
            self.pass_z16 = z16;
            self.stat_passes += 1;
        } else {
            // This draw reads what the previous one wrote.  MoltenVK needs
            // it too: without it blending read stale destination pixels.
            let fbi = self.fb_image;
            let mb = [vk::MemoryBarrier::builder()
                .src_access_mask(if fbi { vk::AccessFlags::SHADER_WRITE } else { vk::AccessFlags::COLOR_ATTACHMENT_WRITE })
                .dst_access_mask(if fbi {
                    vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE
                } else {
                    vk::AccessFlags::INPUT_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                })
                .build()];
            unsafe {
                self.dev.cmd_pipeline_barrier(
                    cb,
                    if fbi { vk::PipelineStageFlags::FRAGMENT_SHADER } else { vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT },
                    if fbi {
                        vk::PipelineStageFlags::FRAGMENT_SHADER
                    } else {
                        vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    },
                    vk::DependencyFlags::BY_REGION,
                    &mb,
                    &[],
                    &[],
                );
            }
        }

        for k in 0..ns {
            let cx = if ns > 1 { k * pkt.rt_pitch } else { 0 };
            let dynoff = [(k as u64 * ustride) as u32, (k as u64 * mstride) as u32];
            // Clip space has y up (as Metal's): flip the viewport.
            let vpt = vk::Viewport {
                x: cx as f32,
                y: pkt.rt_height as f32,
                width: pkt.rt_width as f32,
                height: -(pkt.rt_height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let sc = vk::Rect2D {
                offset: vk::Offset2D { x: (cx + sx0) as i32, y: sy0 as i32 },
                extent: vk::Extent2D { width: sx1 - sx0, height: sy1 - sy0 },
            };
            unsafe {
                self.dev.cmd_set_viewport(cb, 0, &[vpt]);
                self.dev.cmd_set_scissor(cb, 0, &[sc]);
                if pkt.num_verts != 0 {
                    self.dev.cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, pipe);
                    self.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, self.layout, 0, &[set], &dynoff);
                    self.dev.cmd_draw(cb, pkt.num_verts, 1, 0, 0);
                }
                if pkt.num_line_verts != 0 {
                    self.dev.cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, lpipe);
                    self.dev.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, self.layout, 0, &[set], &dynoff);
                    self.dev.cmd_draw(cb, pkt.num_line_verts, 1, pkt.num_verts, 0);
                }
            }
        }
        // What the scissor let through, over every sample's columns.
        for k in 0..natt {
            let x1 = if ns > 1 { (ns - 1) * pkt.rt_pitch + sx1 } else { sx1 };
            self.img_written(att[k], sx0, sy0, x1, sy1);
        }
        self.stat_draws += 1;
        if self.stat_draws % 20000 == 0 {
            eprintln!(
                "ppc-mac-gpu vulkan: {} draws, {} passes, {} uploads, {} write-backs, {} flushes",
                self.stat_draws, self.stat_passes, self.stat_uploads, self.stat_writebacks, self.stat_flushes
            );
        }
        0
    }
}

impl Vk {
    /// PPCGPU_DUMP: a line for every distinct combination of program, state and
    /// textures, written once when first seen (count them with the draw index).
    pub fn dump_draw(&mut self, pkt: &R300DrawPacket, u: &R300FSUniforms, ns: u32, want_z: bool) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::io::Write;
        self.dump_draws += 1;
        let n = self.dump_draws;
        if self.dump.is_none() {
            return;
        }
        let glsl = unsafe { CStr::from_ptr(pkt.glsl) }.to_string_lossy();
        let mut h = DefaultHasher::new();
        glsl.hash(&mut h);
        let ph = h.finish();
        let mut sig = format!(
            "prog={:016x} prim={} cull={:#x} aa={} cb={} z={}({}b) rt={}x{} nv={}+{}",
            ph, pkt.prim_class, pkt.cull, ns, pkt.num_cb, want_z, pkt.depth.bpp, pkt.rt_width, pkt.rt_height,
            pkt.num_verts, pkt.num_line_verts
        );
        for t in 0..NTEX {
            let td = &pkt.tex[t];
            if !td.bound {
                continue;
            }
            sig += &format!(
                " | t{} kind={} fmt={:#x} dim={} {}x{}x{} lv={} pitch={} f0={:#x} f1={:#x} endian={} tile={:#x} vbpp={} host={} swz={:?} info={:?}",
                t, td.kind, td.format, td.dim, td.width, td.height, td.depth, td.levels, td.pitch_bytes,
                td.filter0, td.filter1, td.endian, td.tile, td.view_bpp, !td.host_data.is_null(), u.tex_swz[t], u.tex_info[t]
            );
        }
        // running aggregates over EVERY draw of this kind
        {
            let nvv = (pkt.num_verts + pkt.num_line_verts) as usize;
            let st = self.dump_stats.entry(sig.clone()).or_insert_with(|| DumpStat {
                draws: 0, verts: 0, black: 0,
                v0min: [f32::MAX; 4], v0max: [f32::MIN; 4], v1min: [f32::MAX; 4], v1max: [f32::MIN; 4],
                first: n,
                w_neg: 0, w_tiny: 0, w_min: f32::MAX, zw_min: f32::MAX, zw_max: f32::MIN, xyw_max: 0.0, z_out: 0,
                state: format!("cblend={:#x} ablend={:#x} alpha_func={} chanmask={:#x} zinfo={:?} fog={:#x} rop={:#x}",
                               u.cblend, u.ablend, u.alpha_func, u.chanmask, u.zinfo, u.fog_blend, u.rop),
            });
            st.draws += 1;
            st.verts += nvv as u64;
            for i in 0..nvv {
                let v = unsafe { &*pkt.verts.add(i) };
                {
                    let (x, y, z, w) = (v.pos[0], v.pos[1], v.pos[2], v.pos[3]);
                    st.w_min = st.w_min.min(w);
                    if w <= 0.0 { st.w_neg += 1; } else if w < 1e-3 { st.w_tiny += 1; }
                    if w > 0.0 {
                        st.zw_min = st.zw_min.min(z / w);
                        st.zw_max = st.zw_max.max(z / w);
                        st.xyw_max = st.xyw_max.max((x / w).abs()).max((y / w).abs());
                    }
                    if z < 0.0 || z > w { st.z_out += 1; }
                }
                for k in 0..4 {
                    if !v.v[0][k].is_nan() {
                        st.v0min[k] = st.v0min[k].min(v.v[0][k]);
                        st.v0max[k] = st.v0max[k].max(v.v[0][k]);
                    }
                    if !v.v[1][k].is_nan() {
                        st.v1min[k] = st.v1min[k].min(v.v[1][k]);
                        st.v1max[k] = st.v1max[k].max(v.v[1][k]);
                    }
                }
                if v.v[1][0] == 0.0 && v.v[1][1] == 0.0 && v.v[1][2] == 0.0 {
                    st.black += 1;
                }
            }
        }
        if n % 1500 == 0 {
            let mut v: Vec<(&String, &DumpStat)> = self.dump_stats.iter().collect();
            v.sort_by_key(|(_, s)| std::cmp::Reverse(s.draws));
            let mut out = String::new();
            for (sig, s) in v {
                out += &format!(
                    "{} draws (first #{}), {} verts, {} black\n  {}\n  {}\n  v0 min={:?} max={:?}\n  v1 min={:?} max={:?}\n  pos: w<=0 {}  w<1e-3 {}  w_min {}  z/w [{}, {}]  max|x/w|,|y/w| {}  z outside [0,w] {}\n",
                    s.draws, s.first, s.verts, s.black, sig, s.state, s.v0min, s.v0max, s.v1min, s.v1max,
                    s.w_neg, s.w_tiny, s.w_min, s.zw_min, s.zw_max, s.xyw_max, s.z_out
                );
            }
            let _ = std::fs::write(format!("{}.stats", self.dump.as_ref().map(|d| d.0.as_str()).unwrap_or("/dev/null")), out);
        }
        let Some((path, sigs, progs)) = self.dump.as_mut() else {
            return;
        };
        if sigs.insert(sig.clone()) {
            let mut line = format!("draw#{} {}\n", n, sig);
            for i in 0..(pkt.num_verts.min(3) as usize) {
                let v = unsafe { &*pkt.verts.add(i) };
                line += &format!(
                    "    v{} pos={:?} v0={:?} v1={:?} v2={:?} aux={:?}\n",
                    i, v.pos, v.v[0], v.v[1], v.v[2], v.aux
                );
            }
            // The whole draw's vertex data, not just three vertices: ranges and oddities.
            let nvv = (pkt.num_verts + pkt.num_line_verts) as usize;
            let (mut mn0, mut mx0) = ([f32::MAX; 4], [f32::MIN; 4]);
            let (mut mn1, mut mx1) = ([f32::MAX; 4], [f32::MIN; 4]);
            let mut nan = 0usize;
            let mut zero_col = 0usize;
            for i in 0..nvv {
                let v = unsafe { &*pkt.verts.add(i) };
                for k in 0..4 {
                    if v.v[0][k].is_nan() || v.v[1][k].is_nan() || v.pos[k].is_nan() {
                        nan += 1;
                        continue;
                    }
                    mn0[k] = mn0[k].min(v.v[0][k]);
                    mx0[k] = mx0[k].max(v.v[0][k]);
                    mn1[k] = mn1[k].min(v.v[1][k]);
                    mx1[k] = mx1[k].max(v.v[1][k]);
                }
                if v.v[1][0] == 0.0 && v.v[1][1] == 0.0 && v.v[1][2] == 0.0 {
                    zero_col += 1;
                }
            }
            line += &format!(
                "    ALL {} verts: v0 min={:?} max={:?} | v1 min={:?} max={:?} | nan={} black-colour verts={}\n",
                nvv, mn0, mx0, mn1, mx1, nan, zero_col
            );
            // Byte statistics of the 2D textures (which byte is alpha?), once per signature
            for t in 0..NTEX {
                let td = &pkt.tex[t];
                if td.bound && td.dim == R300_TEXDIM_2D && td.kind == R300_TEXK_RGBA8 && td.host_data.is_null()
                    && (td.gpu_addr as u64 + td.size_bytes as u64) <= self.vram_size
                {
                    let (w, h) = (td.width as usize, td.height as usize);
                    let pitch = td.pitch_bytes as usize;
                    let mut mn = [255u8; 4]; let mut mx = [0u8; 4]; let mut sum = [0u64; 4];
                    for y in 0..h { for x in 0..w { for c in 0..4 {
                        let b = unsafe { *self.vram.add(td.gpu_addr as usize + y * pitch + x * 4 + c) };
                        mn[c] = mn[c].min(b); mx[c] = mx[c].max(b); sum[c] += b as u64;
                    } } }
                    line += &format!("    2D tex unit {} {}x{} @{:#x}: bytes b0..b3 min={:?} max={:?} mean=[{:.0},{:.0},{:.0},{:.0}]\n",
                        t, w, h, td.gpu_addr, mn, mx, sum[0] as f64 / (w * h) as f64, sum[1] as f64 / (w * h) as f64,
                        sum[2] as f64 / (w * h) as f64, sum[3] as f64 / (w * h) as f64);
                }
            }
            // Contents of small 3D textures (the stipple volumes), once each
            for t in 0..NTEX {
                let td = &pkt.tex[t];
                if td.bound && td.dim == R300_TEXDIM_3D && td.host_data.is_null() && td.size_bytes > 0
                    && (td.gpu_addr as u64 + td.size_bytes as u64) <= self.vram_size
                {
                    let (w, h, d) = (td.width as usize, td.height as usize, td.depth as usize);
                    let pitch = td.lvl_pitch[0] as usize;
                    let face = pitch * td.lvl_rows[0] as usize;
                    line += &format!("    3D tex unit {}: {}x{}x{} pitch={} face={} kind={} bytes at {:#x}\n", t, w, h, d, pitch, face, td.kind, td.gpu_addr);
                    for z in 0..d.min(4) {
                        let base = unsafe { self.vram.add(td.gpu_addr as usize + z * face) };
                        let mut mn = 255u8; let mut mx = 0u8; let mut sum = 0u64;
                        for y in 0..h { for x in 0..w { let b = unsafe { *base.add(y * pitch + x) }; mn = mn.min(b); mx = mx.max(b); sum += b as u64; } }
                        line += &format!("      slice {} min={} max={} mean={:.1}  rows0-3:", z, mn, mx, sum as f64 / (w * h) as f64);
                        for y in 0..4.min(h) {
                            line += " [";
                            for x in 0..16.min(w) { line += &format!("{:02x}", unsafe { *base.add(y * pitch + x) }); }
                            line += "]";
                        }
                        line += "\n";
                    }
                }
            }
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&*path) {
                let _ = f.write_all(line.as_bytes());
            }
        }
        if progs.insert(ph) {
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(format!("{path}.glsl")) {
                let _ = writeln!(f, "//////// prog={:016x} (first seen at draw #{})\n{}", ph, n, glsl);
            }
        }
    }
}
