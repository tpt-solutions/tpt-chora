# tpt-chora TODO

## Phase 0: Project Scaffold & Licensing
- [x] Create Cargo workspace root (`Cargo.toml`, `resolver = "2"`, `crates/` members list)
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` (dual license, copyright TPT Solutions)
- [x] Add `README.md`
- [x] Add `CLAUDE.md`
- [x] Add `AGENTS.md`
- [x] Add `ARCHITECTURE.md`
- [x] Add `CHANGELOG.md`
- [x] Add `.gitignore` and `git init`
- [x] Add `.github/` CI workflow scaffold
- [x] **Milestone:** `cargo build` succeeds on an empty workspace skeleton.

## Phase 1: The Core Rendering Engine (The Canvas)
- [x] Integrate wgpu (Vulkan/Metal/DX12/WebGPU backends) (`crates/tpt-chora-render/src/renderer.rs`)
- [x] Build the frame-scoped, dependency-tracked render graph (transient resources, pass optimization, GPU stall elimination) (`crates/tpt-chora-render/src/graph.rs`)
- [x] Implement native GPU-tessellated vector graphics (compute-shader Bezier/path tessellation, replacing SVG) (`crates/tpt-chora-render/src/vector.rs`, `src/shaders/tessellate.wgsl`)
- [x] Build the post-processing pipeline (color grading) driven by tunable visual-constraint parameters (`crates/tpt-chora-render/src/postprocess.rs`, `src/shaders/postprocess.wgsl`) — bloom/depth-of-field/motion-blur left as additional passes in the same slot, pending camera-space depth/velocity buffers
- [x] **Milestone:** Render a triangle and a tessellated vector path through the render graph to an off-screen target. (`cargo run -p tpt-chora-render --example triangle_and_path`)

## Phase 2: The Typography & Text Engine (The Voice)
- [ ] Integrate tpt-eidos text shaping/layout (rustybuzz-based shaper: kerning, ligatures, RTL/bidi), proving text fits its bounding box
- [ ] Implement SDF font atlas pre-compilation and infinite-scale single-fragment-shader rendering
- [ ] Add native sub-pixel LCD rendering and gamma-correct blending
- [ ] Support complex scripts (Indic, Arabic, CJK) natively, without OS text rasterizer fallback
- [ ] **Milestone:** Render a paragraph mixing LTR/RTL/CJK text from SDF atlases, scaled across multiple sizes with no pixelation.

## Phase 3: The Spatial & 3D Engine (The Depth)
- [ ] Implement stereoscopic rendering (dual-view, automated asymmetric frustum projection for VR/AR)
- [ ] Integrate foveated rendering driven by eye-tracking
- [ ] Implement volumetric lighting/shadows (GPU compute shadow mapping, real-time global illumination) respecting tpt-eidos `elevation`
- [ ] Integrate spatial audio (3D-positioned sources, HRTF via head tracking; rodio/OpenAL)
- [ ] **Milestone:** A stereoscopic scene with real-time shadows and a head-tracked spatial audio source running at target XR framerate.

## Phase 4: The Input & Interaction Engine (The Senses)
- [ ] Build unified device abstraction (mouse, keyboard, touch, pen/stylus, gamepad, XR controllers) into one capability-based event stream
- [ ] Integrate gaze/gesture tracking and intent abstraction (Gaze+Pinch, Mouse Down, Touch Tap)
- [ ] Implement GPU-accelerated hit testing (depth buffer + bounding volume hierarchies, no DOM traversal)
- [ ] Route haptics to OS-level APIs (CoreHaptics, Android Vibrator, XR controller rumble)
- [ ] **Milestone:** A hit-test demo where mouse, touch, and simulated gaze+pinch all resolve to the same component intent.

## Phase 5: The Accessibility & Semantics Engine (The Bridge)
- [ ] Consume tpt-eidos's parallel Semantic IR (`ChoraSemanticNode`: role, state, relationships) alongside the visual Chora-IR
- [ ] Build the live, two-way OS accessibility bridge (UIAutomation on Windows, Accessibility API on macOS, UIAutomator on Android)
- [ ] Integrate tpt-eidos's proven focus-traversal algorithm (logically sound, never traps the user)
- [ ] **Milestone:** A screen reader (or OS accessibility inspector) correctly reads roles/states/labels from a rendered scene with no visual DOM.

## Phase 6: The Media & Asset Pipeline (The Content)
- [ ] Implement async, multi-threaded image decoding (JPEG, PNG, WebP, AVIF) directly into GPU-ready texture formats
- [ ] Integrate hardware video decoding (VA-API, VideoToolbox, MediaCodec) with zero-copy GPU texture mapping for 4K/8K playback
- [ ] Implement predictive asset streaming (pre-fetch textures/meshes for components just outside the viewport, from Chora-IR analysis)
- [ ] **Milestone:** Play a 4K video and stream in off-viewport textures/meshes with no dropped frames.

## Phase 7: Integration Contracts (The TPT Trinity)
- [ ] Consume `ChoraVisualNode` from tpt-eidos (transform, geometry, material, clip_mask, z_depth)
- [ ] Wire the shared `ChoraSemanticNode` IR types (builds on Phase 5)
- [ ] Implement zero-copy `ArchonPage` → GPU buffer mapping (`bind_state_to_gpu`) — stub the `ArchonPage` type locally until `tpt-archon` can be vendored from https://github.com/tpt-solutions/tpt-archon, then swap in the real crate
- [ ] Wire input → tpt-telos state transitions (`on_interaction`: hit test confirms target via Eidos proofs → `telos.process_event` → `archon.apply_mutation` → next-frame Eidos re-proof/Chora re-render)
- [ ] **Milestone:** A full round-trip demo — a click fires a telos transition, mutates archon-backed memory, and next frame re-renders via re-proved eidos layout.

## Phase 8: Security, Sandboxing, & Capabilities
- [ ] Enforce hardware viewport isolation (GPU scissor test + stencil buffers per component, no drawing outside the tpt-eidos-proven bounding box)
- [ ] Enforce capability-token-gated memory access (shaders can only sample textures/read uniform buffers their token explicitly allows)
- [ ] Implement strictly hierarchical Z-depth (no global z-index; require a tpt-telos-granted "Modal" capability to break spatial hierarchy)
- [ ] Confirm tpt-chora has zero network/filesystem capabilities (all data fetching delegated to tpt-archon)
- [ ] **Milestone:** An untrusted component's shader is denied access to a texture it lacks a capability token for, and cannot draw outside its bounding box.

## Phase 9: Performance & Memory Model
- [ ] Audit the render loop (input processing → GPU command buffer submission) for zero heap allocation
- [ ] Implement double/triple buffering with lock-free atomic front/back-buffer swap at VBlank
- [ ] Implement incremental re-rendering via tpt-eidos "dirty rect" diffing (only re-rasterize/re-submit changed regions)
- [ ] **Milestone:** Sustained 120fps (90fps in XR) on a representative scene with a heap-allocation profiler showing zero allocations in the steady-state loop.

## Phase 10: Developer Experience (DX)
- [ ] Build the Chora Inspector overlay: live Semantic (A11y) Tree view
- [ ] Chora Inspector: GPU draw call and shader execution time inspection
- [ ] Chora Inspector: dirty-rect and overdraw heatmap visualization
- [ ] Chora Inspector: forced color-blindness / high-contrast proof toggles
- [ ] Implement hot reloading (eidos file change → instant IR recompile → GPU pipeline update without losing application state)
- [ ] **Milestone:** Edit a `.eidos` file while the app is running and see the layout update live, with the Inspector showing before/after dirty rects.

## Phase 11: Three-Tier Hardware Fallback Strategy
- [ ] Tier 1 — Software rasterization fallback (lavapipe/LLVMpipe, DXC) when no hardware Vulkan/Metal/DX12 adapter is found
- [ ] Tier 2 — Headless/server-side mode (null windowing context, off-screen framebuffer, PNG/JPEG/video-frame output over the network)
- [ ] Tier 3 — Dynamic fidelity: query weak-GPU capabilities and instruct tpt-eidos to switch to a "Low Power" profile (shadows/post-processing disabled, SDF→bitmap font fallback, 30/60fps cap), proven still-compliant by tpt-eidos
- [ ] **Milestone:** The same app runs correctly (degraded but functional) on a software-rasterizer-only CI runner and in headless PNG-output mode.

## Phase 12: Migration & Adoption Path
- [ ] Build the `<tpt-chora>` embeddable Web Component ("Trojan Horse" strategy) for dropping into existing React/Vue/HTML apps
- [ ] Build the "Rosetta Stone" legacy HTML/CSS → Chora-IR transpiler via the tpt-eidos compiler, with safety-proof violation warnings and auto-correction
- [ ] Implement WebAssembly/FFI interop via tpt-telos so existing JS/Python/Java business logic can bridge into tpt-archon's zero-copy memory without a rewrite
- [ ] **Milestone:** An existing React app embeds `<tpt-chora>` for one component, and a sample legacy CSS file transpiles with a reported auto-corrected violation.

## Phase 13: Deployment on Existing Infrastructure
- [ ] Confirm compiled output is standard Wasm + WGSL/SPIR-V + binary assets, deployable to standard CDNs (Cloudflare, AWS S3, Vercel, Fastly) over HTTP/3
- [ ] Build the "WebGPU Bootstrap": a ~500kb loader script that initializes a `<canvas>`, fetches the tpt-chora Wasm/shaders, and boots the runtime inside existing Chrome/Edge/Safari/Firefox (bypassing the DOM)
- [ ] Confirm tpt-archon backend adapters (Postgres, Redis, S3, SQLite, HTTP/3, gRPC, WebSockets) work against an existing Node.js/Go/Rust backend without requiring a backend rewrite
- [ ] **Milestone:** A tpt-chora app is served from a CDN, bootstrapped inside an unmodified Chrome tab via WebGPU, with DOM bypass confirmed via DevTools inspection.

## Phase 14: v1.0 Release
- [ ] Build a full end-to-end demo app exercising every subsystem (2D/3D UI, XR path, video, accessibility, fallback tiers, migration component) as an `examples/` crate
- [ ] Documentation pass (`README.md`, `ARCHITECTURE.md`, migration guide)
- [ ] **Milestone:** tpt-chora v1.0 tagged, ready for internal use downstream (e.g. alongside tpt-telos, other TPT apps).
