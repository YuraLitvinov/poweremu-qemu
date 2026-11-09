//! Render passes, framebuffers, shader modules and pipelines.

use std::ffi::{c_char, c_void, CStr};

use ash::vk;

use crate::state::*;
use crate::sys::*;

/// PPCGPU_NO_DEPTH_CLAMP=1: clip against the near and far planes (no depth clamp).
fn no_depth_clamp() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        let on = std::env::var("PPCGPU_NO_DEPTH_CLAMP").map_or(false, |v| v == "1");
        if on {
            eprintln!("ppc-mac-gpu vulkan: PPCGPU_NO_DEPTH_CLAMP=1, clipping against the near and far planes");
        }
        on
    })
}

pub const STAGE_VS: u32 = 0;
pub const STAGE_FS: u32 = 1;
pub const STAGE_FS_Z: u32 = 2;
/// the FS_Z program for a 16-bit depth buffer (R300_Z16 defined): storage-image
/// shaders name the buffer's format.  Compiled as STAGE_FS_Z.
pub const STAGE_FS_Z16: u32 = 3;

extern "C" {
    fn free(p: *mut c_void);
}

impl Vk {
    /// A render pass over `fmt` attachments (colour buffers, then Z), each
    /// both an output and an input attachment, kept in GENERAL layout.
    pub fn render_pass(&mut self, fmt: &[vk::Format]) -> Option<vk::RenderPass> {
        // With storage-image framebuffer access the pass has no attachments.
        let fmt: &[vk::Format] = if self.fb_image { &[] } else { fmt };
        if let Some(r) = self.rps.iter().find(|r| r.fmt == fmt) {
            return Some(r.rp);
        }
        let n = fmt.len();
        let ad: Vec<vk::AttachmentDescription> = fmt
            .iter()
            .map(|&f| {
                vk::AttachmentDescription::builder()
                    .format(f)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(vk::ImageLayout::GENERAL)
                    .final_layout(vk::ImageLayout::GENERAL)
                    .build()
            })
            .collect();
        let refs: Vec<vk::AttachmentReference> =
            (0..n as u32).map(|k| vk::AttachmentReference { attachment: k, layout: vk::ImageLayout::GENERAL }).collect();
        let sd = [vk::SubpassDescription::builder()
            .flags(if self.ordered {
                vk::SubpassDescriptionFlags::RASTERIZATION_ORDER_ATTACHMENT_COLOR_ACCESS_EXT
            } else {
                vk::SubpassDescriptionFlags::empty()
            })
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .input_attachments(&refs)
            .color_attachments(&refs)
            .build()];
        let dep = [
            // between draws: writes before the next draw's input reads
            vk::SubpassDependency::builder()
                .src_subpass(0)
                .dst_subpass(0)
                .src_stage_mask(if self.fb_image {
                    vk::PipelineStageFlags::FRAGMENT_SHADER
                } else {
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                })
                .dst_stage_mask(vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(if self.fb_image {
                    vk::AccessFlags::SHADER_WRITE
                } else {
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                })
                .dst_access_mask(
                    vk::AccessFlags::INPUT_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                        | vk::AccessFlags::SHADER_READ
                        | vk::AccessFlags::SHADER_WRITE,
                )
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build(),
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .dst_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
                .build(),
            vk::SubpassDependency::builder()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .dst_stage_mask(vk::PipelineStageFlags::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
                .build(),
        ];
        if self.rps.len() >= MAX_RP {
            self.warn(0, "too many render pass variants");
            return None;
        }
        let rci = vk::RenderPassCreateInfo::builder().attachments(&ad).subpasses(&sd).dependencies(&dep);
        match unsafe { self.dev.create_render_pass(&rci, None) } {
            Ok(rp) => {
                self.rps.push(Rp { fmt: fmt.to_vec(), rp });
                Some(rp)
            }
            Err(_) => {
                self.warn(0, "render pass creation failed");
                None
            }
        }
    }

    pub fn framebuffer(&mut self, rp: vk::RenderPass, att: &[usize], w: u32, h: u32) -> Option<vk::Framebuffer> {
        let att: &[usize] = if self.fb_image { &[] } else { att };
        let n = att.len();
        let mut views = [vk::ImageView::null(); MAX_ATT];
        for (k, &a) in att.iter().enumerate() {
            views[k] = self.img[a].view;
        }
        for f in self.fbs.iter().flatten() {
            if f.rp == rp && f.n == n && f.w == w && f.h == h && f.view[..n] == views[..n] {
                return Some(f.fb);
            }
        }
        let slot = self.fb_next;
        self.fb_next = (self.fb_next + 1) % MAX_FB;
        if let Some(old) = self.fbs[slot].take() {
            self.trash(Trash::Fb(old.fb));
        }
        let fci = vk::FramebufferCreateInfo::builder()
            .render_pass(rp)
            .attachments(&views[..n])
            .width(w)
            .height(h)
            .layers(1);
        let fb = unsafe { self.dev.create_framebuffer(&fci, None) }.ok()?;
        self.fbs[slot] = Some(Fb { rp, n, w, h, view: views, fb });
        Some(fb)
    }

    /// The generated GLSL with the fragment stage's attachment read-modify-write
    /// bracketed by a pixel interlock, so overlapping triangles of one draw are
    /// shaded in order (Metal's framebuffer fetch gives that for free).
    fn with_interlock(src: &str) -> Option<String> {
        let ver = src.find("#version")?;
        let eol = ver + src[ver..].find('\n')? + 1;
        let main_at = src.rfind("void main()\n{\n")? + "void main()\n{\n".len();
        let close = src.rfind("}\n#endif")?;
        if close < main_at {
            return None;
        }
        let mut out = String::with_capacity(src.len() + 256);
        out.push_str(&src[..eol]);
        out.push_str(
            "#extension GL_ARB_fragment_shader_interlock : enable\n\
             #if defined(R300_FS) || defined(R300_FS_Z)\n\
             layout(pixel_interlock_ordered) in;\n\
             #endif\n",
        );
        out.push_str(&src[eol..main_at]);
        out.push_str("    beginInvocationInterlockARB();\n");
        // The shader marks where the critical section ends: before its final
        // `if (dead) discard;`, so no invocation leaves it early.
        const MARK: &str = "/*R300_END*/";
        match src[main_at..close].find(MARK) {
            Some(m) => {
                out.push_str(&src[main_at..main_at + m]);
                out.push_str("endInvocationInterlockARB();");
                out.push_str(&src[main_at + m + MARK.len()..]);
            }
            None => {
                out.push_str(&src[main_at..close]);
                out.push_str("    endInvocationInterlockARB();\n");
                out.push_str(&src[close..]);
            }
        }
        Some(out)
    }

    fn compile(&mut self, glsl: &CStr, stage: u32) -> (Option<vk::ShaderModule>, String) {
        let mut nw: usize = 0;
        let mut err: *mut c_char = std::ptr::null_mut();
        let spv = unsafe { r300_glsl_to_spirv(glsl.as_ptr(), stage as _, &mut nw, &mut err) };
        let mut module = None;
        let mut msg = String::new();
        if spv.is_null() {
            if !err.is_null() {
                msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
            }
        } else {
            let words = unsafe { std::slice::from_raw_parts(spv, nw) };
            let mci = vk::ShaderModuleCreateInfo::builder().code(words);
            module = unsafe { self.dev.create_shader_module(&mci, None) }.ok();
        }
        unsafe {
            if !spv.is_null() {
                free(spv as *mut c_void);
            }
            if !err.is_null() {
                free(err as *mut c_void);
            }
        }
        (module, msg)
    }

    fn module(&mut self, key: &ProgKey, glsl: &CStr, stage: u32) -> Option<vk::ShaderModule> {
        {
            let pg = self.progs.get(key)?;
            if pg.mods[stage as usize] != vk::ShaderModule::null() || pg.failed[stage as usize] {
                let m = pg.mods[stage as usize];
                return if m == vk::ShaderModule::null() { None } else { Some(m) };
            }
        }
        let mut module = None;
        let mut text = glsl.to_string_lossy().into_owned();
        let cstage = if stage == STAGE_FS_Z16 { STAGE_FS_Z } else { stage };
        if stage == STAGE_FS_Z16 {
            if let Some(v) = text.find("#version") {
                if let Some(nl) = text[v..].find('\n') {
                    text.insert_str(v + nl + 1, "#define R300_Z16 1\n");
                }
            }
        }
        if self.interlock && stage != STAGE_VS {
            if let Some(w) = Self::with_interlock(&text) {
                if let Ok(cs) = std::ffi::CString::new(w.clone()) {
                    let (m, e) = self.compile(&cs, cstage);
                    if m.is_some() {
                        module = m;
                    } else {
                        eprintln!("ppc-mac-gpu vulkan: shader stage {stage} with pixel interlock: {e}\n{w}");
                    }
                }
            }
        }
        if module.is_none() {
            let cs = std::ffi::CString::new(text.clone()).unwrap_or_else(|_| glsl.to_owned());
            let (m, e) = self.compile(&cs, cstage);
            if m.is_none() {
                eprintln!("ppc-mac-gpu vulkan: shader stage {stage}: {e}\n{}", text);
            }
            module = m;
        }
        let _ = &mut text;
        let pg = self.progs.get_mut(key)?;
        match module {
            Some(m) => pg.mods[stage as usize] = m,
            None => pg.failed[stage as usize] = true,
        }
        module
    }

    pub fn pipeline(&mut self, key: &ProgKey, glsl: &CStr, pk: &PipeKey) -> Option<vk::Pipeline> {
        if !self.progs.contains_key(key) {
            self.progs.insert(key.clone(), Prog::default());
            if self.progs.len() % 16 == 1 {
                eprintln!("ppc-mac-gpu vulkan: {} fragment programs", self.progs.len());
            }
        }
        if let Some((_, p)) = self.progs[key].pipes.iter().find(|(k, _)| k == pk) {
            return Some(*p);
        }
        let vs = self.module(key, glsl, STAGE_VS)?;
        let fs = self.module(key, glsl, if !pk.z { STAGE_FS } else if pk.z16 { STAGE_FS_Z16 } else { STAGE_FS_Z })?;
        let name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let st = [
            vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vs).name(name).build(),
            vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(fs).name(name).build(),
        ];
        let vi = vk::PipelineVertexInputStateCreateInfo::builder();
        let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::from_raw(pk.topo));
        let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
        let rs = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(self.depth_clamp && !no_depth_clamp())
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::from_raw(pk.cull))
            .front_face(vk::FrontFace::from_raw(pk.front))
            .line_width(1.0);
        let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let natt = if self.fb_image { 0 } else { pk.ncb as usize + usize::from(pk.z) };
        let cba: Vec<vk::PipelineColorBlendAttachmentState> = (0..natt)
            .map(|_| {
                vk::PipelineColorBlendAttachmentState::builder()
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .build()
            })
            .collect();
        let cb = vk::PipelineColorBlendStateCreateInfo::builder()
            .flags(if self.ordered {
                vk::PipelineColorBlendStateCreateFlags::RASTERIZATION_ORDER_ATTACHMENT_ACCESS_EXT
            } else {
                vk::PipelineColorBlendStateCreateFlags::empty()
            })
            .attachments(&cba);
        let dyns = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let ds = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyns);
        let gci = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&st)
            .vertex_input_state(&vi)
            .input_assembly_state(&ia)
            .viewport_state(&vp)
            .rasterization_state(&rs)
            .multisample_state(&ms)
            .color_blend_state(&cb)
            .dynamic_state(&ds)
            .layout(self.layout)
            .render_pass(pk.rp)
            .subpass(0)
            .build()];
        let pipe = match unsafe { self.dev.create_graphics_pipelines(vk::PipelineCache::null(), &gci, None) } {
            Ok(p) => p[0],
            Err(_) => {
                self.warn(1, "pipeline creation failed");
                return None;
            }
        };
        self.progs.get_mut(key)?.pipes.push((*pk, pipe));
        Some(pipe)
    }
}
