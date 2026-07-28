# tpt-chora-runtime

Integration contracts for [tpt-chora](https://github.com/tpt-solutions/tpt-chora) ("The TPT Trinity"): eidos/archon/telos integration points and zero-copy GPU handle types, bridging [`tpt-chora-render`](https://crates.io/crates/tpt-chora-render) and [`tpt-chora-a11y`](https://crates.io/crates/tpt-chora-a11y).

## Status

Part of tpt-chora's Phase 1 rendering engine. API is pre-1.0 and may change.

The `tpt-eidos`, `tpt-archon`, and `tpt-telos` ecosystem crates this crate is designed to integrate with are still under external development and not yet published; this crate currently uses internal stubs (`archon_stub`, `telos_stub`) in their place. Those stubs will be swapped for real implementations once the upstream crates are available — expect breaking changes at that point.

See the workspace [ARCHITECTURE.md](https://github.com/tpt-solutions/tpt-chora/blob/master/ARCHITECTURE.md) for details.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
