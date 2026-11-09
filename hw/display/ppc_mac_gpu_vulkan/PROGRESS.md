# Vulkan renderer port — progress log

Full design plan: `/home/yurii-sama/.claude/plans/rosy-cooking-valiant.md` (read
this first — it has the full context, hard constraints, and the Milestone
0-7 roadmap this log tracks against).

**Goal**: give `ppc_mac_gpu` (the emulated ATI Radeon 9700 PRO in this QEMU
fork) 3D acceleration on Linux via a new Rust/vulkano backend, complementing
the existing macOS-only Metal backend (`ppc_mac_gpu_metal.m`).

**Hard constraint, still upheld**: `git diff --stat hw/display/r300/` must
stay empty. Never modify `r300_state.c`/`r300_pvs.c`/`r300_us.c`/`r300_draw.c`.

## Status: a real Tiger guest is now running against this backend (this
## environment DOES have one, contradicting an earlier note in this file —
## see "Real guest testing" below) and surfaced two real bugs, both now
## fixed: a texture-descriptor crash and an unrelated stderr trace-spam
## performance bug. Milestones 0-4's original scope is done and verified
## with real-GPU tests, and has since grown well past that original scope:
## uint colour views (`R300_RTV_R8U`/`R16U`/`R32U`/`RG32U`/`RGBA32U`), MRT
## (`num_cb` up to 4), and MSAA (`aa_samples` > 1) are all now supported and
## verified — see "Uint-view colour targets" and "MRT + MSAA" below. This
## also unblocked porting `test_fmt.m`'s colour-buffer sub-tests
## (`tests/fmt.rs`, 8 tests). 3D/cube texture sampling and raw/uint-view
## textures, and mip-mapped textures beyond level 0 (the last two
## "Remaining work" backend-feature gaps) are now BOTH done too — see
## "3D/cube + raw/uint-view textures" and "Mip-mapped textures" below.
## There are no remaining backend feature gaps at all: every sub-test in
## `test_fmt.m`/`test_raster.m` is now transpiler-compatible, though
## porting those specific sub-tests to Rust hasn't been done yet (see
## "Milestone 5 progress"). 36/36 tests passing (4 new real-GPU end-to-end
## tests added across both features: 3D volume, cube, raw/uint-view 2D,
## and mip level 1 texture sampling — `tests/r300_draw_e2e.rs`).
## Milestone 5 (retargeting the 4 macOS-only Metal-oracle tests) is
## **3/4 done**: `mslcheck.m`/`test_zs.m` are fully ported
## (`tests/shadercheck.rs`/`tests/zs.rs`); `test_fmt.m` is done for its
## colour-buffer half, its texture half (32F/16F/YUV 4:2:2, all via the now
## newly-supported raw/uint-view path) is unblocked but not yet ported;
## `test_raster.m` is next — nothing in it is backend-blocked anymore.
## `qemu/tests/r300/run.sh` runs the whole Vulkan-backend R300 suite on
## Linux via `cargo test` (verified end-to-end on this host).

## Real guest testing found two bugs this session — both fixed

This environment turns out to have a real, bootable Tiger guest after all
(an earlier note further down this file, under "How to build/test right
now", claiming none was available is now known wrong — leaving it further
down as a historical note rather than editing it away, but do not trust it).
Running one against this backend immediately found two real problems real
desktop/QuartzExtreme usage exercises that no synthetic test had covered:

1. **Crash**: `draw_r300 failed: descriptor set allocation failed: ...
   descriptor_writes[N].binding: does not exist in the descriptor set
   layout`. Same *class* of bug as the earlier depth-binding gotcha
   (see "Milestone 4 progress" below), but for **textures**, and worse
   because it's not something a fixed boolean like `has_depth` can gate:
   `draw_r300.rs` computed its candidate `tex_units` list purely from
   register bind state (`R300TexDesc.bound && kind != RAW`), but a unit can
   be bound that way while the *specific compiled program* never actually
   samples it (e.g. left enabled from an earlier draw, or bound for a
   later node the current instruction stream doesn't reach) — declaring an
   unreferenced `texN`/`smpN` binding gets silently stripped from the
   SPIR-V's reflected descriptor set layout, so writing a descriptor for it
   fails outright. **Fixed** by having `transpile()` itself filter the
   candidate list against the real extracted shader body (a `\btexN\b`
   regex check) and hand the *actual* used units back to the caller —
   `shader/r300_transpile.rs`'s `transpile()` now returns a `Transpiled {
   glsl, tex_units }` struct instead of a bare `String`, and `draw_r300.rs`
   builds its texture uploads/descriptor writes from the returned
   (filtered) list, not the candidate one. Regression test:
   `tests/r300_msl_gen.rs`'s `bound_but_unreferenced_texture_unit_is_filtered_out`
   (an empty program — guaranteed not to reference any unit — given a
   non-empty candidate list, asserting the filtered result is empty and the
   assembled GLSL declares no such binding).
2. **Severe stderr spam + slowness ("shaky cursor")**: `ppc_mac_gpu.c`'s
   `ppc_mac_gpu_r300_packet3()` had `fprintf(s->r3_dump ? s->r3_dump :
   stderr, "T3 %02x n=%u %08x %08x\n", ...)` gated only on `s->r3->draws`
   (true forever after the very first R300 draw) — `s->r3_dump` is only
   non-NULL when the user explicitly sets the `R300_DUMP` env var (see
   `ppc_mac_gpu_realize()`), so by default this fell back to `stderr`
   **unconditionally** once real 3D drawing started, up to 40000 lines,
   exactly matching the reported `T3 35 n=49 0004003d 43ca0000`-style spam
   and the associated slowdown (synchronous per-draw stderr I/O during
   normal desktop compositing). This is **pre-existing, unrelated to the
   Vulkan port** (in `ppc_mac_gpu.c`, not the frozen r300 core, and not
   something this session's work touched) — it just never fired before
   because 3D acceleration never actually worked long enough to matter.
   **Fixed**: added `s->r3_dump &&` to the guard, matching the two
   neighbouring `r3_dump`-gated trace blocks in the same function, which
   never had the stray `: stderr` fallback. `R300_DUMP=<path>` still works
   exactly as intended for anyone who wants this trace deliberately.

Neither fix is covered by an automated test (the crash fix has one, per
above; the trace-spam fix is a pure diagnostics change with no observable
behavioural surface to assert on beyond "stderr stays quiet", which isn't
worth a test). Both were verified via the real guest session directly.

## Uint-view colour targets + `test_fmt.m`'s colour half: DONE (condensed)

Added `R300_RTV_R8U`/`R16U`/`R32U`/`RG32U`/`RGBA32U` support. Not a small
RGBA8 extension: reading `us_emit_cb()` (r300_us.c) in full showed only
RGBA8/NONE is register-independent; every other view's pixel encoding
(RGB565, 1555/4444, I8, UV88, several 16/32-bit float/int layouts) is
generated *per draw* from `RB3D_COLORPITCH`/`US_OUT_FMT` bits this backend
doesn't decode on the Rust side. Fix: **extract** the `r300_cbK_t`
typedef/defines/pack/unpack verbatim from the real MSL (`extract_cb_block()`
in `r300_transpile.rs`, brace-counted like the fragment-body extraction)
instead of reimplementing that logic — same reasoning as the original
"MSL is a transpiler input" correction. Needed four new `translate_syntax`
rules (`as_type<T>` bitcast, `typedef`->`#define` since GLSL has no
`typedef`, stripping `static`, stripping Metal's `constant T &name`
reference style) and `uint`/`uvec2`/`uvec4` overloads of `r300_rop` in the
prelude (previously `vec4`-only) — all four found via real compile errors,
not anticipated. Also fixed a real crash found against a running guest
(see "Real guest testing" above): `transpile()` now filters `tex_units`
against what the shader body actually references and returns them via a
`Transpiled { glsl, tex_units }` struct (was a bare `String`) — a
register-bound-but-unused texture unit was getting an unreferenced,
compiler-stripped descriptor binding, and writing to it crashed.

All five views got both a transpile-level test (`tests/r300_msl_gen.rs`,
register recipes transcribed from `test_fmt.m`'s own `render_cb()` calls)
and a real-GPU-draw test (`tests/r300_draw_e2e.rs`, one per distinct
wrapping shape: scalar `uint` via I8/R8U, `uvec2` via RG32U, `uvec4` via
RGBA32U) — every one passed, most on the first try after I8 shook out the
four syntax-rule bugs above. This then unblocked porting `test_fmt.m`'s
ten colour-buffer sub-tests to `tests/fmt.rs` (8 new, since 2 were already
covered by the uint-view work's own draw tests) — all 8 passed first try,
real confirmation the machinery generalizes rather than being coincidentally
right for the views tested directly. `test_fmt.m`'s *texture* half (formats
the shader decodes itself) still needs raw-view textures, a separate,
not-yet-started feature ("Remaining work" item 3).

## MRT + MSAA: DONE

Added `R300DrawPacket.num_cb > 1` (MRT, up to `R300_US_MAX_TARGETS` = 4)
and `aa_samples > 1` (MSAA) support — previously hard errors, and the item
requested explicitly to prioritize next.

**MRT**: `shader/r300_transpile.rs`'s `TranspileInput.rt_view: u32` became
`cb_views: &[u32]` (one view per target); `assemble()` now loops
`0..num_cb` generating one `color_imgK`/`fbK`/`R300FOut.cK`/`imageStore`
per target instead of hardcoding target 0 (bindings shift accordingly:
colour images at `2..2+num_cb-1`, depth at `2+num_cb`, textures after
that) — `extract_cb_block`/`fixed_cb_decl` were already parameterized by
target index from the uint-view work, so no new extraction logic was
needed, only the surrounding scaffold generalization. `draw_r300.rs` loops
the same per-target VRAM read/upload/write-back logic `num_cb` times
(target 0 from `pkt.rt_*`, targets 1+ from `pkt.cb[k-1]`, per that
struct's own doc comment). Verified with a real two-texture-unit-free,
two-render-target draw (`tests/r300_msl_gen.rs`'s
`two_render_targets_extracts_and_compiles`, register recipe transcribed
from `test_raster.m`'s `program(&st, false, true, false)`, and
`tests/r300_draw_e2e.rs`'s `two_render_targets_round_trip_through_vram`) —
**both passed on the first try**.

**MSAA**: the real VRAM layout (`R300DrawPacket.aa_samples`'s doc: "sample
k of row y is row y*aa_samples+k") was cross-checked against the
*reference Metal backend's own R300 draw path* (`ppc_mac_gpu_metal.m`,
not just the header comment, since a second doc comment on
`r300_msaa_offset()` initially looked contradictory — that function turned
out to be for a wholly unrelated CPU-surface-mapping code path in
`ppc_mac_gpu.c`, confirmed by checking its actual call sites, not the
renderer's own addressing at all). The reference's own comment resolved
it exactly: "one view ns rows' worth of bytes wide holds sample k in
columns k*pitch onwards" — an equivalent, alternate way to express the
same row-interleaved layout. This backend renders each sample as its own
full-resolution draw (rather than trying to rasterize into non-contiguous
striped rows directly) and reuses the *existing* single-sample VRAM
helpers unchanged, just feeding them a per-sample-adjusted `(offset,
pitch)` pair (`draw_r300.rs`'s `msaa_effective_offset_pitch`) — reading
`height` rows at `(gpu_addr + sample*pitch*bpp, pitch*aa_samples)` lands on
exactly the striped physical rows, with zero new VRAM-addressing code.
Each sample's `ms_shift` push-constant formula (`-2*aa_pos.x/width,
2*aa_pos.y/height`) was transcribed directly from the same reference
comment, not re-derived. Verified with a real 2x-MSAA draw
(`tests/r300_draw_e2e.rs`'s `msaa_2x_round_trips_through_striped_vram`,
`AA_ENABLE`/`GB_MSPOS` register writes transcribed from `test_raster.m`,
`aa_samples`/`aa_pos` read back from the real, frozen
`r300_aa_samples`/`r300_aa_positions` for that state via a new
`common::msl_and_aa_from` helper rather than guessed) — **passed on the
first try**.

Full suite: **32/32** (up from 12 at the start of this session). All 7
R200 `test_draw` checkpoints still pass (zero regression from the
`batch.rs`/`draw_r300.rs` refactoring this required); the real `ppcosx`
meson/ninja build compiles and links clean; the frozen
`hw/display/r300/*.c` core has zero diff.

## 3D/cube + raw/uint-view textures: DONE

Added `R300TexDesc.dim` (2D/3D/cube, float-sampled) and `R300_TEXK_RAW`
(the shader decodes texels itself) support — previously a hard
`draw_r300` error (3D/cube) or a silent `tex_units` exclusion (raw); this
was "Remaining work" item 3, and the one backend feature gap blocking the
most remaining `test_fmt.m`/`test_raster.m` porting work.

**Prelude** (`r300_prelude.rs`): line-for-line GLSL translations of every
`r300_us.c` function either path calls, none of which existed before —
`r300_tex`'s `texture3D`/`textureCube` overloads (mirroring the existing
`texture2D` one: same split texture+sampler call shape, `sampler3D(tx,
sm)`/`samplerCube(tx, sm)` constructed at each use since GLSL forbids
opaque-typed locals), and the whole raw/uint-view pipeline —
`r300_wrap1`/`r300_lvl`/`r300_texel`/`r300_texl`/`r300_cubeface`/
`r300_texu` — reusing the byte-decode helpers (`r300_cswap`/`r300_h16`/
`r300_tdecode`/...) already translated for the uint-view *colour-target*
work, since `r300_texel`'s texel decode is the exact same `r300_tdecode`
call. One real translation subtlety: Metal's `32 - clz(x)`
next-power-of-two rounding (`r300_lvl`'s NPOT-height case) became
`findMSB(x) + 1` (GLSL has no `clz`, but `findMSB` gives the same bit
length via `clz(x) == 31 - findMSB(x)` for a nonzero 32-bit value).

**Transpiler** (`r300_transpile.rs`): `TranspileInput.tex_units` changed
from a bare `&[u32]` to `&[TexUnitInput]` (`{unit, raw, dim}`), since a
unit's *shape* — which of the four `r300_prelude.rs` overloads/paths its
declaration must match — has no equivalent in the MSL text itself (the
call sites `r300_tex(tex0, smp0, ...)`/`r300_texu(tex0, ...)` are
textually identical regardless of dimension; only `R300TexDesc.dim`/`kind`
on the Rust side carries that information). `assemble()` now declares
`tex{unit}` as `texture2D`/`texture3D`/`textureCube` + a separate
`sampler` (float, 2 bindings) or a single real combined-image-sampler
`usampler2D` (raw, 1 binding, since `r300_texu` only ever `texelFetch`s —
never filters — but GLSL's opaque type still needs one bound); binding
numbers are now assigned with a running counter instead of a fixed
`2 * slot`, since the per-unit width varies.

**Backend** (`draw_r300.rs`): the 3D/cube hard error and raw `tex_units`
exclusion are gone. A 3D texture uploads as a real `ImageType::Dim3d`
image; a cube texture as a `CUBE_COMPATIBLE`, 6-array-layer 2D image
viewed as `ImageViewType::Cube` — both upload in **one**
`copy_buffer_to_image` call with no custom region at all, because
vulkano's default region (`Image::subresource_layers()`/`extent()`)
already spans every array layer and the whole depth extent; only the
*source* bytes needed reordering (`read_level0_faces`, face-major/
slice-major, using `R300TexDesc.lvl_pitch[0]`/`lvl_rows[0]` for the
real per-face/slice stride — confirmed against `r300_draw.c`'s
`r300_tex_layout()`, the frozen, shared-with-Metal function that fills
those fields, not guessed). A raw unit's `usampler2D` (`upload_raw_texture`)
is a flat re-view of the guest texture's own VRAM bytes, addressed at
whatever `(rowel, rows)` shape this backend chooses (mirroring the
reference Metal backend's own `r300_texture_raw`/`r300_raw_pf` split, but
addressing the common `view_bpp <= 4` case in fixed DWORDs — matching
`r300_texel`'s fixed `off >> 2` — since deriving this by reading the
shader's own contract turned out to be more reliable here than replicating
Metal's per-view-size sizing formula verbatim, whose interaction with
`view_bpp < 4` and Apple's `bytesPerRow` validation rules couldn't be
fully verified by static reading alone); the chosen `rowel` is patched
into a local copy of `R300FSUniforms.tex_info[unit].y` before upload,
exactly like the reference Metal backend patches its own copy per-draw.
`R300TexDesc.dim`-mismatched dummy fallbacks (`CONVERT16`/DXT kinds, or a
failed raw upload) now come in all four shapes (2D/3D/cube/raw), mirroring
the reference Metal backend's own `r300_dummy(dev, raw, dim)`.

Verified with three new real-GPU end-to-end tests
(`tests/r300_draw_e2e.rs`): `textured_3d_volume_round_trips_through_vram`
(a 2-slice volume texture, slice 0 sampled deterministically since the
unrouted `t[0]` source defaults to `vec4(0.0)`), `textured_cube_round_trips_through_vram`
(6 faces, a routed constant direction `(1,0,0)` selects face +X
unambiguously), and `textured_raw_2d_round_trips_through_vram` (format
0x1B, 32-bit float, decoded through the full `r300_wrap1`/`r300_lvl`/
`r300_texel`/`r300_texl`/`r300_texu` pipeline against a real bound
`R32_UINT` image) — all three passed once the *first* run's failure (RGB
came back black on the 3D/cube tests) was root-caused to the test's own
uniforms, not the new code: `R300FSUniforms.tex_info[unit].x` (the "is
this unit bound" flag `r300_tex` checks) has no automatic relationship to
`R300TexDesc.bound` in this Rust-only test harness (a real guest's
`pkt.uniforms` arrives already populated consistently by the frozen core;
a hand-built test packet must set both itself) — a real, if minor, gotcha
recorded here for the next test that binds a float-sampled texture unit.
Full suite: **35/35**. `hw/display/r300/*.c` and `ppc_mac_gpu_metal.m`
still have zero diff.

**What's left**: `test_raster.m` can now be attempted much more fully —
its MRT and MSAA sub-tests are unblocked, and border-colour/fog/ROP/
discard-src were already in scope. Its 3D/cube-texture sub-test is
unblocked by this section's work; its mip-level sub-test needed one more
feature ("Mip-mapped textures" below, also now done) — nothing in the
file is backend-blocked at all any more. This is very likely the next
natural step for Milestone 5's last file.

## Mip-mapped textures: DONE

Added real mip-chain upload for the float-sampled (non-raw) 2D/3D/cube
texture path — previously level 0 only, regardless of `R300TexDesc.levels`
("Remaining work" item 2, now item... see the renumbered list below).
The raw/uint-view path needed **no** new code at all: `upload_raw_texture`
already copies the unit's whole `size_bytes` span verbatim (the frozen
core's `r300_tex_layout()` sets that to the sum across every level, not
just level 0, since the reference Metal backend's own `r300_texture_raw`
treats the whole guest texture as one flat, level-agnostic byte blob too),
and `r300_lvl`/`r300_texel` (`r300_prelude.rs`) already compute any
level's byte offset within that same blob from `R300FSUniforms.tex_dim`/
`tex_info` alone — so a raw unit's mip selection (driven entirely by
`tex_lod[unit]`, per `r300_texu`) was already correct once "3D/cube +
raw/uint-view textures" landed; only the float/hardware-sampled path (a
real Vulkan image with a real Vulkan sampler doing the level selection)
had a gap.

**`draw_r300.rs`**: `record()`'s float-texture branch now allocates the
image with `mip_levels: tu.levels` (clamped to `R300_TEX_MAX_LEVELS`) and
loops `0..levels`, computing each level's `(w, h, depth)` by halving
(standard hardware mip convention) and reading that level's bytes via the
generalized `read_level_faces` (was `read_level0_faces`; now takes a
`level` index and uses `R300TexDesc.lvl_off`/`lvl_pitch`/`lvl_rows` at
that index instead of hardcoding `[0]` — these fields were already read
correctly by the frozen `r300_tex_layout()` for every level, just never
consulted here). The new `upload_mipped_bytes` issues **one** command
buffer submission for the whole chain: one staging buffer with every
level's bytes concatenated, one `BufferImageCopy` region per level (each
with its own `mip_level`/`array_layers`/`image_extent`) — unlike
`upload_bytes`'s single implicit region (which only ever covers mip 0, since
`Image::subresource_layers()` fixes `mip_level: 0`), a real chain needs an
explicit region per level.

**The one non-obvious gotcha**: `SamplerCreateInfo::default()`'s `lod`
range is `0.0..=0.0` — every sampler this backend had ever created (before
this change, always paired with a 1-level image) was already, silently,
hard-clamped to level 0. Uploading a real mip chain with no other change
would have had zero effect: the sampler would still only ever read level
0. Fixed by widening every R300 sampler's `lod` to `0.0..=LOD_CLAMP_NONE`
in `get_or_create_sampler` (samplers are decoupled from images in Vulkan,
so this is always safe — the *image's own* mip count is what actually
bounds the accessible levels for any given draw) and giving that same
function a small companion fix already relevant to 3D textures: the
sampler's third address-mode axis (`CLAMP_R`, `TX_FILTER0` bits 8:6) was
hardcoded to `ClampToEdge` and is now decoded from the register like the
other two. **A second, real gotcha, hit immediately after the first fix**:
setting `mipmap_mode: SamplerMipmapMode::Linear` (for a smoother blend
between levels) failed descriptor-set validation for the raw/uint-view
test — `usampler2D`'s integer image format can never advertise
`SAMPLED_IMAGE_FILTER_LINEAR` on any real hardware, and this shared,
per-`filter0`-keyed sampler cache binds the exact same sampler object to
both float and raw units. Fixed by leaving `mipmap_mode` at `Nearest`
(selects the single closest level, no blend) — valid for every format
this backend ever binds through it; `TX_FILTER1`'s real mip-filter-mode
bit isn't read at all yet, a documented scope trim, not a correctness gap
for level *selection* (only for the smoothness of the transition between
levels).

Verified with a new real-GPU end-to-end test
(`tests/r300_draw_e2e.rs`'s `textured_mip_level_1_round_trips_through_vram`):
a 2-level 2D texture (level 0 2x2 blue, level 1 1x1 magenta), sampled via
a TXB instruction with a `+10` LOD bias on top of a real per-vertex `(u,
v)` gradient (needed for a genuine, finite, negative base LOD -- a
*constant* texcoord across the quad has a zero screen-space derivative,
which makes the automatic LOD `-infinity` regardless of any bias added on
top, a real dead end hit and documented on
`textured_quad_with_uv_and_bias`'s doc comment before switching to a real
gradient) — passed on the first try once that gradient was in place. Full
suite: **36/36**. `hw/display/r300/*.c` and `ppc_mac_gpu_metal.m` still
have zero diff.

## How to build/test right now

```sh
cd qemu/hw/display/ppc_mac_gpu_vulkan
# One-time: this host's shaderc-sys can't find the system libshaderc_shared.so
# (missing unversioned dev symlink) unless you point it at a local one. The
# meson-driven build (build.sh) handles this automatically; for manual
# `cargo` runs during development, either create the symlink yourself or:
mkdir -p .shaderc-lib && ln -sf /usr/lib64/libshaderc_shared.so.1 .shaderc-lib/libshaderc_shared.so
SHADERC_LIB_DIR="$(pwd)/.shaderc-lib" cargo build --lib        # library
SHADERC_LIB_DIR="$(pwd)/.shaderc-lib" cargo run --bin test_draw # checkpoint tests (7 tests, all passing)

# Full QEMU rebuild (from qemu/build/, already configured):
cd ../../../build
env -u SHADERC_LIB_DIR ninja qemu-system-ppc   # meson's custom_target uses build.sh, doesn't need the env var
```

Smoke-test the real binary:
```sh
./qemu-system-ppc -machine mac99,via=pmu -accel tcg -m 512 -nodefaults -vga none \
  -audio none -device ati-radeon-9700,addr=e.0 -display none
# stderr should show: "ppc-mac-gpu: using Vulkan renderer for 3D"
```

**Correction**: this note previously (wrongly) said no real Tiger guest/disk
image was available in this environment. One is — see "Real guest testing
found two bugs this session" above, which is exactly how the two bugs
documented there were found. Everything in "What's built"/milestone
sections below was still originally verified via the vtable-level
`test_draw`/`tests/*.rs` checkpoints (real GPU rendering, real pixel/VRAM
assertions) rather than a booted guest, since that's what this crate's own
test suite is designed around — but a real guest is available for
additional manual verification and should be used more, not treated as
unavailable.

## What's built (by file)

`qemu/hw/display/ppc_mac_gpu_vulkan/` (new Cargo crate, not yet committed —
untracked in git):

- `src/ffi/types.rs` — hand-written `#[repr(C)]` mirrors of every struct/fn
  pointer in `ppc_mac_gpu_renderer.h` (not bindgen — see the plan for why).
  `R300DrawPacket` is still just an opaque forward-declared type (Milestone 4
  will need its real fields from `r300_draw.h`).
- `src/ffi/mod.rs` — vtable construction (`ppc_mac_gpu_renderer_vulkan()`)
  and every `extern "C"` glue fn, each wrapped in `catch_unwind` so a Rust
  panic never unwinds into QEMU's C stack.
- `src/device.rs` — headless `VulkanContext` (Instance/Device/Queue +
  allocators incl. `StandardDescriptorSetAllocator`), picks the best
  physical device (discrete > integrated > ...).
- `src/rt_cache.rs` — image cache keyed `(offset, pitch, width, height, format)`,
  serves color AND depth (depth is "just another VRAM-aliasing view").
- `src/pipeline_cache.rs` — R200 graphics pipeline cache. Key insight from
  the reference analysis: R200 needs only ONE fragment shader (all
  combiner/texture/fog/alpha-test logic is uniform-driven); the only things
  that need a distinct `VkPipeline` are colour-attachment format + real
  blend/write-mask state (depth is a storage-image binding, not an
  attachment, so it never affects the pipeline).
- `src/shader/r200_static.rs` — the R200 uber-shader (GLSL, compiled at Rust
  build time via `vulkano_shaders!`). Full 8-stage colour+alpha combiner
  (all arg selectors/modifiers/ops transliterated from the extracted Metal
  reference), manual depth/stencil via a storage image (`imageLoad`/
  `imageStore` + `bs32`/`bs16`/`zcmp`/`sop`, matching the reference's bit
  layout exactly), specular/fog/alpha-test, exclusion-rect discard. No
  swizzle gymnastics (unlike Metal) since this backend's staged-copy VRAM
  strategy fixes up byte order once at the VRAM boundary, not per-fragment.
- `src/texture.rs` — R200 texture upload. Supported formats: ARGB8888/
  RGBA8888/ABGR8888 (unified to true RGBA via CPU-side byte reorder),
  I8/Y8/AI88 (via image-view component-swizzle, not shader branching),
  packed-16 RGB565/ARGB1555/ARGB4444 (native Vulkan PACK16 formats +
  2-byte swap on upload, real hardware bilinear). **Not supported yet**
  (warn + skip unit, matching the reference's own fallback): YUV 4:2:2
  (fmt 10/11), DXT1/3/5 (fmt 12/14/15), AGP/host-memory textures
  (`host_data != NULL`).
- `src/vram/staged.rs` — VRAM <-> Vulkan-visible-memory conversion:
  `read_argb8_from_vram`/`write_rgba8_to_vram` (colour, big-endian ARGB <->
  true RGBA8), `read_raw_from_vram`/`write_raw_to_vram` (depth, verbatim —
  the shader owns all endian handling), `expand16_to_32`/`contract32_to_16`
  (Z16 <-> the storage image's fixed 4-byte texel size).
- `src/batch.rs` — **Milestone 3** (generalized this session for R300):
  `BatchState` holds one open, unsubmitted command buffer + a dirty-target
  list. `draw_r200`/`draw_r300` both record into the SAME shared batch and
  mark targets dirty instead of submitting immediately (per
  `ppc_mac_gpu_renderer.h`'s documented contract that `flush_r200()`
  finishes both); `flush()` (called from `flush_r200`/`submit_r200`/
  `zpass_r300`) is the only place that actually submits, waits, and writes
  dirty targets back to VRAM. Each dirty target now has a `TargetKind`
  (`ArgbColor`/`RawDepth`/`RawFloatColor`, replacing the old R200-only
  `is_depth: bool`) plus separate `gpu_bpp`/`vram_bpp` fields (see
  "Milestone 4 progress" below for the real R200 Z16 bug this generalization
  fixed as a side effect). Verified empirically that vulkano's automatic
  resource-usage barrier insertion correctly orders multiple
  `begin_rendering`/`end_rendering` scopes touching the same image within
  one command buffer — no manual barriers needed.
- `src/draw_r200.rs` — ties it all together: `R200Draw::record()` builds
  per-draw uniforms/descriptor-set/vertex-buffer and records the draw into
  the shared batch. Render target is always assumed 32bpp ARGB8888 (16bpp
  colour *render targets* deferred — unlike 16bpp *textures*, which work —
  since confidently decoding R200's colour-pitch format bits needs
  re-verifying against `ppc_mac_gpu.c`, not yet done).
- `src/draw_r300.rs` — **Milestone 4**: `R300Draw::record()` transpiles+
  compiles `pkt.msl`, builds/caches the pipeline and per-draw descriptor
  set (uniform buffer is a straight copy of `pkt.uniforms`, no reassembly),
  uploads bound RGBA8/R8/RG8 textures verbatim, and records into the same
  shared batch. See "Milestone 4 progress" below for the full design
  rationale (especially: no real colour/depth attachments at all, storage
  images throughout) and the bugs this pass's own end-to-end test caught.
- `src/stubs.rs` — trivial vtable stand-ins for everything that's dead code
  in this batched-path-only config (`blit_2d`, `fill_2d`, `scanout`,
  `draw_3d: None`, `fill_notify_r200`, `get_drag_state`,
  `flush_drag_paste`) — see the plan's "verified scope corrections" for why
  each one is genuinely unreachable, not a shortcut. (`zpass_r300` no
  longer lives here — it has a real implementation now, `Backend::zpass_r300`.)
- `src/bin/test_draw.rs` — the R200 checkpoint test suite (7 tests, run via
  `cargo run --bin test_draw`, drives the vtable exactly like C will):
  solid triangle, textured quad, alpha blend, colour-write mask, depth test
  (3-draw sequence), batching-defers-writeback (proves VRAM isn't touched
  until flush, and that a 2nd unflushed draw to the same target correctly
  builds on the 1st's still-pending result), range_busy. Still all passing
  after `batch.rs`'s Milestone-4 generalization (re-verified this session).
- `tests/common/mod.rs` — **Milestone 5**: shared harness for the R300
  integration test files (`msl_from`, `read_register_dump`,
  `r300_tests_file`, `compile_glsl_ok`, `new_backend`, `leak_msl`/`free_msl`,
  `base_uniforms`, `base_packet`, `rect`, `full_screen_quad`). Introduced
  when `tests/zs.rs` needed the same MSL-generation/backend-driving helpers
  `tests/r300_draw_e2e.rs` had already duplicated; that file was refactored
  onto this module too (see the plan's own `tests/common/mod.rs` layout note).
- `tests/r300_draw_e2e.rs` — one real-GPU end-to-end draw test (untextured,
  no depth, empty fragment program) — see "Milestone 4 progress".
- `tests/shadercheck.rs` — **Milestone 5**: port of the original
  `mslcheck.m`, including replaying the real captured `qe_state_draw1.txt`
  Quartz Extreme program — see "Milestone 5 progress".
- `tests/zs.rs` — **Milestone 5**: port of the original `test_zs.m`, all 7
  depth/stencil/culling/Z-pass scenarios, against the real GPU — see
  "Milestone 5 progress". This port caught the negative-viewport-height
  bug documented in "Milestone 4 progress".
- `src/bin/spike.rs` — Milestone 0's original headless-Vulkan smoke test,
  kept around.

QEMU C-side changes (all Milestone 1, unchanged since):
- `meson.build`: probes `dependency('vulkan', required: false)` + `cargo`,
  sets `CONFIG_PPC_MAC_GPU_VULKAN` in `config_host_data`.
- `hw/display/meson.build`: `custom_target` running
  `ppc_mac_gpu_vulkan/build.sh` (handles the shaderc-sys packaging-gap
  workaround automatically) to build the Rust staticlib, wired into
  `system_ss` when available.
- `hw/display/ppc_mac_gpu.c`: `#include` for the new header; renderer
  cascade tries Vulkan after Metal, before software fallback; also fixes a
  **pre-existing, unrelated** Linux-linking bug (`g_frame_tracker` was only
  ever defined in the Metal file, so this fork could never link on Linux in
  any configuration before this session — 2-line `#ifndef CONFIG_DARWIN`
  fix, not part of the Vulkan work itself).

## Milestone-by-milestone summary

- **M0 (spike)**: headless vulkano device creation + GPU round-trip
  verified on this host's real GPU (AMD RX 7600S / RADV NAVI33). Confirmed
  vulkano/ash `dlopen`s the Vulkan loader at runtime (no link-time
  `-lvulkan` needed in the meson wiring).
- **M1 (first triangle)**: minimal `draw_r200` — one fixed solid-colour
  shader, staged VRAM, full FFI/meson/C wiring end-to-end. This is where
  the `g_frame_tracker` Linux-link bug was found and fixed.
- **M2 (full R200 fidelity)**: full combiner, multi-texture, real
  blend/write-mask pipeline state (a deliberate, documented improvement
  over Metal's manual approach — this backend doesn't have Metal's
  byte-swizzle constraint), manual depth/stencil, scissor + exclusion
  rects, specular/fog/alpha-test.
- **M3 (batching)**: real command-buffer batching, real `flush_r200`/
  `submit_r200`/`range_busy_r200` (previously stubs, now do real work).
- **M4 (`draw_r300` + transpiler)**: MSL->GLSL transpiler, `draw_r300.rs`
  dispatch, real `zpass_r300`, all verified against a real GPU for this
  pass's scoped feature set (RGBA8 colour view, 2D float textures, single
  target, no MSAA). Found and fixed 3 real bugs (a `batch.rs` gap that also
  fixed a latent R200 Z16 bug, an unconditional-descriptor-write gotcha,
  and — via Milestone 5's `zs.rs` port — a vertical-flip/winding bug from
  Metal-vs-Vulkan NDC-Y convention differences).
- **M5 (retarget the 4 macOS-only Metal-oracle tests)**: 2 of 4 fully done
  (`mslcheck.m`/`test_zs.m`), 1 of 4 partially done (`test_fmt.m`'s
  colour-buffer half); every backend feature gap that blocked the other 2
  (uint colour views, raw/3D/cube textures, MRT, MSAA, mip levels) is now
  fixed — what's left for `test_fmt.m`'s texture half and `test_raster.m`
  is purely test-porting work, not backend work. See "Milestone 5
  progress".

## End goal

Two acceptance criteria for calling this port "done":

1. **The whole R300 test suite passes**: every scenario across
   `tests/r300_msl_gen.rs`, `tests/shadercheck.rs`, `tests/zs.rs`,
   `tests/r300_draw_e2e.rs` — plus, once backend feature work unblocks
   them (see "Remaining work"/"Milestone 5 progress" above), the
   in-scope-and-beyond parts of `test_fmt.m`/`test_raster.m` too.
   **Currently: 12/12 passing** across the four files above (real GPU,
   `cargo test` from the crate dir or via `qemu/tests/r300/run.sh`). Not
   fully met yet: `test_fmt.m`/`test_raster.m`'s full scope needs the
   backend feature work (item 2-4/6 in "Remaining work") first.
2. **The real `ppcosx` QEMU build compiles clean end-to-end** — not just
   `cargo build`/`cargo test` in isolation, but the actual meson/ninja
   build of `qemu-system-ppc` picking up this crate through
   `hw/display/meson.build`'s `custom_target` (`build.sh`), with the
   renderer-selection cascade in `ppc_mac_gpu.c` finding
   `ppc_mac_gpu_renderer_vulkan()` and linking successfully. **Currently:
   MET** — `cd qemu/build && env -u SHADERC_LIB_DIR ninja qemu-system-ppc`
   builds clean and the resulting binary really does initialize the
   Vulkan renderer (verified via the documented smoke test:
   `ppc-mac-gpu-vulkan: using AMD Radeon RX 7600S (RADV NAVI33)
   (DiscreteGpu)` / `ppc-mac-gpu: using Vulkan renderer for 3D` on stderr).
   **This caught a real, previously-undiscovered gap**: Milestone 4 added
   `shader/runtime_compile.rs`'s runtime `shaderc` dependency (unlike
   Milestones 0-3's R200 shaders, compiled at Rust *build* time via
   `vulkano_shaders!`, needing no runtime dylib), but
   `hw/display/meson.build`'s `declare_dependency(link_args: [...])` for
   the Rust staticlib never linked `libshaderc_shared.so` at all — a Rust
   `staticlib`'s transitive dynamic-library needs aren't visible to an
   external (non-Cargo) linker automatically, so `cargo test`/`cargo build`
   from inside the crate could never have caught this (Cargo resolves its
   own binaries' transitive deps fine); only the actual meson/ninja
   `qemu-system-ppc` link — a completely different build graph — surfaced
   it, as a wall of `undefined reference to shaderc_*` linker errors. Fixed
   by adding `-lshaderc_shared` (plus a harmless-if-absent `-L` pointing at
   `build.sh`'s own packaging-gap workaround directory) to that
   `link_args` list. **This is exactly why criterion 2 is tracked
   separately from criterion 1, and why both must be checked after any
   change touching this crate or its C-side wiring** — a green `cargo test`
   does not by itself prove the meson/ninja integration still works.

## Milestone 4 progress (`draw_r300` + the MSL->GLSL transpiler)

**Core design (do not re-litigate, verified by reading the actual code):**
`draw_r300()` receives an `R300DrawPacket` whose fragment-shader field is
`char *msl` — already-compiled Metal Shading Language text, generated by
the frozen `r300_us_to_msl()` inside `r300_draw.c`. The raw R300 microcode/
`R300State`/`R300FSDesc` inputs that function needs never reach the
renderer. So the R300 fragment path is an **MSL-to-GLSL transpiler** over
`pkt->msl`'s text (cached by that string itself, matching its own "the
string is its own cache key" doc comment), not a reimplementation of R300
microcode decoding. `ppc_mac_gpu_metal.m`'s own R300 integration is
literally `r300_pipeline(dev, pkt->msl, ...)` -> `newLibraryWithSource:` —
it does nothing more than compile the text it's handed.

**A full research pass over `r300_us.c` (1487 lines) produced the exact
vocabulary this transpiler needs to handle** — the key finding that makes
this tractable: the generated MSL is composed of (a) a **fixed prelude**
(`us_prelude`/`us_prelude_z`), emitted byte-identical for *every* R300 draw
regardless of the guest's program, defining every helper function the rest
of the text calls into, and (b) a small, **fully enumerable set of
per-instruction line templates** (from `us_emit_tex`/`us_emit_alu`) that
vary only in integer-literal substitutions — R300 fragment programs here
are fully unrolled straight-line code, no runtime branches/loops in the
per-instruction text at all (only `discard_fragment()` and ternaries).
This means (a) only needs hand-translating *once* (done, see below), and
(b) is mechanical enough for simple pattern-based translation, not a real
parser.

### Done so far

- **`ffi/types.rs`**: real `R300DrawPacket`/`R300ColorDesc`/`R300DepthDesc`/
  `R300TexDesc`/`R300FSUniforms`/`R300Vertex` mirrors, replacing the old
  opaque placeholder. **Verified byte-exact against the real C layout**: a
  throwaway C program (`sizeof`/`offsetof` on every field) and a matching
  Rust `sizecheck` bin (`cargo run --bin sizecheck`) print identical numbers
  for every struct/field checked — high confidence the hand-written mirror
  is correct, not just "looks right."
- **`shader/r300_vs.rs`**: the fixed vertex-shader pass-through, written by
  hand (confirmed against the real `r300_vs` MSL text — a ~15-line
  passthrough, vertex transform already ran on the CPU in frozen
  `r300_pvs.c`/`r300_draw.c`). Clip planes (`R300Vertex.ucp`) not
  implemented yet (needs the `shaderClipDistance` device feature) —
  deferred, documented in the file.
- **`shader/r300_prelude.rs`**: the *entire* fixed MSL prelude, hand-
  translated to GLSL line-for-line (same function names, same order, diffable
  against `r300_us.c`) — texture sampling (2D only; 3D/cube deferred),
  the full 19-format raw-texel decode table (`r300_tdecode`), YUV
  conversion, blend factors/combine (`r300_bfactor`/`r300_combine`), the
  RGBA8-view colour-buffer pack/unpack (`r300_pack`/`r300_unpack`/
  `r300_chan`/`r300_to_hw`/`r300_from_hw` — uint-view targets R8U/R16U/
  R32U/RG32U/RGBA32U deferred), ROP (float4 case only; uint/uint2/uint4
  overloads deferred), fog, discard-src, alpha test, and the **complete**
  depth/stencil implementation (`r300_ztest`/`r300_zcmp`/`r300_sop`/
  `r300_zswap`) — this one wasn't scoped down, it's fully translated.
  **Verified to actually compile**: `cargo run --bin prelude_check` wraps
  the whole prelude in a minimal shader exercising every function and
  compiles it via shaderc — currently passing. Two real bugs were caught
  and fixed this way (a mistranscribed `.abgr` swizzle that would have
  silently reversed colour-buffer byte order, and a GLSL rule that atomic
  ops can't target a value forwarded through a regular function's `inout`
  parameter — `r300_ztest`'s Z-pass counting was restructured so the
  `atomicAdd` happens at the call site instead, where the real SSBO
  variable is in scope; see the comment above `r300_ztest`).
- **`shader/runtime_compile.rs`**: `ShaderCache`, wrapping `shaderc` to
  compile transpiled GLSL to SPIR-V at runtime and build a
  `vulkano::shader::ShaderModule` from raw words (not the `vulkano_shaders!`
  macro, since content is guest-microcode-dependent), cached by the source
  GLSL string.
- **Design decision, worked out carefully, not yet implemented**: the
  prelude's texture/blend/ztest functions were deliberately written to take
  Metal's *exact* call signature shape — e.g. `r300_tex(sampler2D tx,
  R300FSUniforms u, uint unit, vec4 c, bool proj, float bias, uint bm)`,
  passing the whole uniforms struct + a unit index by value, exactly like
  Metal's `constant R300FSUniforms &u` parameter — rather than pre-extracting
  per-unit fields into separate GLSL arguments. This means the actual
  per-draw transpiler (not yet written) should need almost no argument
  restructuring at call sites, just Metal->GLSL *type-keyword* renaming
  (`float4`->`vec4` etc.) plus detecting a few "not supported yet" cases
  (raw/uint-view textures, non-2D texture dimensions, uint-view colour
  targets) — this was a real simplification found partway through and is
  why the prelude functions look the way they do; don't "clean this up" by
  making them take flattened arguments, that would make the transpiler
  harder, not easier.
- **`R300FSUniforms`'s GLSL declaration** (`R300_UNIFORMS_STRUCT` constant
  in `r300_prelude.rs`) needed real care for std140 layout correctness: any
  Rust field that's a fixed-size array of 4-or-fewer elements (`[u32; 4]`,
  `[u32; 2]`, ...) is declared as a `vecN`/`uvecN`/`ivecN` in GLSL, **never**
  as a GLSL array — std140 forces every array element to a 16-byte stride
  regardless of the element's own size, which would silently break
  alignment against the tightly-packed Rust struct for anything smaller
  than vec4. Arrays of already-vec4-sized elements (`consts[32]`,
  `tex_swz[16]`, ...) are unaffected (vec4's own alignment already is 16)
  and stay as real GLSL arrays. This was reasoned through carefully but
  **not yet verified with a sizecheck-style tool** the way the FFI structs
  were — doing that (a small GLSL/SPIR-V reflection check, or just careful
  re-derivation) before wiring up real uniform uploads would be prudent,
  since a std140 mistake here would silently corrupt every R300 draw's
  shader inputs with no compile-time signal.

### Explicit scope trim for this pass (documented in code, not oversights)

- RGBA8 (float) colour-buffer view only (`R300_RTV_RGBA8`) — uint-view
  targets (R8U/R16U/R32U/RG32U/RGBA32U) deferred.
- Float-sampled 2D textures only — raw/uint-view textures (`r300_texu`/
  `r300_tex_raw_bpp` path) and 3D/cube textures deferred (the *shader-side*
  decode math for raw textures, `r300_tdecode`, is already fully
  translated and ready — only the Rust-side Vulkan texture setup for that
  path is what's deferred).
- Single render target (no MRT) for this pass, though the prelude doesn't
  block adding it later.
- User clip planes and 3D/cube texture sampling deferred (documented in
  `r300_vs.rs`/`r300_prelude.rs`).

### The bindgen test harness is now DONE (and paying off immediately)

`build.rs` (crate root) compiles the frozen `r300_state.c`/`r300_pvs.c`/
`r300_us.c`/`r300_draw.c` via the `cc` crate and generates bindgen bindings
(`R300State`, `R300FSDesc`, `r300_state_reset`, `r300_state_write`,
`r300_us_to_msl`) into `OUT_DIR/r300_bindings.rs` — test-only, nothing in
`src/` references it, so the shipped production staticlib is unaffected.
`tests/r300_msl_gen.rs` drives the real function and asserts on genuinely
generated MSL text. Run with:
```sh
SHADERC_LIB_DIR="$(pwd)/.shaderc-lib" cargo test --test r300_msl_gen -- --nocapture
```
(2 tests currently, both passing.)

**One real Cargo gotcha hit and worked around**: `cc::Build::compile()`'s
own `cargo:rustc-link-lib=static=...` directive did not make it into the
integration test's link line in this workspace (root cause not fully
understood — possibly related to having multiple `[[bin]]` targets in the
same package; the directive *was* present in the build script's declared
output but the final `cc` invocation genuinely never included `-lr300core`,
confirmed via `--verbose`). Fixed by also emitting
`cargo:rustc-link-arg-tests=<absolute path to libr300core.a>` directly in
`build.rs`, which forces the archive onto the test binary's link line
regardless of the `-l`/`-L` propagation issue. If a future `cc`/Cargo
version fixes the root cause, this extra line is harmless to leave in.

**First real ground-truth win**: the "empty program" test's generated MSL
prelude is **function-for-function identical** to the hand-translated
`R300_GLSL_PRELUDE`/`R300_UNIFORMS_STRUCT` in `r300_prelude.rs` — same
names, same order, same struct layout. This is a strong, direct
confirmation that the prelude translation work is correct, not just
"looks plausible."

### The transpiler is now DONE for this pass's scope, and both tests pass

`src/shader/r300_transpile.rs` is written and working:
`transpile(&TranspileInput { msl, tex_units, has_depth })` returns complete,
shaderc-verified GLSL. Its design (see the file's module doc, which is
load-bearing — read before touching this file) turned out much smaller than
originally scoped, because of a mid-implementation realization: **every
byte of `pkt->msl` outside `r300_shade`'s function body is scaffolding this
backend can already regenerate from `R300DrawPacket`'s Rust-side fields
directly** (`num_cb`, `depth.attach`, `tex[k].bound`) — the struct decls,
`r300_vs`/`r300_fs`/`r300_fs_z` signatures, texture parameter lists. So
`transpile()` does NOT parse/strip/detect the prelude at all; it:

1. **Extracts** `r300_shade`'s body verbatim (brace-counted, since the body
   contains nested blocks) — this is the *only* text with no Rust-side
   equivalent, since it's the actual decoded-from-microcode fragment logic.
2. **Translates syntax only** (`translate_syntax`): word-boundary type
   renames (`float4`->`vec4` etc.), the reserved-keyword rename
   `in`->`vsin`, and a handful of **direct, known-fixed-line substitutions**
   for Metal/C's implicit int-as-bool-in-ternary idiom, which GLSL
   disallows outright (`(mask) ? a : b` needs `((mask) != 0u) ? a : b`).
   This is not a general rewrite rule — it targets specific fixed skeleton
   lines from `us_emit_cb()`/the SC_CLIP_RULE check that are byte-identical
   across every draw (confirmed by reading `r300_us.c`'s printf templates
   directly, not guessed) — an instruction-dependent ternary condition that
   already reads as a real comparison (e.g. `zcmp`'s hand-written GLSL)
   never hits this rule because the hand-translated prelude already writes
   proper bool conditions.
3. **Assembles** the final shader by regenerating everything else from the
   Rust-side packet fields, inlining the translated body into `main()`.

Both `tests/r300_msl_gen.rs` tests fully pass (transpile + real shaderc
compile): `empty_program_generates_valid_msl_skeleton` and
`single_textured_stage_generates_tex_call` (the latter's register encoding
was also fixed this pass — see below).

**Bugs found and fixed while getting here** (beyond the prelude bugs listed
above): (a) `single_textured_stage`'s original register-write sequence
produced 0 tex instructions because `US_CONFIG` bit 3 (`FIRST_NODE_HAS_TEX`)
wasn't set — `us_nodes()` force-zeroes node 0's `tex_count` unless that bit
is set (confirmed by reading `us_nodes()` directly); fixed with the correct
encoding (documented in exhaustive comments in the test itself, citing
exact bit positions). (b) `R300_CB0_CLAMP`/`R300_CB0_BLEND` (fixed
`#define`s this transpiler's `assemble()` emits, matching what
`us_emit_cb()`'s RGBA8-view case always emits too) had to be `true`/`true`,
not `1`/`1` — GLSL's `?:` requires an actual bool, unlike Metal/C. (c) Three
more fixed lines from `us_emit_cb()` (`r300_us.c:1226,1241-1242`) had the
same implicit-int-as-bool ternary idiom as (b) but weren't `#define`-guarded
— fixed via the direct substitutions in `translate_syntax()` described
above. All of (b)/(c) were only caught by actually compiling real generated
MSL's transpiled output via shaderc, not by inspection — reinforces why the
bindgen harness was worth building first.

### `draw_r300.rs` is now DONE for this pass's scope, verified with a real GPU draw

`src/draw_r300.rs` dispatches R300 draws: transpiles+compiles `pkt.msl`
(cached by `ShaderCache`), builds/caches a `GraphicsPipeline` (keyed by
fragment-shader identity + primitive topology + cull mode/front face —
there's no blend or attachment-format axis at all, see below), uploads
bound RGBA8/R8/RG8 textures verbatim (no CPU-side byte reordering — the
shader owns all of that, same principle as the colour buffer), builds the
descriptor set, and records into the **same shared `BatchState`**
`draw_r200` uses (`ppc_mac_gpu_renderer.h`'s documented contract:
"Batched with draw_r200 work: flush_r200() finishes both" — R300 does NOT
get its own command buffer). `zpass_r300` is also fully implemented now
(`Backend::zpass_r300`): flushes pending work, reads the persistent Z-pass
SSBO (`Backend::zpass_buffer`, a real host-visible `Subbuffer<u32>` the
shader `atomicAdd`s into), and resets it to `value` when asked — replacing
the old always-empty stub.

**Key design point carried over from the transpiler**: the R300 colour
buffer is **not a real Vulkan attachment at all** — it's an `rgba32f`
storage image holding one raw VRAM byte per float channel (see
`draw_r300.rs`'s module doc), because the transpiled shader does all
blending/ROP/channel-packing itself (R300 exposes non-standard colour views
Vulkan's fixed-function blend hardware can't represent). This means the
render pass genuinely has **zero colour or depth attachments** —
`RenderingInfo`/`PipelineRenderingCreateInfo` both go through with empty
attachment lists, `render_area_extent`/`layer_count` set explicitly since
there's no attachment for Vulkan to infer them from. This works and is
verified against a real GPU (see below), not just reasoned through.

**Two real bugs found and fixed via the new end-to-end test** (beyond the
transpiler-only bugs from before):
- **`batch.rs`'s dirty-target tracking couldn't represent R300's colour
  buffer** (a third kind, on top of R200's ARGB-colour/raw-depth). Fixed by
  generalizing `DirtyKey.is_depth: bool` into `DirtyKey.kind: TargetKind`
  (`ArgbColor`/`RawDepth`/`RawFloatColor`), and splitting the old single
  `bpp` field into `gpu_bpp` (the Vulkan image's real texel size, for
  readback buffer sizing) and `vram_bpp` (VRAM's own footprint, for
  write-back and `range_busy`). **This also fixed a real latent bug in the
  existing R200 depth path**: `mark_dirty` was passing `pkt.depth_bpp` (2
  for Z16) as if it were the GPU image's texel size, when the depth image
  is always a 4-byte `R32_UINT` — an undersized flush readback buffer for
  any Z16 batched depth draw, never caught before because
  `test_draw.rs`'s existing batching test only exercises `depth_bpp == 4`.
  Added `vram::staged::bytes_to_floats01`/`floats01_to_bytes` for the new
  `RawFloatColor` kind's byte<->float conversion. All 7 existing R200
  `test_draw` checkpoints still pass after this refactor (real GPU,
  re-verified).
- **Unconditionally writing a descriptor for binding 3 (`depth_img`) failed
  outright** ("binding 3 does not exist in the descriptor set layout")
  whenever `has_depth` is false: `assemble()`'s generated shader simply
  never references `depth_img` in that case, and vulkano's SPIR-V-reflected
  descriptor set layout only allocates slots for bindings the shader
  actually uses — a declared-but-unreferenced `layout(binding=...)` gets no
  slot at all. Fixed by making that one `WriteDescriptorSet` conditional on
  `has_depth` (every other fixed binding — UBO, Z-pass SSBO, colour image —
  *is* referenced unconditionally by the shader, so those stay
  unconditional). This is a real, general Vulkan/vulkano gotcha worth
  remembering: **a descriptor write for a binding the shader doesn't
  reference will fail, not silently no-op.**

**New end-to-end verification**: `tests/r300_draw_e2e.rs` drives the real
`PPCMacGPURenderer` vtable (real GPU, not just transpile+shaderc-compile)
with a genuine empty-program MSL from `r300_us_to_msl` (via the same
bindgen FFI `r300_msl_gen.rs` uses, namespaced under `mod bg` to avoid
colliding with `ffi::types`'s hand-written `R300Vertex`/`R300FSUniforms`
mirrors — a real name clash hit and fixed this pass) and a hand-built
`R300DrawPacket` (untextured, no depth, full-screen quad). Asserts: (a) the
rendered VRAM bytes are exactly opaque black in the expected raw
`(R,G,B,A)` byte order for the `out_sel`/`rt_swap32` chosen, and (b) the
Z-pass SSBO counts exactly `width * height` fragments and correctly resets
— a real, working regression test for the two bugs above and for the whole
pipeline (transpile -> compile -> pipeline -> descriptor set -> vertex
buffer -> draw -> storage-image round-trip -> VRAM writeback -> Z-pass
readback).

**A third real bug, found via Milestone 5's `tests/zs.rs` port (see below),
not this section's own e2e test**: `R300Vertex.pos` arrives pre-transformed
to clip space by the frozen, CPU-side `r300_pvs.c`, which computes it
assuming *Metal's* NDC-Y-up convention (matching the reference Metal
backend's own viewport) — Vulkan's NDC-Y convention is the opposite (down).
Passing the position through unchanged (`shader/r300_vs.rs`, unchanged
since it was written) rendered every real R300 draw vertically mirrored,
*and* with inverted front-face winding — invisible to this section's own
e2e test because a symmetric full-screen quad and a fragment-count-only
Z-pass check are both insensitive to a Y-flip. Fixed with a single change
in `draw_r300.rs`: a **negative-height viewport**
(`offset: [0, rt_height], extent: [rt_width, -rt_height]`, core since
Vulkan 1.1) — this corrects both the Y direction *and* the winding-order
interpretation in one step, per the Vulkan spec, and is verified against
real pixel/depth/winding checks in `tests/zs.rs`, not just reasoned
through. R200 is unaffected (its own vertex shader already computes NDC
from pixel coordinates in Vulkan's native convention directly, see
`shader/r200_static.rs`).

### Completed (moved out of "Remaining work" as each landed)

- **MRT and MSAA** — see "MRT + MSAA" above.
- **3D/cube texture sampling and raw/uint-view textures** — see "3D/cube +
  raw/uint-view textures" above.
- **Mip-mapped textures** — see "Mip-mapped textures" above.
- **Uint-view colour render targets** — see "Uint-view colour targets +
  test_fmt.m's colour half" above.
- **Verify the `R300FSUniforms` std140 GLSL layout**:
  `cargo run --bin uniform_layout_check` compiles `R300_UNIFORMS_STRUCT`
  inside a real std140 UBO via shaderc, parses the resulting SPIR-V's
  `OpMemberDecorate ... Offset` decorations directly (no crate needed —
  SPIR-V is a fixed word-stream format), and compares every one of the 26
  fields' actual glslang-assigned offset against Rust's own
  `std::mem::offset_of!` — all 26 match exactly (`size_of::<R300FSUniforms>()
  == 2000`, consistent with the earlier FFI `sizecheck` result). This is
  the compiler's *real* view of the layout, not a second hand-derivation
  that could repeat the same mistake as the first.

### Remaining work

There are no known backend *feature* gaps left — every item below is
either test-corpus growth or porting an already-unblocked sub-test.

1. Grow the `tests/r300_msl_gen.rs` corpus: only two scenarios exist there
   (empty program, one textured stage with no blend/depth). Add blended,
   depth-tested, and multi-instruction-node scenarios.
2. `num_line_verts` (polygon-mode wireframe edges) — silently not drawn;
   see `draw_r300.rs`'s module doc.
3. Polygon offset and OMASK_W (program-supplied depth) — `assemble()`'s
   `depth_read_write` template already emits the GLSL for both (see
   `r300_transpile.rs`), so this may already work; not yet exercised by any
   test (`test_raster.m`'s "depth from the program; polygon offset" scenario
   is no longer blocked by anything else in that file now that MRT/MSAA are
   done — a good next sub-test to port on its own).

**If picking this up fresh**: read `draw_r300.rs`'s module doc first (the
storage-image colour buffer design, the negative-viewport-height note, the
MRT/MSAA and texture-units sections, and the exact scope-trim list), then
`r300_transpile.rs`'s module doc, then `r300_prelude.rs`/`r300_vs.rs`
(finished, tell you the exact GLSL vocabulary available), then look at
"Milestone 5 progress" below before picking an item — there are no
remaining *backend feature* gaps; everything left is *porting*
(`test_fmt.m`'s texture half, `test_raster.m`'s sub-tests) or test-corpus
growth rather than new backend work.

## Milestone 5 progress (retargeting the 4 macOS-only Metal-oracle tests)

**`mslcheck.m` -> `tests/shadercheck.rs`: DONE.** Drives the real
`r300_us_to_msl()` — including replaying `qe_state_draw1.txt`, the exact
same real Quartz Extreme fragment program (captured from a booted Tiger
guest) and `R300FSDesc.route` the original `test_us.c`/`mslcheck.m`
pairing used — and asserts the transpiled GLSL compiles via shaderc, the
direct analog of the original's `newLibraryWithSource:` check. 2/2 tests
passing, including the real capture (confirmed by inspecting its generated
MSL directly: exactly one float-sampled 2D texture unit, fully within this
transpiler's supported vocabulary).

**`test_zs.m` -> `tests/zs.rs`: DONE, all 7 scenarios.** Depth (Z24S8
LESS with/without write, DEPTHENDIAN dword swap, Z16 GEQUAL), stencil
(ALWAYS/REPLACE then EQUAL, write-mask), two-sided stencil +
front-facing-vs-cull interaction, culling (back/front/CW-front), and the
Z-pass counter (half-occluded quad, exact count) — all against the real
GPU via the real `PPCMacGPURenderer.draw_r300` vtable entry, real VRAM
bytes, real depth words. Register writes/uniform fields/expected values
are transcribed directly from the original file (already-correct fixed
points independent of the Metal-vs-Vulkan API choice), not re-derived.
**This port is what caught the negative-viewport-height bug documented
above** — the culling and two-sided-stencil scenarios are winding-sensitive
and a symmetric quad can't expose a Y-flip, exactly the gap Milestone 5's
"port the real tests, don't just keep the ones you already had" purpose is
for.

**`test_fmt.m` -> `tests/fmt.rs`: colour-buffer half DONE (8 sub-tests, plus
2 more already covered elsewhere); texture half still blocked.** Once
uint-view colour targets landed (see "IN PROGRESS" above), all ten of the
original's colour-buffer sub-tests became portable: `argb16161616_unorm`
(RG32U, unorm not FP), `c_32_fp`/`c2_16_fp` (R32U), `rgb565`/`argb1555`/
`argb4444_blend`/`uv88` (R16U), `chanmask_keeps_old_value` (RG32U) are new
in `tests/fmt.rs`; the two RG32U/RGBA32U "_FP" sub-tests were already
ported (as `rg32u_colour_view_round_trips_through_vram`/
`rgba32u_colour_view_round_trips_through_vram`) while verifying the
uint-view work itself, and aren't duplicated. **All 8 new tests passed on
the first try** — real confirmation that the uint-view colour-target
machinery generalizes correctly, not just for the views already directly
tested. The file's *texture* half (32F/16F/YUV 4:2:2 formats the shader
decodes itself, all via `unit_is_raw()`'s raw path) is a completely
separate, not-yet-started feature (raw-view textures, "Remaining work"
item 3) and is not attempted — `tests/fmt.rs`'s module doc says so
explicitly, and the original `.m` file is left as-is (still
building/running via `clang`+Metal on macOS, per `run.sh`'s untouched
fallback block) for that half.

**`test_raster.m`: partially portable now, not yet attempted.** Covers mip
levels, 3D/cube texture sampling, border colour, fog/ROP/discard-src, MRT,
depth-from-program (OMASK_W), polygon offset, and 2x/4x multisampling. Now
that MRT and MSAA are done (see "MRT + MSAA" above), only its mip-level
and 3D/cube-texture sub-tests are still blocked ("Remaining work" items 3,
6) — border colour, fog/ROP/discard-src, MRT, depth-from-program/polygon
offset, and the 2x/4x-multisampling sub-tests are all within scope right
now. Given the file still mixes in-scope and out-of-scope sub-tests
together, porting it will need splitting (port the in-scope sub-tests now,
leave the mip/3D-cube ones for later) rather than a straight 1:1
translation, or an all-at-once port once those two remaining features
land — either is reasonable, not yet decided. This is the natural next
step for Milestone 5.

`qemu/tests/r300/run.sh` runs all three completed ports via `cargo test`
on any platform with `cargo` + a Vulkan loader (verified end-to-end on
this Linux host: full portable-C-test block, then `shadercheck`/`zs`/
`fmt`/`r300_msl_gen`/`r300_draw_e2e`, all passing — 29 R300 tests total),
while still building+running all 4 original `.m` files on macOS as before
(the already-completed portions are redundant there now, but the files are
left in place — deciding whether to retire them, or split `test_fmt.m`
into its still-relevant texture-only remainder, is a separate call, not
made here).

**If resuming Milestone 5**: only `test_raster.m` is left, and most of it
is unblocked now (MRT/MSAA done this session) — only its mip-mapping and
3D/cube-texture sub-tests remain gated ("Remaining work" items 3, 6).
`tests/common/mod.rs` (the shared harness: `msl_from`, `msl_and_aa_from`,
`read_register_dump`, `r300_tests_file`, `compile_glsl_ok`, `new_backend`,
`base_uniforms`, `base_packet`, `rect`, `full_screen_quad`) already has
everything needed to port its in-scope sub-tests (border colour,
fog/ROP/discard-src, MRT, depth-from-program/polygon offset,
multisampling) right now — this is likely the most valuable next step in
this whole file.

## Known limitations / deferred items (all documented in code comments too)

- No texture or pipeline *view* caching across draws yet (everything
  re-uploads/re-creates fresh every draw — correctness first).
- Texture uploads and RT/depth VRAM seeding remain eager (their own small
  submit+wait), not folded into the M3 batch — a reasonable, bounded scope
  trim, not a correctness gap (see `batch.rs`'s module doc).
- Depth/stencil read-modify-write via `imageLoad`/`imageStore` has no
  ordering guarantee across *overlapping primitives within the same draw
  call* on non-tile-based GPUs (cross-draw-call ordering is fine). Fixable
  later via `VK_EXT_fragment_shader_interlock`, deliberately deferred.
- A texture read always samples VRAM as of the last flush — a
  render-to-texture pattern within one unflushed batch sees stale data
  until the next flush. This matches a real hardware hazard (an explicit
  sync is normally required there too), not a shortcut.
- 16bpp colour *render targets* unsupported (16bpp *textures* work) — see
  M2 notes above.
- Legacy `draw_3d` path, real async `submit_r200` worker thread,
  `get_drag_snap`, `scanout`/`fini` real logic, and the entire shadow-RT/
  drag-tracking/CPU-2D-blit subsystem are explicit **non-goals** per the
  plan (verified unreachable in the `PPCGPU_R200_DIRECT`-default
  configuration this backend targets) — not gaps, don't try to "complete"
  these later without re-reading the plan's reasoning first.
- Zero-copy VRAM mode (`PPCGPU_ZEROCOPY=1` opt-in) — Milestone 6, not
  started, lowest priority (staged/default path must stay solid first).
