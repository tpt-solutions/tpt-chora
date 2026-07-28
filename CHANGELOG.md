# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `tpt-chora-render`: `spatial` feature nodes (`create_stereo_node`,
  `create_foveation_node`, `create_volumetric_node`) now render real,
  nonzero geometry, actually use the computed foveation level to size a
  shadow-map resource, and are exercised by both new tests and the
  `tpt-chora-all-subsystems` demo (previously dead code with zero callers).
- `tpt-chora-input`: `GpuHitTest::hit_test_gpu` dispatches a real WGSL
  compute shader (`shaders/hit_test.wgsl`) against its GPU buffers instead
  of scanning a `HashMap` on the CPU.
- `tpt-chora-compat`: `FfiBridge` now enforces a fuel budget and a
  memory/table limiter on every loaded Wasm module, so an untrusted module
  can't hang or OOM the host process.
- `tpt-chora-cli`: `new <name>` validates the project name before touching
  the filesystem (rejects path traversal/absolute paths/invalid
  characters); added `completions <shell>` and `preview <project-dir>`
  subcommands.
- `deny.toml` + a `cargo-deny` CI job for supply-chain vulnerability/license
  scanning; CI now also runs `cargo test --workspace` (previously
  build-only) using Mesa's software Vulkan driver.

### Changed
- `README.md`/`AGENTS.md` now document all 12 workspace crates and the
  `tpt-chora-cli` subcommands (previously described Phase 1/one crate only).
- Workspace scaffold: dual MIT/Apache-2.0 licensing, `README.md`,
  `ARCHITECTURE.md`, `AGENTS.md`, `CLAUDE.md`.
- `tpt-chora-render`: Phase 1, the Core Rendering Engine ("The Canvas").
  - `graph`: a frame-scoped, dependency-tracked render graph over
    transient wgpu textures, topologically sorted and executed in one
    command-buffer submission.
  - `vector`: GPU-compute-shader cubic-Bezier tessellation
    (`shaders/tessellate.wgsl`), plus a `circle_path` builder.
  - `postprocess`: a color-grading fullscreen pass
    (`shaders/postprocess.wgsl`) driven by `ColorGradeParams`.
  - `renderer`: headless `GpuContext`/`Renderer` wiring the graph, scene
    draw calls, and post-process pass together, with an off-screen
    RGBA8 readback path.
  - `examples/triangle_and_path.rs`: the Phase 1 milestone, rendering a
    triangle and a GPU-tessellated vector path to a PNG.
