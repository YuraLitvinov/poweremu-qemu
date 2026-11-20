# Dependency of PCI devices written in Rust; it is not a device by itself.
_libpci_rs = cargo_ws.package('pci').library()
cargo_ws.package('pci').override_dependency(declare_dependency(link_with: _libpci_rs))

pci_rs = declare_dependency(link_with: [_libpci_rs], dependencies: [hwcore_rs])
