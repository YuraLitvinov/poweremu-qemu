project('shaderc-sys', 'rust', version: '0.10.1', meson_version: '>=1.5.0')

# The crate's build script compiles shaderc from source with CMake; QEMU
# links the system library instead (the C side of the device uses it too).
shaderc_dep = dependency('shaderc', method: 'pkg-config')

# `extern crate link_cplusplus;` only exists to link the C++ standard library.
link_cplusplus_rs = static_library('link_cplusplus',
  configure_file(output: 'link_cplusplus.rs', command: ['echo', '// intentionally empty'], capture: true),
  rust_abi: 'rust',
  link_args: ['-lstdc++'],
)

libshaderc_sys = static_library('shaderc_sys', 'src/lib.rs',
  rust_abi: 'rust',
  link_with: [link_cplusplus_rs],
  dependencies: [shaderc_dep, dependency('libc-0.2-rs').as_system()],
)

meson.override_dependency('shaderc-sys-0.10-rs',
  declare_dependency(link_with: libshaderc_sys, dependencies: [shaderc_dep]))
