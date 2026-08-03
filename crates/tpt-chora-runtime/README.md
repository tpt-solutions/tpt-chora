# tpt-chora-runtime

Integration contracts for [tpt-chora](https://github.com/tpt-solutions/tpt-chora) ("The TPT Trinity"): eidos/archon/telos integration points and zero-copy GPU handle types, bridging [`tpt-chora-render`](https://crates.io/crates/tpt-chora-render) and [`tpt-chora-a11y`](https://crates.io/crates/tpt-chora-a11y).

## Status

Part of tpt-chora's Phase 1 rendering engine. API is pre-1.0 and may change.

The real `tpt-eidos`, `tpt-archon`, and `tpt-telos` sibling projects were investigated (2026-07-30) and found to implement unrelated domains (numeric refinement-type verification; embedded storage/kernel/SQL stack; DSL verification/codegen compiler, respectively) — none implement UI layout proofs, zero-copy paged GPU-bindable state, or UI event/state-machine processing. This crate's local `archon_stub`/`telos_stub` are therefore the permanent implementations of these roles, not placeholders awaiting a swap.

See the workspace [ARCHITECTURE.md](https://github.com/tpt-solutions/tpt-chora/blob/master/ARCHITECTURE.md) for details.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
