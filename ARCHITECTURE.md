# ARCHITECTURE.md — tpt-chora

`tpt-chora` is the presentation/interaction/media runtime of the TPT
trinity: it consumes proven layout/semantics from `tpt-eidos`, zero-copy
state from `tpt-archon`, and drives interaction through `tpt-telos`. See
`spec.txt` for the full design document; this file tracks what actually
exists in this repo today versus what is still directional.

## Workspace layout

```
spec.txt                    design doc (the full vision, six subsystems +
                             fallback/migration/deployment strategy)
todo.md                     phased roadmap (source of truth for tasks)
crates/
  tpt-chora-render/          Phase 1: The Core Rendering Engine ("The Canvas")
    src/graph.rs             frame-scoped, dependency-tracked render graph
    src/vector.rs             GPU compute-shader cubic-Bezier tessellation
    src/postprocess.rs        color-grading post-process pass
    src/renderer.rs            headless GpuContext + Renderer wiring it together
    src/shaders/               WGSL: scene.wgsl, tessellate.wgsl, postprocess.wgsl
    examples/triangle_and_path.rs   Phase 1 milestone (renders to PNG)
```

## Phase 1: The Core Rendering Engine

- **Render graph** (`graph.rs`): nodes declare the transient textures they
  `reads`/`writes`/`creates` by name (`ResourceId`); the graph topologically
  sorts nodes (Kahn's algorithm over a producer/consumer edge set) so every
  writer runs before its readers, allocates each declared transient texture
  once per `execute`, and records every node's pass into a single
  `CommandEncoder`/submission per frame.
- **Vector tessellation** (`vector.rs`): cubic Bezier curves are tessellated
  on the GPU via a compute shader (`shaders/tessellate.wgsl`) that evaluates
  the Bernstein/De Casteljau closed form per sample point in parallel, never
  on the CPU. `circle_path` builds the four-curve cubic-Bezier approximation
  of a circle used by the milestone example.
- **Post-processing** (`postprocess.rs`): a single fullscreen color-grading
  pass (exposure/contrast/saturation/gamma) driven by a `ColorGradeParams`
  struct. This is the slot bloom/DoF/motion-blur will occupy once
  camera-space scene data (depth, velocity buffers) exists to drive them;
  `ColorGradeParams` stands in for the visual constraints tpt-eidos will
  drive once the eidos integration (Phase 7) lands.
- **Renderer** (`renderer.rs`): `GpuContext::new_headless` requests a wgpu
  adapter/device with no surface (headless), which doubles as the
  foundation for the Tier 2 headless fallback (spec.txt fallback strategy).
  `Renderer::render_frame` builds the graph (`scene` node -> `postprocess`
  node), executes it, and reads the final texture back to CPU-side RGBA8
  bytes via a staging buffer + `map_async`.

## Conventions

- `cargo fmt --all -- --check` and `cargo build --workspace` should stay
  clean.
- New crates: add to root `Cargo.toml` `[workspace.members]` and
  `[workspace.dependencies]`, named `tpt-chora-<name>` (matching the
  directory name `crates/tpt-chora-<name>`, per repo convention).
- Do not add comments to code unless the *why* is non-obvious.

## What's still directional

Everything past Phase 1 in `todo.md`: typography/text, spatial/3D, input,
accessibility, media, the tpt-eidos/tpt-archon/tpt-telos integration
contracts, security/capabilities, the zero-allocation performance model, the
Chora Inspector/hot-reload DX, the three-tier hardware fallback, the
migration path (`<tpt-chora>` web component, CSS transpiler, Wasm/FFI), and
CDN/WebGPU-bootstrap deployment.
