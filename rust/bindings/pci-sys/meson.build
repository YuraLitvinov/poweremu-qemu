_bindgen_pci_rs = rust.bindgen(
  args: bindgen_args_common + bindgen_args_data['pci-sys'].split(),
  kwargs: bindgen_kwargs)

_pci_sys_rs = cargo_ws.package('pci-sys').library(
  structured_sources(['lib.rs', _bindgen_pci_rs]))
cargo_ws.package('pci-sys').override_dependency(declare_dependency(link_with: _pci_sys_rs))

pci_sys_rs = declare_dependency(link_with: [_pci_sys_rs])
