# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `tpt-chora-media`: real software MJPEG frame decoding (`decode.rs` —
  bitstream parsing, Huffman decode, YCbCr upsampling, IDCT, RGB
  conversion) with unit tests; the VA-API branch stays a documented
  handshake-only backend.
- `tpt-chora-a11y`: `native-a11y-backends` now exposes a real Windows UI
  Automation provider (`bridge.rs`) — a fragment root hosted on a
  message-only window answering `WM_GETOBJECT`/`UiaReturnRawElementProvider`,
  implementing `IRawElementProviderSimple`/`Fragment`/`FragmentRoot` with
  `Navigate`, `GetPropertyValue`, `GetRuntimeId`, focus, and live-region
  announce over SAFEARRAY/BSTR, instead of the previous desktop-element
  placeholder. Verified with `cargo check`/`clippy -D warnings`/29 tests
  under the feature.
- `tpt-chora-fallback`: genuine Tier 1 CPU/software rasterization
  (`software.rs` — span-based triangle scanline fill, painter's-algorithm
  depth, source-over blending, linear gradients) plus a PPM/ASCII encoder
  (`encoding.rs`), wired into three-tier fallback selection.
- `tpt-chora-inspector`: `GpuTimer::readback` computes `elapsed_ns` from
  real GPU-reported timestamps (resolved buffer mapped via
  `map_async`/`get_mapped_range`); pure helpers extracted and unit-tested.

### Fixed
- `tpt-chora-compat`: bumped `wasmtime` from 43.x to 46.x — 43.0.2 had an
  open RustSec advisory (RUSTSEC-2026-0222) with no patched 43.x release;
  the `FfiBridge` API surface required no source changes.
- `tpt-chora-a11y`: `bridge.rs` Windows UIA provider builds clean under
  `clippy -- -D warnings` (`Send`/`Sync` on the shared provider state is an
  explicit `unsafe impl` justified by the COM apartment model), and its
  tests cover navigation, stable provider identity, and stale-provider
  pruning.

### Changed
- `ARCHITECTURE.md`: Phase 4 device-abstraction section now states that
  gaze/eye-tracking are declared capabilities only — the input pipeline
  consumes a simulated gaze position, not hardware eye-tracking.

### Added
- New workspace crate `crates/tpt-chora-bench` (Phase 18): `criterion`
  benchmarks for GPU Bezier-tessellation throughput (10k/50k/100k curves)
  and zero-copy `ChoraRuntime::bind_state_to_gpu` ingestion (4KB/64KB/1MB
  payloads), a plain frame-pacing benchmark reporting mean/p95/p99 frame
  times, and a `dhat`-backed steady-state heap-growth test (feature
  `dhat-heap`) that catches per-frame leaks automatically. Wired into CI as
  a `benchmarks` job running in criterion's fast `--test` mode (a
  build/execution regression signal, not a perf-threshold gate — shared CI
  runners are too noisy for stable statistical baselines).

### Fixed
- `tpt-chora-render`: `RenderGraph::execute`'s security guard validated
  every node's texture reads/writes against `CapabilityGuard` but nothing
  ever granted access in the first place — every render call (including
  the Phase 1 milestone example, `triangle_and_path`) failed with
  `SecurityViolation("component 0 denied access to texture ...")`. A node
  is now implicitly granted access to the resources it `creates` before
  validation runs, matching the intended "producer owns what it makes"
  model (`crates/tpt-chora-render/src/graph.rs`,
  `src/security/capability.rs`).
- `tpt-chora-compat`: bumped `wasmtime` from 24.0.11 to 43.x — the pinned
  24.0.11 had six open RustSec advisories including multiple Wasm-sandbox
  escapes (RUSTSEC-2026-0086/0088/0089/0094/0095/0096), caught by the new
  `cargo deny check` CI job.

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
