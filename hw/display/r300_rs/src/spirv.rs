//! R300 shader compilation: the GLSL from `r300_us_to_glsl()` to SPIR-V
//! (`r300_spirv.c`, the shaderc half).
//!
//! The SPIR-V to MSL half of the C file serves the Metal backend and is not
//! ported.

use crate::ffi::{compile_glsl, ShaderKind};

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

    fn entry_c(self) -> &'static core::ffi::CStr {
        match self {
            Stage::Vs => c"r300_vs",
            Stage::Fs => c"r300_fs",
            Stage::FsZ => c"r300_fs_z",
        }
    }
}

/// Compiles one stage of an `r300_us_to_glsl()` source to SPIR-V.  The error
/// is the compiler's message.
pub fn glsl_to_spirv(glsl: &str, stage: Stage) -> Result<Vec<u32>, String> {
    let (kind, defines): (_, &[&str]) = match stage {
        Stage::Vs => (ShaderKind::Vertex, &["R300_VS"]),
        Stage::Fs => (ShaderKind::Fragment, &["R300_FS"]),
        // The Z-pass shader is the fragment shader plus the Z export.
        Stage::FsZ => (ShaderKind::Fragment, &["R300_FS_Z", "R300_FS"]),
    };
    compile_glsl(glsl, kind, stage.entry_c(), defines)
}
