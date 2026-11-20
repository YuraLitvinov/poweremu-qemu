//! R300 shader compilation: the GLSL from `r300_us_to_glsl()` to SPIR-V
//! (`r300_spirv.c`, the shaderc half), through the safe `shaderc` crate.
//!
//! The SPIR-V to MSL half of the C file serves the Metal backend and is not
//! ported.

use std::sync::Mutex;

use shaderc::{CompileOptions, Compiler, EnvVersion, ShaderKind, SpirvVersion, TargetEnv};

/// The stage of a GLSL source that holds all of them behind macros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// `R300_VS`: entry point `r300_vs`.
    Vs,
    /// `R300_FS`: `r300_fs`.
    Fs,
    /// `R300_FS_Z`: `r300_fs_z`.
    FsZ,
}

impl Stage {
    /// The stage's entry point name (also the file name in error messages).
    pub fn entry(self) -> &'static str {
        match self {
            Stage::Vs => "r300_vs",
            Stage::Fs => "r300_fs",
            Stage::FsZ => "r300_fs_z",
        }
    }
}

static COMPILER: Mutex<Option<Compiler>> = Mutex::new(None);

/// Compiles one stage of an `r300_us_to_glsl()` source to SPIR-V 1.3 for Vulkan
/// 1.1.  The error is the compiler's message.
pub fn glsl_to_spirv(glsl: &str, stage: Stage) -> Result<Vec<u32>, String> {
    let (kind, defines): (_, &[&str]) = match stage {
        Stage::Vs => (ShaderKind::Vertex, &["R300_VS"]),
        Stage::Fs => (ShaderKind::Fragment, &["R300_FS"]),
        // The Z-pass shader is the fragment shader plus the Z export.
        Stage::FsZ => (ShaderKind::Fragment, &["R300_FS_Z", "R300_FS"]),
    };
    let mut guard = COMPILER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_none() {
        *guard = Some(Compiler::new().map_err(|_| "shaderc: cannot initialise the compiler".to_string())?);
    }
    let compiler = guard.as_ref().expect("initialised above");
    let mut options = CompileOptions::new().map_err(|_| "shaderc: out of memory".to_string())?;
    options.set_target_env(TargetEnv::Vulkan, EnvVersion::Vulkan1_1 as u32);
    options.set_target_spirv(SpirvVersion::V1_3);
    for name in defines {
        options.add_macro_definition(name, Some("1"));
    }
    match compiler.compile_into_spirv(glsl, kind, stage.entry(), "main", Some(&options)) {
        Ok(artifact) => Ok(artifact.as_binary().to_vec()),
        Err(shaderc::Error::CompilationError(_, msg)) => Err(format!("GLSL: {msg}")),
        Err(e) => Err(format!("GLSL: {e}")),
    }
}
