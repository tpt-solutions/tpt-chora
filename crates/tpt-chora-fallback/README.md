# tpt-chora-fallback

Headless output and dynamic fidelity scaling for [tpt-chora](https://github.com/tpt-solutions/tpt-chora).

This crate provides two fallback tiers when full GPU acceleration isn't available:
- **Headless output** (`headless.rs`): off-screen PNG/JPEG/raw RGBA rendering via the core renderer.
- **Dynamic fidelity** (`dynamic_fidelity.rs`): 5-tier adaptive quality scaling (Ultra → Minimum) controlling shadows, post-processing, SDF fonts, FPS caps, shadow map sizes, texture limits, MSAA, volumetric lighting, and foveated rendering.

The first fallback tier — automatic software-rasterizer selection (lavapipe/LLVMpipe) — is handled by `wgpu`'s `force_fallback_adapter` in `tpt-chora-render`, not in this crate.

## Status

Part of tpt-chora's Phase 1 rendering engine. API is pre-1.0 and may change.

See the workspace [ARCHITECTURE.md](https://github.com/tpt-solutions/tpt-chora/blob/master/ARCHITECTURE.md) for how this crate fits into the rest of tpt-chora.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
