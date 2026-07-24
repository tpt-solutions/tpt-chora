# CLAUDE.md

This file mirrors AGENTS.md for coding agents that read CLAUDE.md. See
AGENTS.md for the authoritative workspace documentation.

tpt-chora: proof-native, GPU-accelerated presentation/interaction/media
runtime. Phase 1 (in progress) = `tpt-chora-render` (render graph, GPU
vector tessellation, post-processing), a wgpu-backed headless `Renderer`.
See `todo.md` for the full 14-phase roadmap and `ARCHITECTURE.md` for
built-vs-directional detail.

Conventions: new crates named `tpt-chora-<name>` at
`crates/tpt-chora-<name>`. Keep `cargo fmt --all -- --check` and
`cargo build --workspace` clean before finishing. Building requires a
working MSVC linker (VS Build Tools C++ workload) on this Windows machine.
