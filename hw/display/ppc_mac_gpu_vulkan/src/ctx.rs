//! Instance, device, memory helpers and the VRAM buffer.

use std::collections::{HashMap, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Condvar, Mutex};

use ash::vk;

use crate::state::*;
use crate::sys::*;

pub type Res<T> = Result<T, String>;

pub fn vkerr<T>(r: ash::prelude::VkResult<T>, what: &str) -> Res<T> {
    r.map_err(|e| format!("{what} failed ({e:?})"))
}

impl Vk {
    pub fn create() -> Res<&'static mut Vk> {
        unsafe { Self::create_inner() }
    }

    unsafe fn create_inner() -> Res<&'static mut Vk> {
        let entry = ash::Entry::load().map_err(|e| format!("no Vulkan loader or driver ({e})"))?;

        let mut iext: Vec<*const i8> = Vec::new();
        let mut portability = false;
        let exts = vkerr(entry.enumerate_instance_extension_properties(None), "vkEnumerateInstanceExtensionProperties")?;
        let portability_name = vk::KhrPortabilityEnumerationFn::name();
        if exts.iter().any(|e| CStr::from_ptr(e.extension_name.as_ptr()) == portability_name) {
            iext.push(portability_name.as_ptr());
            portability = true;
        }

        // Ask for 1.3 where the loader has it: MoltenVK reports a device
        // only as new as the version asked for.
        let iver = entry
            .try_enumerate_instance_version()
            .ok()
            .flatten()
            .unwrap_or(vk::API_VERSION_1_0);
        let api = if iver >= vk::API_VERSION_1_1 { vk::API_VERSION_1_3 } else { vk::API_VERSION_1_0 };
        let app = vk::ApplicationInfo::builder()
            .application_name(CStr::from_bytes_with_nul_unchecked(b"qemu ati-radeon-9700\0"))
            .api_version(api);
        let ici = vk::InstanceCreateInfo::builder()
            .application_info(&app)
            .enabled_extension_names(&iext)
            .flags(if portability {
                vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                vk::InstanceCreateFlags::empty()
            });
        let inst = vkerr(entry.create_instance(&ici, None), "vkCreateInstance")?;

        let pds = vkerr(inst.enumerate_physical_devices(), "vkEnumeratePhysicalDevices")?;
        if pds.is_empty() {
            return Err("no Vulkan device".into());
        }
        // PPCGPU_VK_DEVICE=index picks one; else discrete, then integrated.
        let want_idx: Option<usize> = std::env::var("PPCGPU_VK_DEVICE").ok().and_then(|s| s.parse().ok());
        let mut best: Option<(usize, i32)> = None;
        for (i, &pd) in pds.iter().enumerate() {
            let p = inst.get_physical_device_properties(pd);
            let mut sc = match p.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 3,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 2,
                _ => 1,
            };
            if p.api_version < vk::API_VERSION_1_1 {
                continue;
            }
            if want_idx == Some(i) {
                sc = 100;
            }
            if best.map_or(true, |(_, b)| sc > b) {
                best = Some((i, sc));
            }
        }
        let pdev = pds[best.ok_or("no Vulkan 1.1 device")?.0];
        let props = inst.get_physical_device_properties(pdev);
        let memprops = inst.get_physical_device_memory_properties(pdev);

        let dexts = vkerr(inst.enumerate_device_extension_properties(pdev), "vkEnumerateDeviceExtensionProperties")?;
        let has_dext = |n: &CStr| dexts.iter().any(|e| CStr::from_ptr(e.extension_name.as_ptr()) == n);
        let mut dext: Vec<*const i8> = Vec::new();
        if has_dext(vk::KhrPortabilitySubsetFn::name()) {
            dext.push(vk::KhrPortabilitySubsetFn::name().as_ptr());
        }
        let mirror_ext = props.api_version < vk::API_VERSION_1_2
            && has_dext(vk::KhrSamplerMirrorClampToEdgeFn::name());
        if mirror_ext {
            dext.push(vk::KhrSamplerMirrorClampToEdgeFn::name().as_ptr());
        }

        // Ordered attachment read-modify-write within a draw (see Vk::ordered).
        // PPCGPU_NO_ORDER=1 leaves it off, to compare.
        let has_roa = has_dext(vk::ExtRasterizationOrderAttachmentAccessFn::name())
            && std::env::var("PPCGPU_NO_ORDER").map_or(true, |v| v != "1");
        let mut roa = vk::PhysicalDeviceRasterizationOrderAttachmentAccessFeaturesEXT::default();
        if has_roa {
            let mut f2 = vk::PhysicalDeviceFeatures2::builder().push_next(&mut roa);
            inst.get_physical_device_features2(pdev, &mut f2);
        }
        let ordered = has_roa && roa.rasterization_order_color_attachment_access != 0;
        if ordered {
            dext.push(vk::ExtRasterizationOrderAttachmentAccessFn::name().as_ptr());
        }
        let has_il = !ordered
            && has_dext(vk::ExtFragmentShaderInterlockFn::name())
            && std::env::var("PPCGPU_NO_ORDER").map_or(true, |v| v != "1");
        let mut il = vk::PhysicalDeviceFragmentShaderInterlockFeaturesEXT::default();
        if has_il {
            let mut f2 = vk::PhysicalDeviceFeatures2::builder().push_next(&mut il);
            inst.get_physical_device_features2(pdev, &mut f2);
        }
        let interlock = has_il && il.fragment_shader_pixel_interlock != 0;
        if interlock {
            dext.push(vk::ExtFragmentShaderInterlockFn::name().as_ptr());
        }
        eprintln!(
            "ppc-mac-gpu vulkan: attachment ordering: {}",
            if ordered {
                "rasterization order attachment access"
            } else if interlock {
                "fragment shader pixel interlock"
            } else {
                "NONE (overlapping triangles of one draw may blend out of order)"
            }
        );

        let feat = inst.get_physical_device_features(pdev);
        // Coherent storage images inside the interlock, where only interlock orders
        // the fragments (RADV): PPCGPU_NO_FBIMAGE=1 keeps the input attachments.
        let fb_image = interlock
            && !ordered
            && feat.shader_storage_image_extended_formats != 0
            && std::env::var("PPCGPU_NO_FBIMAGE").map_or(true, |v| v != "1");
        eprintln!("ppc-mac-gpu vulkan: framebuffer access: {}", if fb_image { "storage images (coherent, in the interlock)" } else { "input attachments" });
        if feat.fragment_stores_and_atomics == 0 {
            return Err(format!("{} lacks fragmentStoresAndAtomics", CStr::from_ptr(props.device_name.as_ptr()).to_string_lossy()));
        }
        let mut f12 = vk::PhysicalDeviceVulkan12Features::default();
        if props.api_version >= vk::API_VERSION_1_2 {
            let mut f2 = vk::PhysicalDeviceFeatures2::builder().push_next(&mut f12);
            inst.get_physical_device_features2(pdev, &mut f2);
        }
        let want = vk::PhysicalDeviceFeatures::builder()
            .fragment_stores_and_atomics(true)
            .shader_storage_image_extended_formats(fb_image)
            .depth_clamp(feat.depth_clamp != 0)
            .shader_clip_distance(feat.shader_clip_distance != 0)
            .sampler_anisotropy(feat.sampler_anisotropy != 0)
            .texture_compression_bc(feat.texture_compression_bc != 0)
            .build();
        let mut w12 = vk::PhysicalDeviceVulkan12Features::builder()
            .sampler_mirror_clamp_to_edge(f12.sampler_mirror_clamp_to_edge != 0)
            .build();

        let qf = inst.get_physical_device_queue_family_properties(pdev);
        let qfam = qf
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .ok_or("no graphics queue")? as u32;
        let prio = [1.0f32];
        let qci = [vk::DeviceQueueCreateInfo::builder().queue_family_index(qfam).queue_priorities(&prio).build()];
        let mut dci = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&qci)
            .enabled_extension_names(&dext)
            .enabled_features(&want);
        let mut roa_on = vk::PhysicalDeviceRasterizationOrderAttachmentAccessFeaturesEXT::builder()
            .rasterization_order_color_attachment_access(true)
            .build();
        if props.api_version >= vk::API_VERSION_1_2 {
            dci = dci.push_next(&mut w12);
        }
        let mut il_on = vk::PhysicalDeviceFragmentShaderInterlockFeaturesEXT::builder()
            .fragment_shader_pixel_interlock(true)
            .build();
        if ordered {
            dci = dci.push_next(&mut roa_on);
        }
        if interlock {
            dci = dci.push_next(&mut il_on);
        }
        let dev = vkerr(inst.create_device(pdev, &dci, None), "vkCreateDevice")?;
        let queue = dev.get_device_queue(qfam, 0);

        let lim = &props.limits;
        let align = lim
            .min_uniform_buffer_offset_alignment
            .max(lim.min_storage_buffer_offset_alignment)
            .max(256);

        let cmdpool = vkerr(
            dev.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(qfam),
                None,
            ),
            "vkCreateCommandPool",
        )?;

        // One descriptor set layout for every R300 program (R300_BIND_*).
        let all = vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT;
        let frag = vk::ShaderStageFlags::FRAGMENT;
        let bind = |b: u32, t: vk::DescriptorType, n: u32, s: vk::ShaderStageFlags| {
            vk::DescriptorSetLayoutBinding::builder().binding(b).descriptor_type(t).descriptor_count(n).stage_flags(s).build()
        };
        let mut b = vec![
            bind(R300_BIND_UNIFORMS, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, 1, all),
            bind(R300_BIND_ZPASS, vk::DescriptorType::STORAGE_BUFFER, 1, all),
            bind(R300_BIND_VERTS, vk::DescriptorType::STORAGE_BUFFER, 1, all),
            bind(R300_BIND_MS, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, 1, all),
            bind(R300_BIND_VRAM, vk::DescriptorType::STORAGE_BUFFER, 1, all),
            bind(R300_BIND_AUX, vk::DescriptorType::STORAGE_BUFFER, 1, all),
        ];
        for k in 0..MAX_ATT as u32 {
            b.push(bind(
                R300_BIND_FB0 + k,
                if fb_image { vk::DescriptorType::STORAGE_IMAGE } else { vk::DescriptorType::INPUT_ATTACHMENT },
                1,
                frag,
            ));
        }
        for k in 0..NTEX as u32 {
            b.push(bind(R300_BIND_TEX0 + k, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1, frag));
        }
        let dsl = vkerr(
            dev.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo::builder().bindings(&b), None),
            "vkCreateDescriptorSetLayout",
        )?;
        let layouts = [dsl];
        let layout = vkerr(
            dev.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::builder().set_layouts(&layouts), None),
            "vkCreatePipelineLayout",
        )?;

        let mut batch = Vec::with_capacity(NBATCH);
        for _ in 0..NBATCH {
            let cb = vkerr(
                dev.allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::builder()
                        .command_pool(cmdpool)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .command_buffer_count(1),
                ),
                "vkAllocateCommandBuffers",
            )?[0];
            let fence = vkerr(dev.create_fence(&vk::FenceCreateInfo::default(), None), "vkCreateFence")?;
            batch.push(Batch {
                state: 0,
                seq: 0,
                cb,
                fence,
                pools: Vec::new(),
                pool_cur: 0,
                chunks: Vec::new(),
                trash: Vec::new(),
            });
        }

        let shared = Arc::new(Shared {
            q: Mutex::new(VecDeque::new()),
            wake: Condvar::new(),
            idle: Condvar::new(),
            done_seq: AtomicU32::new(0),
            worker_seq: AtomicU32::new(0),
        });
        {
            let (d, s) = (dev.clone(), shared.clone());
            std::thread::Builder::new()
                .name("vk-wait".into())
                .spawn(move || crate::batch::waiter(d, s))
                .map_err(|e| format!("cannot start the completion thread: {e}"))?;
        }

        eprintln!(
            "ppc-mac-gpu vulkan: {} (Vulkan {}.{})",
            CStr::from_ptr(props.device_name.as_ptr()).to_string_lossy(),
            vk::api_version_major(props.api_version),
            vk::api_version_minor(props.api_version)
        );

        let v = Vk {
            _entry: entry,
            inst,
            pdev,
            props,
            memprops,
            dev,
            queue,
            depth_clamp: want.depth_clamp != 0,
            aniso: want.sampler_anisotropy != 0,
            bc: want.texture_compression_bc != 0,
            mirror_clamp: mirror_ext || f12.sampler_mirror_clamp_to_edge != 0,
            ordered,
            interlock,
            fb_image,
            align,
            cmdpool,
            dsl,
            layout,
            vram: std::ptr::null_mut(),
            vram_size: 0,
            vram_mem: vk::DeviceMemory::null(),
            vram_buf: vk::Buffer::null(),
            npages: 0,
            page_gen: Vec::new(),
            page_wseq: Vec::new(),
            page_rseq: Vec::new(),
            dirty_bm: Vec::new(),
            gen: 0,
            dirty_fn: None,
            dirty_arg: std::ptr::null_mut(),
            zpass_buf: vk::Buffer::null(),
            zpass_mem: vk::DeviceMemory::null(),
            zpass: std::ptr::null_mut(),
            batch,
            cur: None,
            seq: 0,
            free_chunks: Vec::new(),
            chunk: None,
            shared,
            in_pass: false,
            pass_rp: vk::RenderPass::null(),
            pass_n: 0,
            pass_img: [0; MAX_ATT],
            pass_z: false,
            pass_z16: false,
            img: (0..MAX_IMG).map(|_| Img::empty()).collect(),
            texfull: Vec::new(),
            clock: 0,
            rps: Vec::new(),
            fbs: (0..MAX_FB).map(|_| None).collect(),
            fb_next: 0,
            progs: HashMap::new(),
            samplers: HashMap::new(),
            dummy: [(vk::Image::null(), vk::DeviceMemory::null(), vk::ImageView::null()); 3],
            dummy_ready: false,
            warned: 0,
            dump: std::env::var("PPCGPU_DUMP").ok().filter(|p| !p.is_empty()).map(|p| {
                eprintln!("ppc-mac-gpu vulkan: PPCGPU_DUMP={p}: writing distinct draws to it (and GLSL to {p}.glsl)");
                let _ = std::fs::write(&p, "");
                let _ = std::fs::write(format!("{p}.glsl"), "");
                (p, Default::default(), Default::default())
            }),
            dump_draws: 0,
            rec: crate::record::Rec::from_env(),
            dump_stats: Default::default(),
            stat_draws: 0,
            stat_passes: 0,
            stat_uploads: 0,
            stat_writebacks: 0,
            stat_flushes: 0,
        };
        Ok(set_vk(v))
    }

    pub fn warn(&mut self, bit: u32, msg: &str) {
        if self.warned & (1 << bit) == 0 {
            self.warned |= 1 << bit;
            eprintln!("ppc-mac-gpu vulkan: {msg}");
        }
    }

    pub fn memtype(&self, bits: u32, want: vk::MemoryPropertyFlags) -> Option<u32> {
        (0..self.memprops.memory_type_count).find(|&i| {
            bits & (1 << i) != 0 && self.memprops.memory_types[i as usize].property_flags.contains(want)
        })
    }

    /// A buffer in host-visible memory, mapped.
    pub fn buffer(
        &self,
        size: u64,
        usage: vk::BufferUsageFlags,
        want: vk::MemoryPropertyFlags,
        prefer: vk::MemoryPropertyFlags,
    ) -> Res<(vk::Buffer, vk::DeviceMemory, *mut u8)> {
        unsafe {
            let buf = vkerr(
                self.dev.create_buffer(&vk::BufferCreateInfo::builder().size(size).usage(usage), None),
                "vkCreateBuffer",
            )?;
            let req = self.dev.get_buffer_memory_requirements(buf);
            let mt = self
                .memtype(req.memory_type_bits, want | prefer)
                .or_else(|| self.memtype(req.memory_type_bits, want));
            let Some(mt) = mt else {
                self.dev.destroy_buffer(buf, None);
                return Err(format!("no memory type for a {size}-byte buffer"));
            };
            let mem = match self.dev.allocate_memory(
                &vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(mt),
                None,
            ) {
                Ok(m) => m,
                Err(e) => {
                    self.dev.destroy_buffer(buf, None);
                    return Err(format!("vkAllocateMemory failed ({e:?})"));
                }
            };
            let r = self
                .dev
                .bind_buffer_memory(buf, mem, 0)
                .and_then(|_| self.dev.map_memory(mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()));
            match r {
                Ok(p) => Ok((buf, mem, p as *mut u8)),
                Err(e) => {
                    self.dev.destroy_buffer(buf, None);
                    self.dev.free_memory(mem, None);
                    Err(format!("binding or mapping a buffer failed ({e:?})"))
                }
            }
        }
    }

    /// A device-local image and a view of all of it.
    #[allow(clippy::too_many_arguments)]
    pub fn image(
        &self,
        itype: vk::ImageType,
        vtype: vk::ImageViewType,
        fmt: vk::Format,
        w: u32,
        h: u32,
        d: u32,
        levels: u32,
        layers: u32,
        usage: vk::ImageUsageFlags,
    ) -> Res<(vk::Image, vk::DeviceMemory, vk::ImageView)> {
        unsafe {
            let ici = vk::ImageCreateInfo::builder()
                .flags(if vtype == vk::ImageViewType::CUBE {
                    vk::ImageCreateFlags::CUBE_COMPATIBLE
                } else {
                    vk::ImageCreateFlags::empty()
                })
                .image_type(itype)
                .format(fmt)
                .extent(vk::Extent3D { width: w, height: h, depth: d })
                .mip_levels(levels)
                .array_layers(layers)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(usage)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            let img = vkerr(self.dev.create_image(&ici, None), "vkCreateImage")?;
            let req = self.dev.get_image_memory_requirements(img);
            let mt = self
                .memtype(req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                .or_else(|| self.memtype(req.memory_type_bits, vk::MemoryPropertyFlags::empty()));
            let Some(mt) = mt else {
                self.dev.destroy_image(img, None);
                return Err("no memory type for an image".into());
            };
            let mem = match self.dev.allocate_memory(
                &vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(mt),
                None,
            ) {
                Ok(m) => m,
                Err(e) => {
                    self.dev.destroy_image(img, None);
                    return Err(format!("vkAllocateMemory (image) failed ({e:?})"));
                }
            };
            let vci = vk::ImageViewCreateInfo::builder().image(img).view_type(vtype).format(fmt).subresource_range(
                vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: levels,
                    base_array_layer: 0,
                    layer_count: layers,
                },
            );
            let r = self.dev.bind_image_memory(img, mem, 0).and_then(|_| self.dev.create_image_view(&vci, None));
            match r {
                Ok(view) => Ok((img, mem, view)),
                Err(e) => {
                    self.dev.destroy_image(img, None);
                    self.dev.free_memory(mem, None);
                    Err(format!("binding an image or creating its view failed ({e:?})"))
                }
            }
        }
    }

    /// The guest's VRAM, as host-visible Vulkan memory: the CPU's copy.
    /// The GPU only copies from and to it, and reads shader-decoded
    /// textures, so it need not be device-local.  CPU-cached where there is
    /// a choice: the guest reads VRAM a lot, and write-combined memory
    /// reads slowly.
    pub fn alloc_vram(&mut self, size: u64) -> Res<*mut u8> {
        if !self.vram.is_null() {
            return Err("VRAM already allocated (one Vulkan-rendered card only)".into());
        }
        let (buf, mem, map) = self.buffer(
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            vk::MemoryPropertyFlags::HOST_CACHED,
        )?;
        let page = unsafe { libc_page_size() };
        if map as usize % page != 0 {
            unsafe {
                self.dev.destroy_buffer(buf, None);
                self.dev.free_memory(mem, None);
            }
            return Err("mapped VRAM is not page aligned".into());
        }
        unsafe { std::ptr::write_bytes(map, 0, size as usize) };
        self.vram = map;
        self.vram_size = size;
        self.vram_buf = buf;
        self.vram_mem = mem;
        self.npages = size.div_ceil(PAGE);
        self.page_gen = vec![0; self.npages as usize];
        self.page_wseq = vec![0; self.npages as usize];
        self.page_rseq = vec![0; self.npages as usize];
        self.dirty_bm = vec![0; (self.npages as usize).div_ceil(64)];
        Ok(map)
    }
}

unsafe fn libc_page_size() -> usize {
    extern "C" {
        fn sysconf(name: i32) -> i64;
    }
    const SC_PAGESIZE: i32 = if cfg!(target_os = "macos") { 29 } else { 30 };
    let p = sysconf(SC_PAGESIZE);
    if p > 0 {
        p as usize
    } else {
        4096
    }
}

#[allow(dead_code)]
pub fn unused(_: *mut c_void) {}
