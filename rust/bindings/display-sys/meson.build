_bindgen_display_rs = rust.bindgen(
  args: bindgen_args_common + bindgen_args_data['display-sys'].split(),
  # ui/console.h includes pixman.h, which is not among the common dependencies.
  kwargs: bindgen_kwargs + {'dependencies': bindgen_kwargs['dependencies'] + [pixman]})

_display_sys_rs = cargo_ws.package('display-sys').library(
  structured_sources(['lib.rs', _bindgen_display_rs]))
cargo_ws.package('display-sys').override_dependency(declare_dependency(link_with: _display_sys_rs))

display_sys_rs = declare_dependency(link_with: [_display_sys_rs])
