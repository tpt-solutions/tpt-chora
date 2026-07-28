# tpt-chora-render

Core rendering engine for [tpt-chora](https://github.com/tpt-solutions/tpt-chora) ("The Canvas"): a wgpu-backed render graph, GPU vector tessellation (Bezier/path), and a post-processing pipeline (color grading, etc.), driven by a headless `Renderer`.

## Status

Part of tpt-chora's Phase 1 rendering engine. API is pre-1.0 and may change.

## Features

- `spatial` — enables optional integration with [`tpt-chora-spatial`](https://crates.io/crates/tpt-chora-spatial) for stereoscopic/3D rendering.

See the workspace [ARCHITECTURE.md](https://github.com/tpt-solutions/tpt-chora/blob/master/ARCHITECTURE.md) for how this crate fits into the rest of tpt-chora.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
