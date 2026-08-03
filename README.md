# tpt-chora

**Proof-native, GPU-accelerated presentation, interaction, and media runtime
for the TPT ecosystem.**

`tpt-chora` entirely replaces the traditional browser rendering pipeline: it
natively handles 2D/3D UI rendering, complex typography, stereoscopic
rendering, hardware-accelerated media, multi-modal input, and OS-level
accessibility. It operates strictly as the "receptacle," reading zero-copy
memory from [`tpt-archon`](https://github.com/tpt-solutions/tpt-archon)
(the substrate), consuming proven geometry and semantics from
[`tpt-eidos`](https://github.com/tpt-solutions/tpt-eidos) (form/layout), and
executing stateful interactions via
[`tpt-telos`](https://github.com/tpt-solutions/tpt-telos) (logic/state).

See `spec.txt` for the full design document, `todo.md` for the phased build
roadmap, and `ARCHITECTURE.md` for a crate-by-crate account of what's
actually implemented versus still directional (platform-specific hardware
integrations that need real OS/device bindings).

## Prerequisites

- **Rust**: MSRV 1.74 (see `Cargo.toml` `rust-version`)
- **GPU driver**: A Vulkan/Metal/DX12/WebGPU-capable GPU adapter.
  On Linux without hardware GPU, install Mesa's software Vulkan
  driver (`mesa-vulkan-drivers` on Debian/Ubuntu) — `wgpu` will
  automatically fall back to lavapipe via `force_fallback_adapter`.
- **Windows**: MSVC linker (Visual Studio Build Tools, "Desktop
  development with C++" workload) — the bundled GNU/mingw Rust
  toolchain's linker cannot build wgpu's Windows bindings.
- **macOS**: Xcode command-line tools (`xcode-select --install`).
- **Android**: Android NDK + JDK (for `native-haptics-backends` and
  `native-video-backends` features).

## Status

All 16 planned phases have a working prototype (Phase 0 scaffold through
Phase 16 adoption tooling), plus a Phase 17 hardening pass. Platform-specific
hardware integrations now have real, feature-gated, best-effort implementations:
`native-a11y-backends` (Windows UIA verified, macOS/Android unverified),
`native-haptics-backends` (macOS CoreHaptics unverified, Android unverified),
and `native-video-backends` (Linux VA-API CI-verified, macOS/Android unverified).
See `ARCHITECTURE.md` for details.

## Workspace crates

| Crate | Purpose |
| --- | --- |
| `tpt-chora-render` | Core Rendering Engine ("The Canvas"): wgpu render graph, GPU vector tessellation, post-processing |
| `tpt-chora-text` | Typography & Text Engine ("The Voice"): SDF font atlas, text shaping, sub-pixel rendering |
| `tpt-chora-spatial` | Spatial & 3D Engine ("The Depth"): stereoscopic rendering, foveated rendering, volumetric lighting |
| `tpt-chora-input` | Input & Interaction Engine ("The Senses"): unified device abstraction, hit testing, haptics |
| `tpt-chora-a11y` | Accessibility & Semantics Engine ("The Bridge"): semantic IR, OS accessibility bridge, focus traversal |
| `tpt-chora-media` | Media & Asset Pipeline ("The Content"): async image/video decoding, asset streaming |
| `tpt-chora-runtime` | Integration Contracts ("The TPT Trinity"): eidos/archon/telos integration, zero-copy GPU binding |
| `tpt-chora-inspector` | Developer Experience: Chora Inspector overlay, GPU timing, dirty-rect heatmap, hot reload |
| `tpt-chora-fallback` | Fallback: headless PNG/JPEG output and dynamic fidelity scaling |
| `tpt-chora-compat` | Migration & Adoption: HTML/CSS transpiler, Wasm/FFI interop, embeddable web component |
| `tpt-chora-all-subsystems` | Full end-to-end demo binary exercising every subsystem |
| `tpt-chora-bench` | Benchmarking: tessellation throughput, frame pacing, zero-allocation proof, archon ingestion |
| `tpt-chora-cli` | Adoption Tooling: `doctor` diagnostics, `new` project scaffolding, `css-report`, `audit`, shell completions, `preview` dev loop |

## Try it

```sh
# The Phase 1 milestone: a triangle and a GPU-tessellated vector path,
# rendered off-screen through the render graph and post-process pass.
cargo run -p tpt-chora-render --example triangle_and_path

# The full end-to-end demo, exercising every subsystem in one binary.
cargo run -p tpt-chora-all-subsystems --bin all_subsystems_demo
```

## `tpt-chora-cli`

The `tpt-chora` binary (`cargo run -p tpt-chora-cli --`) provides adoption
tooling on top of the workspace:

```sh
# Diagnose your toolchain and GPU backend before filing a confusing bug report.
tpt-chora doctor

# Scaffold a minimal new crate that renders its first frame.
tpt-chora new my-app
cd my-app && cargo run

# Run the "Rosetta Stone" legacy CSS -> Chora-IR transpiler against a real
# stylesheet and see a compatibility score plus flagged violations.
tpt-chora css-report path/to/site.css

# Run a consolidated health and security audit: doctor diagnostics,
# cargo-deny check, and native-backend feature status.
tpt-chora audit

# Generate shell completions.
tpt-chora completions bash > /etc/bash_completion.d/tpt-chora

# Watch a project's .eidos/shader/asset files and re-render a PNG snapshot
# on every change — a live dev loop with no windowed app required.
tpt-chora preview ./my-app
```

## Security

`tpt-chora-render`'s `security` module enforces capability-token-gated
texture/buffer access and hierarchical Z-depth inside the render graph (see
`ARCHITECTURE.md`). `tpt-chora-compat`'s Wasm/FFI bridge runs untrusted
modules with an empty import linker (no host I/O), a fuel budget (traps
runaway loops instead of hanging), and a memory/table cap (traps runaway
allocation instead of exhausting host memory). Supply-chain scanning
(`cargo deny check`, see `deny.toml`) runs in CI alongside `fmt`/`clippy`/
`build`/`test`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
