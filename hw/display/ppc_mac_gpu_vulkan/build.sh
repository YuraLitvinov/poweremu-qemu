#!/bin/sh
# Build the Vulkan renderer's Rust staticlib and drop it at the path meson's
# custom_target expects. Invoked by ../meson.build, not run directly.
set -eu

crate_dir=$1
target_dir=$2
output=$3
# Cargo features to enable (the Rust R300 crate, for builds without the C one).
features=${4:-}

cargo build --release --manifest-path "$crate_dir/Cargo.toml" --target-dir "$target_dir" ${features:+--features "$features"}
cp "$target_dir/release/libppc_mac_gpu_vulkan.a" "$output"
