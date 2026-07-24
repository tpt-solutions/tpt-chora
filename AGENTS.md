# AGENTS.md — tpt-chora

tpt-chora is a proof-native, GPU-accelerated presentation, interaction, and
media runtime for the TPT ecosystem (see `spec.txt` for the full design
document). It replaces the traditional browser rendering pipeline, consuming
zero-copy state from `tpt-archon`, proven geometry/semantics from
`tpt-eidos`, and driving interaction through `tpt-telos`.

This repo currently implements **Phase 1: the Core Rendering Engine ("The
Canvas")** — a wgpu render graph, GPU-compute vector tessellation, and a
post-processing pipeline. See `todo.md` for the full phased roadmap (14
phases, one per spec.txt subsystem/section) and `ARCHITECTURE.md` for what
exists versus what is still directional.

## Workspace layout

```
spec.txt                 design doc (the full vision)
todo.md                  phased roadmap (source of truth for tasks)
ARCHITECTURE.md          what's built vs. directional, crate-by-crate detail
crates/
  tpt-chora-render/      Phase 1: render graph, vector tessellation, post-processing
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
- `cargo fmt --all -- --check` and `cargo build --workspace` must stay
  clean before finishing a change.
- Do not add comments to code unless asked, or unless the *why* is
  genuinely non-obvious (a hidden constraint, a workaround, a subtle
  invariant).
- This is a Windows dev environment: building `tpt-chora-render` requires a
  working MSVC linker (Visual Studio Build Tools, "Desktop development with
  C++" workload) — the bundled GNU/mingw Rust toolchain's linker is
  link-only and cannot build wgpu's Windows bindings.
