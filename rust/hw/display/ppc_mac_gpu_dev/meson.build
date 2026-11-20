# The C types the Vulkan renderer shares with the device (renderer vtable,
# R300 draw packet, uniforms, ...) are generated from QEMU's own headers, so a
# change to them cannot silently shift the layout the Rust code relies on.
_libppc_mac_gpu_dev_bindings_inc_rs = rust.bindgen(
  input: 'wrapper.h',
  output: 'bindings.inc.rs',
  include_directories: include_directories('../../../../hw/display'),
  args: [
    '--disable-header-comment',
    '--raw-line', '// @generated',
    '--generate-block',
    '--impl-debug',
    '--no-doc-comments',
    '--no-layout-tests',
    '--no-prepend-enum-name',
    '--formatter', 'none',
    '--allowlist-type', 'PPCMacGPU.*',
    '--allowlist-type', 'R300.*',
    '--allowlist-type', 'R200.*',
    '--allowlist-function', 'r300_.*',
    '--allowlist-var', 'R300_.*',
    '--allowlist-var', 'PPC_MAC_GPU_.*',
  ],
)

_libppc_mac_gpu_dev_rs = cargo_ws.package('ppc_mac_gpu_dev').library(
  structured_sources(
    ['lib.rs'],
    {
    'src': [
      'src/global.rs',
      'src/logs.rs',
      'src/qemu.rs',
    ],
    'src/dev_ns': [
      'src/dev_ns/ffi.rs',
      'src/dev_ns/fn_2d.rs',
      'src/dev_ns/fn_blit.rs',
      'src/dev_ns/fn_frame.rs',
      'src/dev_ns/fn_gpu.rs',
      'src/dev_ns/fn_mc.rs',
      'src/dev_ns/fn_misc.rs',
      'src/dev_ns/fn_pe.rs',
      'src/dev_ns/fn_ppc_mac_gpu_1.rs',
      'src/dev_ns/fn_ppc_mac_gpu_2.rs',
      'src/dev_ns/fn_ppc_mac_gpu_3.rs',
      'src/dev_ns/fn_ppc_mac_gpu_4.rs',
      'src/dev_ns/fn_r200.rs',
      'src/dev_ns/fn_r300_1.rs',
      'src/dev_ns/fn_r300_2.rs',
      'src/dev_ns/fn_r300_render.rs',
      'src/dev_ns/fn_r300_trace.rs',
      'src/dev_ns/mod.rs',
      'src/dev_ns/statics.rs',
      'src/dev_ns/types_gpu.rs',
      'src/dev_ns/types_qemu_1.rs',
      'src/dev_ns/types_qemu_2.rs',
    ],
    'src/vulkan': [
      'src/vulkan/batch.rs',
      'src/vulkan/cache.rs',
      'src/vulkan/ctx.rs',
      'src/vulkan/draw.rs',
      'src/vulkan/mod.rs',
      'src/vulkan/pipe.rs',
      'src/vulkan/record.rs',
      'src/vulkan/state.rs',
      'src/vulkan/tex.rs',
      _libppc_mac_gpu_dev_bindings_inc_rs,
    ],
    },
  ),
)

rust_devices_ss.add(when: 'CONFIG_X_PPC_MAC_GPU_RUST', if_true: [declare_dependency(
  link_whole: [_libppc_mac_gpu_dev_rs],
  variables: {'crate': 'ppc_mac_gpu_dev'},
)])
