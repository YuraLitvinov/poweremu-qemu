# Dependency of ppc_mac_gpu_dev; it is not a device by itself.
_libr300_rs = cargo_ws.package('r300').library(
  structured_sources(
    [
      'src/lib.rs',
      'src/capi.rs',
      'src/cfmt.rs',
      'src/pvs.rs',
      'src/pvs_glsl.rs',
      'src/regnames.rs',
      'src/spirv.rs',
      'src/state.rs',
      'src/texel.rs',
      'src/types.rs',
      'src/us.rs',
      'src/us_text.rs',
    ],
    {
    'draw': [
      'src/draw/assemble.rs',
      'src/draw/build.rs',
      'src/draw/emit.rs',
      'src/draw/fetch.rs',
      'src/draw/gpu_vs.rs',
      'src/draw/layout.rs',
      'src/draw/misc.rs',
      'src/draw/mod.rs',
      'src/draw/pvsrec.rs',
      'src/draw/regs.rs',
      'src/draw/route.rs',
      'src/draw/setup.rs',
    ],
    },
  ),
)

cargo_ws.package('r300').override_dependency(declare_dependency(link_with: _libr300_rs))
