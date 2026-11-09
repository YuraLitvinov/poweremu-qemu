//! Samplers, stand-in textures, and textures rebuilt on the CPU.

use std::ffi::c_void;

use ash::vk;

use crate::state::*;
use crate::sys::*;

extern "C" {
    fn free(p: *mut c_void);
}

impl Vk {
    fn wrap(&self, w: u32) -> vk::SamplerAddressMode {
        match w & 7 {
            0 => vk::SamplerAddressMode::REPEAT,
            1 => vk::SamplerAddressMode::MIRRORED_REPEAT,
            3 | 5 | 7 => {
                if self.mirror_clamp {
                    vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE
                } else {
                    vk::SamplerAddressMode::CLAMP_TO_EDGE
                }
            }
            // r300_tex adds the border colour itself
            6 => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            _ => vk::SamplerAddressMode::CLAMP_TO_EDGE,
        }
    }

    /// The sampler TX_FILTER0 describes, for a texture of `levels` levels.
    pub fn sampler(&mut self, f0: u32, levels: u32) -> vk::Sampler {
        let key = (f0 & 0xFF_FFFF) | (levels.min(15) << 24);
        if let Some(&s) = self.samplers.get(&key) {
            return s;
        }
        let mag = (f0 >> 9) & 3;
        let min = (f0 >> 11) & 3;
        let mip = (f0 >> 13) & 3;
        // PPCGPU_NO_MIPS=1: always sample level 0 (tells a broken mip chain from
        // a broken sampler or texture coordinates)
        let no_mips = {
            use std::sync::OnceLock;
            static ON: OnceLock<bool> = OnceLock::new();
            *ON.get_or_init(|| {
                let on = std::env::var("PPCGPU_NO_MIPS").map_or(false, |v| v == "1");
                if on {
                    eprintln!("ppc-mac-gpu vulkan: PPCGPU_NO_MIPS=1, sampling mip level 0 only");
                }
                on
            })
        };
        let mipped = levels > 1 && mip != 0 && !no_mips;
        let mut sci = vk::SamplerCreateInfo::builder()
            .mag_filter(if mag == 1 { vk::Filter::NEAREST } else { vk::Filter::LINEAR })
            .min_filter(if min == 1 { vk::Filter::NEAREST } else { vk::Filter::LINEAR })
            .mipmap_mode(if mip == 2 { vk::SamplerMipmapMode::LINEAR } else { vk::SamplerMipmapMode::NEAREST })
            .address_mode_u(self.wrap(f0))
            .address_mode_v(self.wrap(f0 >> 3))
            .address_mode_w(self.wrap(f0 >> 6))
            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK)
            .min_lod(if mipped { ((f0 >> 17) & 0xF).min(levels - 1) as f32 } else { 0.0 })
            .max_lod(if mipped { (levels - 1) as f32 } else { 0.0 });
        if self.aniso && (mag == 3 || min == 3) && ((f0 >> 21) & 7) != 0 {
            sci = sci
                .anisotropy_enable(true)
                .max_anisotropy(((1u32 << ((f0 >> 21) & 7).min(4)) as f32).min(self.props.limits.max_sampler_anisotropy));
        }
        match unsafe { self.dev.create_sampler(&sci, None) } {
            Ok(s) => {
                self.samplers.insert(key, s);
                s
            }
            Err(_) => vk::Sampler::null(),
        }
    }

    /// 1x1 stand-ins for unbound units: 2D, 3D, cube.
    pub fn dummies(&mut self) -> bool {
        if self.dummy_ready {
            return true;
        }
        let it = [vk::ImageType::TYPE_2D, vk::ImageType::TYPE_3D, vk::ImageType::TYPE_2D];
        let vt = [vk::ImageViewType::TYPE_2D, vk::ImageViewType::TYPE_3D, vk::ImageViewType::CUBE];
        let cb = self.cb();
        for i in 0..3 {
            let layers = if i == 2 { 6 } else { 1 };
            let r = self.image(
                it[i],
                vt[i],
                vk::Format::R8G8B8A8_UNORM,
                1,
                1,
                1,
                1,
                layers,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            );
            let Ok(x) = r else {
                return false;
            };
            self.dummy[i] = x;
            self.layout_general(cb, x.0, 1, layers);
            let zero = vk::ClearColorValue { float32: [0.0; 4] };
            let sr = vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: layers,
            };
            unsafe { self.dev.cmd_clear_color_image(cb, x.0, vk::ImageLayout::GENERAL, &zero, &[sr]) };
        }
        self.full_barrier(cb);
        self.dummy_ready = true;
        true
    }

    pub fn tex_format(kind: u32) -> vk::Format {
        match kind {
            R300_TEXK_R8 => vk::Format::R8_UNORM,
            R300_TEXK_RG8 => vk::Format::R8G8_UNORM,
            R300_TEXK_DXT1 => vk::Format::BC1_RGBA_UNORM_BLOCK,
            R300_TEXK_DXT3 => vk::Format::BC2_UNORM_BLOCK,
            R300_TEXK_DXT5 => vk::Format::BC3_UNORM_BLOCK,
            _ => vk::Format::R8G8B8A8_UNORM,
        }
    }

    /// Textures rebuilt from their bytes on the CPU and cached by a hash of
    /// them: mip chains, 3D and cube maps, 16bpp formats, DXT, and anything
    /// copied out of the GART.  The caller has made sure no GPU work still
    /// writes the bytes.
    pub fn texture_full(&mut self, td: &R300TexDesc) -> Option<vk::ImageView> {
        let src: *const u8 = if !td.host_data.is_null() {
            td.host_data
        } else {
            unsafe { self.vram.add(td.gpu_addr as usize) }
        };
        let dxt = td.kind >= R300_TEXK_DXT1;
        let fmt = Self::tex_format(td.kind);
        let key = TexKey {
            addr: td.gpu_addr,
            format: td.format,
            kind: td.kind,
            width: td.width,
            height: td.height,
            depth: td.depth,
            dim: td.dim,
            levels: td.levels,
            pitch: td.pitch_bytes,
            host: if !td.host_data.is_null() { 1 + (td.endian << 1) } else { td.endian << 1 },
            hash: unsafe { r300_hash_tex(src, td.size_bytes as usize) },
        };
        if dxt && !self.bc {
            self.warn(2, "DXT texture on a GPU without BC formats");
            return None;
        }
        if td.dim == R300_TEXDIM_CUBE && td.width != td.height {
            self.warn(3, "cube map with non-square faces");
            return None;
        }
        let mut lru: Option<usize> = None;
        for i in 0..self.texfull.len() {
            let t = &self.texfull[i];
            if t.live && t.key == key {
                self.clock += 1;
                let c = self.clock;
                self.texfull[i].used = c;
                return Some(self.texfull[i].view);
            }
            if !t.live || lru.map_or(true, |l| self.texfull[l].live && t.used < self.texfull[l].used) {
                lru = Some(i);
            }
        }
        let slot = if self.texfull.len() < MAX_TEXFULL {
            self.texfull.push(TexFull {
                live: false,
                key,
                img: vk::Image::null(),
                mem: vk::DeviceMemory::null(),
                view: vk::ImageView::null(),
                used: 0,
            });
            self.texfull.len() - 1
        } else {
            lru?
        };
        if self.texfull[slot].live {
            let (v, i, m) = (self.texfull[slot].view, self.texfull[slot].img, self.texfull[slot].mem);
            self.trash(Trash::View(v));
            self.trash(Trash::Image(i));
            self.trash(Trash::Mem(m));
            self.texfull[slot].live = false;
        }
        let w = if dxt { (td.width + 3) & !3 } else { td.width };
        let h = if dxt { (td.height + 3) & !3 } else { td.height };
        let cube = td.dim == R300_TEXDIM_CUBE;
        let vol = td.dim == R300_TEXDIM_3D;
        let (img, mem, view) = self
            .image(
                if vol { vk::ImageType::TYPE_3D } else { vk::ImageType::TYPE_2D },
                if vol {
                    vk::ImageViewType::TYPE_3D
                } else if cube {
                    vk::ImageViewType::CUBE
                } else {
                    vk::ImageViewType::TYPE_2D
                },
                fmt,
                w,
                h,
                if vol { td.depth } else { 1 },
                td.levels,
                if cube { 6 } else { 1 },
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            )
            .map_err(|e| crate::state::fail(&e))
            .ok()?;
        self.end_pass();
        let cb = self.cb();
        self.layout_general(cb, img, td.levels, if cube { 6 } else { 1 });
        for l in 0..td.levels {
            let (mut lw, mut lh, mut n) = (0u32, 0u32, 0u32);
            unsafe { r300_tex_level_dims(td, l, &mut lw, &mut lh, &mut n) };
            let mw = (w >> l).max(1);
            let mh = (h >> l).max(1);
            let (cw, ch) = if dxt { (mw, mh) } else { (lw, lh) };
            let face = td.lvl_pitch[l as usize] as usize * td.lvl_rows[l as usize] as usize;
            for f in 0..n {
                let mut bpr = 0u32;
                let bytes = unsafe {
                    r300_tex_level_bytes(td, src.add(td.lvl_off[l as usize] as usize + f as usize * face), l, cw, ch, &mut bpr)
                };
                if bytes.is_null() {
                    continue;
                }
                let len = if dxt { bpr as usize * ((ch as usize + 3) / 4) } else { bpr as usize * ch as usize };
                let Some((sb, so, dst)) = self.arena(len as u64) else {
                    unsafe { free(bytes as *mut c_void) };
                    continue;
                };
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes, dst, len);
                    free(bytes as *mut c_void);
                }
                let r = vk::BufferImageCopy {
                    buffer_offset: so,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: l,
                        base_array_layer: if cube { f } else { 0 },
                        layer_count: 1,
                    },
                    image_offset: vk::Offset3D { x: 0, y: 0, z: if vol { f as i32 } else { 0 } },
                    image_extent: vk::Extent3D { width: cw, height: ch, depth: 1 },
                };
                let cb = self.cb();
                unsafe { self.dev.cmd_copy_buffer_to_image(cb, sb, img, vk::ImageLayout::GENERAL, &[r]) };
            }
        }
        let cb = self.cb();
        self.full_barrier(cb);
        self.clock += 1;
        self.texfull[slot] = TexFull { live: true, key, img, mem, view, used: self.clock };
        Some(view)
    }
}
