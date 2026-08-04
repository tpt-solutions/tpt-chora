# tpt-chora-fallback

Headless output and dynamic fidelity scaling for [tpt-chora](https://github.com/tpt-solutions/tpt-chora).

This crate provides three fallback tiers when full GPU acceleration isn't available:
- **Software rasterizer** (`software.rs`): a genuine CPU rasterizer rendering an immediate-mode scene graph of filled/outlined primitives (rects, circles, triangles, lines, clip stacks) into an RGBA8 framebuffer with supersampled antialiasing and source-over alpha blending. No GPU adapter, windowing, or `wgpu` involved — it runs anywhere.
- **Headless output** (`headless.rs`): off-screen PNG/JPEG/raw RGBA rendering via the core renderer.
- **Dynamic fidelity** (`dynamic_fidelity.rs`): 5-tier adaptive quality scaling (Ultra → Minimum) controlling shadows, post-processing, SDF fonts, FPS caps, shadow map sizes, texture limits, MSAA, volumetric lighting, and foveated rendering.

The automatic software-adapter selection for the GPU path (lavapipe/LLVMpipe) is
handled by `wgpu`'s `force_fallback_adapter` in `tpt-chora-render`; it is a
separate mechanism from the `SoftwareRenderer` above, which is the crate's own
CPU rasterization tier.

## Status

Part of tpt-chora's Phase 1 rendering engine. API is pre-1.0 and may change.

See the workspace [ARCHITECTURE.md](https://github.com/tpt-solutions/tpt-chora/blob/master/ARCHITECTURE.md) for how this crate fits into the rest of tpt-chora.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
