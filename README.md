# tpt-chora

**Proof-native, GPU-accelerated presentation, interaction, and media runtime
for the TPT ecosystem.**

`tpt-chora` entirely replaces the traditional browser rendering pipeline: it
natively handles 2D/3D UI rendering, complex typography, spatial computing
(XR), hardware-accelerated media, multi-modal input, and OS-level
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

## Status

All 16 planned phases have a working prototype (Phase 0 scaffold through
Phase 16 adoption tooling), plus a Phase 17 hardening pass. A few
integration points are honestly still facades pending platform-specific
bindings — hardware video decode, the native OS accessibility bridge, and
native haptics — see `ARCHITECTURE.md`'s "still directional" section before
relying on those specifically.

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
| `tpt-chora-fallback` | Three-Tier Hardware Fallback: software rasterization, headless output, dynamic fidelity |
| `tpt-chora-compat` | Migration & Adoption: HTML/CSS transpiler, Wasm/FFI interop, embeddable web component |
| `tpt-chora-all-subsystems` | Full end-to-end demo binary exercising every subsystem |
| `tpt-chora-cli` | Adoption Tooling: `doctor` diagnostics, `new` project scaffolding, `css-report`, shell completions, `preview` dev loop |

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
