# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
