# AGENTS.md — tpt-chora

tpt-chora is a proof-native, GPU-accelerated presentation, interaction, and
media runtime for the TPT ecosystem (see `spec.txt` for the full design
document). It replaces the traditional browser rendering pipeline, consuming
zero-copy state from `tpt-archon`, proven geometry/semantics from
`tpt-eidos`, and driving interaction through `tpt-telos`.

This repo has a working prototype of all 16 planned phases (12 crates), plus
a Phase 17 hardening/security/adoption-tooling pass. See `todo.md` for the
full phased roadmap and `ARCHITECTURE.md` for what exists versus what is
still directional (hardware video decode, native OS accessibility bridge,
native haptics — all disclosed platform-integration gaps, not silent stubs).

## Workspace layout

```
spec.txt                 design doc (the full vision)
todo.md                  phased roadmap (source of truth for tasks)
ARCHITECTURE.md          what's built vs. directional, crate-by-crate detail
deny.toml                cargo-deny config (advisories/licenses/sources)
crates/
  tpt-chora-render/      Phase 1: render graph, vector tessellation, post-processing (+ optional `spatial` feature: stereo/foveation/volumetric graph nodes)
  tpt-chora-text/        Phase 2: SDF text shaping/atlas/sub-pixel rendering
  tpt-chora-spatial/     Phase 3: stereoscopic, foveated, volumetric, spatial-audio primitives
  tpt-chora-input/       Phase 4: device abstraction, GPU compute-shader hit testing, haptics
  tpt-chora-a11y/        Phase 5: semantic IR, OS accessibility bridge, focus traversal
  tpt-chora-media/       Phase 6: image/video decode, asset streaming
  tpt-chora-runtime/     Phase 7: eidos/archon/telos integration contracts
  tpt-chora-inspector/   Phase 10: Inspector overlay, GPU timing, dirty-rect heatmap, hot reload
  tpt-chora-fallback/    Phase 11: software rasterization, headless output, dynamic fidelity
  tpt-chora-compat/      Phase 12-13: HTML/CSS transpiler, Wasm/FFI interop, web component, CDN bootstrap
  tpt-chora-all-subsystems/  Phase 14: full end-to-end demo binary
  tpt-chora-cli/         Phase 16: doctor/new/css-report/completions/preview adoption tooling
    examples/            runnable milestone demos (cargo run -p tpt-chora-render --example <name>)
```

## Pipeline (Phase 1 scope)

`Renderer::render_frame` builds a `RenderGraph` each frame: a `scene` node
(draws geometry, `creates` the `scene_color` transient texture) followed by
a `postprocess` node (`reads` `scene_color`, `creates` `final_color`). The
graph topologically sorts nodes by their declared resource dependencies,
allocates transient textures once, and records all passes into one
`CommandEncoder` submission. `GpuContext::new_headless` requests a wgpu
adapter/device with no window surface, so the same path doubles as the
foundation for the Tier 2 headless fallback described in spec.txt.

## Conventions

- New crates are named `tpt-chora-<name>` and live at
  `crates/tpt-chora-<name>` (package name matches directory name, both
  carrying the `tpt-chora` prefix).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo build --workspace` must stay clean before finishing a change.
  CI also runs `cargo test --workspace` (plus `-p tpt-chora-render --features
  spatial`) and `cargo deny check` — run these locally when touching
  dependencies or anything gated behind the `spatial` feature.
- Do not add comments to code unless asked, or unless the *why* is
  genuinely non-obvious (a hidden constraint, a workaround, a subtle
  invariant).
- This is a Windows dev environment: building `tpt-chora-render` requires a
  working MSVC linker (Visual Studio Build Tools, "Desktop development with
  C++" workload) — the bundled GNU/mingw Rust toolchain's linker is
  link-only and cannot build wgpu's Windows bindings.
