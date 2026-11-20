#!/bin/sh
# Build the device crate's static library (the Rust device and the Vulkan
# renderer) and drop it at the path meson's custom_target expects.  Invoked by
# ../meson.build, not run directly.
set -eu

crate_dir=$1
target_dir=$2
output=$3

cargo build --release --manifest-path "$crate_dir/Cargo.toml" --target-dir "$target_dir"
cp "$target_dir/release/libppc_mac_gpu_dev.a" "$output"
