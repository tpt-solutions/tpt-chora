# ARCHITECTURE.md — tpt-chora

`tpt-chora` is the presentation/interaction/media runtime of the TPT
trinity: it consumes proven layout/semantics from `tpt-eidos`, zero-copy
state from `tpt-archon`, and drives interaction through `tpt-telos`. See
`spec.txt` for the full design document; this file tracks what actually
exists in this repo today versus what is still directional.

## Workspace layout

```
spec.txt                    design doc (the full vision, six subsystems +
                             fallback/migration/deployment strategy)
todo.md                     phased roadmap (source of truth for tasks)
crates/
  tpt-chora-render/          Phase 1: The Core Rendering Engine ("The Canvas")
    src/graph.rs             frame-scoped, dependency-tracked render graph
    src/vector.rs             GPU compute-shader cubic-Bezier tessellation
    src/postprocess.rs        color-grading post-process pass
    src/renderer.rs            headless GpuContext + Renderer wiring it together
    src/framebuffer.rs         double/triple buffering with atomic swap
    src/security/              capability tokens, viewport isolation, z-depth
    src/shaders/               WGSL: scene.wgsl, tessellate.wgsl, postprocess.wgsl
    examples/triangle_and_path.rs   Phase 1 milestone (renders to PNG)
  tpt-chora-text/           Phase 2: The Typography & Text Engine ("The Voice")
    src/shaping.rs           rustybuzz text shaping (kerning, ligatures, RTL/bidi)
    src/atlas.rs             SDF font atlas pre-compilation (1024x1024)
    src/sdf.rs               GPU SDF text rendering pipeline
    src/subpixel.rs          sub-pixel LCD rendering, gamma-correct blending
    src/shaders/sdf_text.wgsl
  tpt-chora-spatial/        Phase 3: The Spatial & 3D Engine ("The Depth")
    src/stereoscopic.rs      dual-view stereoscopic rendering, asymmetric frustum
    src/foveated.rs          3-zone foveated rendering, gaze-position-driven quality selection (simulated gaze, not hardware eye-tracking)
    src/volumetric.rs        GPU compute volumetric lighting/shadows
    src/spatial_audio.rs     3D-positioned audio, HRTF, head tracking (rodio)
    src/shaders/             stereo.wgsl, volumetric.wgsl
  tpt-chora-input/          Phase 4: The Input & Interaction Engine ("The Senses")
    src/devices.rs           unified device abstraction (mouse/kb/touch/pen/gamepad)
    src/intent.rs            gesture tracking, intent resolution (gaze is simulated)
    src/hit_test.rs          GPU-accelerated BVH hit testing
    src/haptics.rs           haptic routing (CoreHaptics/Android; XR rumble is a documented stub)
  tpt-chora-a11y/           Phase 5: The Accessibility & Semantics Engine ("The Bridge")
    src/semantic.rs          Semantic IR (34 roles, 11 states)
    src/bridge.rs            two-way OS accessibility bridge
    src/focus.rs             focus-traversal algorithm (never traps user)
  tpt-chora-media/          Phase 6: The Media & Asset Pipeline ("The Content")
    src/decode.rs            async image decoding (JPEG/PNG/WebP) + GPU format conversion
    src/texture.rs           GPU texture cache with LRU eviction
    src/streaming.rs         predictive asset streaming (viewport-priority queue)
  tpt-chora-runtime/        Phase 7: Integration Contracts (The TPT Trinity)
    src/contracts.rs         ChoraVisualNode, ChoraSemanticNode, visual/semantic trees
    src/archon_stub.rs       ArchonPage zero-copy GPU binding (stub until tpt-archon)
    src/telos_stub.rs        TelosState event processing + EidosProof validation
  tpt-chora-inspector/      Phase 10: Developer Experience (DX)
    src/inspector.rs         Chora Inspector overlay (GPU pipeline, config)
    src/gpu_timing.rs        GPU timer queries
    src/dirty_rect.rs        dirty-rect tracking with auto-merge
    src/heatmap.rs           overdraw heatmap (8x8 cells)
    src/color_proof.rs       color-blindness simulation (6 modes)
    src/hot_reload.rs        file watcher (eidos/shader/asset change detection)
  tpt-chora-fallback/       Phase 11: Three-Tier Hardware Fallback
    src/headless.rs          headless PNG/JPEG/raw RGBA output
    src/dynamic_fidelity.rs  5-tier adaptive fidelity (Ultra→Minimum)
  tpt-chora-compat/         Phase 12: Migration & Adoption
    src/css_parser.rs        hand-written CSS tokenizer
    src/eidos_transpiler.rs  CSS→Chora-IR with safety-proof violation detection
    src/ffi_bridge.rs        Wasm/FFI interop bridge
    src/web_component.rs     <tpt-chora> embeddable web component
    src/deployment/bootstrap.rs  WebGPU Bootstrap JS generator
  tpt-chora-all-subsystems/ Phase 14: v1.0 demo binary
    all_subsystems.rs        exercises all 14 phases in a single main()
```

## Subsystem detail

### Phase 1: The Core Rendering Engine

- **Render graph** (`graph.rs`): nodes declare the transient textures they
  `reads`/`writes`/`creates` by name (`ResourceId`); the graph topologically
  sorts nodes (Kahn's algorithm over a producer/consumer edge set) so every
  writer runs before its readers, allocates each declared transient texture
  once per `execute`, and records every node's pass into a single
  `CommandEncoder`/submission per frame.
- **Vector tessellation** (`vector.rs`): cubic Bezier curves are tessellated
  on the GPU via a compute shader (`shaders/tessellate.wgsl`) that evaluates
  the Bernstein/De Casteljau closed form per sample point in parallel, never
  on the CPU. `circle_path` builds the four-curve cubic-Bezier approximation
  of a circle used by the milestone example.
- **Post-processing** (`postprocess.rs`): a single fullscreen color-grading
  pass (exposure/contrast/saturation/gamma) driven by a `ColorGradeParams`
  struct. This is the slot bloom/DoF/motion-blur will occupy once
  camera-space scene data (depth, velocity buffers) exists to drive them;
  `ColorGradeParams` stands in for the visual constraints tpt-eidos will
  drive once the eidos integration (Phase 7) lands.
- **Renderer** (`renderer.rs`): `GpuContext::new_headless` requests a wgpu
  adapter/device with no surface (headless), which doubles as the
  foundation for the Tier 2 headless fallback (spec.txt fallback strategy).
  `Renderer::render_frame` builds the graph (`scene` node -> `postprocess`
  node), executes it, and reads the final texture back to CPU-side RGBA8
  bytes via a staging buffer + `map_async`.
- **Double/triple buffering** (`framebuffer.rs`): `FrameBufferSet` manages
  N textures (2 or 3) with lock-free `AtomicUsize` front/back index swap.
  `swap()` exchanges front and back; `swap_next_triple()` advances through
  three buffers. `read_back_rgba` reads the front buffer to CPU.
- **Security** (`security/`): `CapabilityGuard` validates shader access via
  bitflag tokens; `ViewportGuard` enforces scissor/stencil isolation per
  component; `HierarchicalZDepth` computes z from parent+sibling index with
  modal-capability break-glass.

### Phase 2: The Typography & Text Engine

- **Text shaping** (`shaping.rs`): rustybuzz shapes text with kerning,
  ligatures, and complex scripts. `unicode-bidi` handles RTL paragraph
  reordering. Returns `Vec<ShapedGlyph>` with codepoint, glyph_id, and
  scaled positions.
- **SDF font atlas** (`atlas.rs`): `SdfAtlasBuilder` rasterizes glyphs via
  `ab_glyph`, computes a two-pass (horizontal + vertical) signed distance
  field with configurable spread, and packs them into a 1024x1024 atlas.
- **SDF text pipeline** (`sdf.rs`): wgpu render pipeline that samples the
  atlas with `smoothstep` threshold, enabling infinite-scale rendering from
  a single texture.
- **Sub-pixel rendering** (`subpixel.rs`): gamma-correct sub-pixel vertex
  generation with configurable RGB distribution and gamma value.

### Phase 3: The Spatial & 3D Engine

- **Stereoscopic rendering** (`stereoscopic.rs`): dual left/right pipelines
  with depth textures and backface culling. `create_stereo_views` computes
  asymmetric frustum projection for VR/AR based on eye separation and
  convergence distance.
- **Foveated rendering** (`foveated.rs`): 3 concentric quality zones
  (inner/mid/outer) plus full fallback. `compute_foveation_level` selects
  quality from a simulated gaze position; `get_shadow_map_size` returns resolution per
  level (2048/1024/512/256).
- **Volumetric lighting** (`volumetric.rs`): GPU compute shader with ray
  marching, Henyey-Greenstein phase function, scatter/absorption
  coefficients, and depth-aware density.
- **Spatial audio** (`spatial_audio.rs`): `SpatialAudioEngine` manages
  listener position + `HashMap<u64, AudioSource>`. `compute_hrtf` returns
  azimuth, elevation, distance, gain, ITD, and near-field gain. rodio for
  playback.

### Phase 4: The Input & Interaction Engine

- **Device abstraction** (`devices.rs`): `DeviceCapability` bitflags for
  mouse/keyboard/touch/pen/gamepad/eye-tracking/gesture. `DeviceEvent`
  enum covers all input types. State structs track current state per device.
- **Intent resolution** (`intent.rs`): `IntentResolver` with double-click
  detection (300ms), long-press (500ms), drag threshold (5px). Maps
  `GestureIntent` to `InteractionIntent` (20 variants).
- **BVH hit testing** (`hit_test.rs`): `BoundingBoxHierarchy` with
  median-split tree building, recursive point/region queries. `GpuHitTest`
  wraps GPU buffers for hardware-accelerated hit testing.
- **Haptic routing** (`haptics.rs`): `HapticRouter` detects platform and
  maps `DeviceEvent`s to `HapticPattern`s. Platform-specific methods
  (CoreHaptics, Android Vibrator) are implemented; XR rumble is a documented
  stub pending an XR session API.

### Phase 5: The Accessibility & Semantics Engine

- **Semantic IR** (`semantic.rs`): `SemanticIR` tree of `SemanticNode`s with
  34 `AccessibilityRole`s and 11 `AccessibilityState` flags. Serialized to
  `Vec<BridgeNode>` for the OS bridge.
- **OS bridge** (`bridge.rs`): `A11yBridge` manages live tree updates, focus,
  and announcements. Validates focused node exists in each update.
- **Focus traversal** (`focus.rs`): `FocusTraversal` with depth-first order
  computation (excluding hidden nodes). `move_focus` handles linear
  (forward/backward/first/last) and 2D spatial (up/down pick nearest by
  horizontal distance).

### Phase 6: The Media & Asset Pipeline

- **Image decoding** (`decode.rs`): `ImageDecoder` detects format by magic
  bytes (PNG, JPEG, WebP), decodes via `image` crate, always outputs RGBA8.
  `decode_to_gpu_format` converts to target wgpu formats (Bgra8, R8Unorm,
  Rgba16Float).
- **Texture cache** (`texture.rs`): `GpuTextureCache` with LRU eviction by
  `last_used_frame`. Creates wgpu textures, evicts oldest if over budget.
- **Asset streaming** (`streaming.rs`): `AssetStreamer` with `BinaryHeap`
  priority queue. `enqueue_viewport_prefetch` adds Low priority with expanded
  viewport bounds; `prioritize_viewport` promotes overlapping requests.

### Phase 7: Integration Contracts

- **Visual/semantic nodes** (`contracts.rs`): `ChoraVisualNode` (transform
  Mat4, geometry/material/clip_mask handles, z_depth, bounds, visible).
  `ChoraSemanticNode` (role, label, state, bounding_box_2d, children).
  Arena-allocated tree structures with parent/child tracking.
- **Archon stubs** (`archon_stub.rs`): `ArchonPage` (id, data bytes, layout,
  dirty flag). `ChoraRuntime` with `bind_state_to_gpu` creating wgpu storage
  buffers from ArchonPages (dirty-aware caching).
- **Telos stubs** (`telos_stub.rs`): `TelosState` with transition history.
  `process_event` validates against `EidosProof`s then generates
  `StateMutation`. Handles Click/DoubleClick/LongPress/Focus/Blur/KeyDown/
  KeyUp/TouchBegin/TouchEnd/ValueChange.

### Phase 8: Security

- **Capability tokens** (`security/capability.rs`): `CapabilityToken` bitflags
  (TEXTURE_READ/WRITE, UNIFORM_READ, STORAGE_READ/WRITE, SAMPLER,
  RENDER_TARGET, MODAL). `CapabilityGuard` validates shader access.
- **Viewport isolation** (`security/viewport.rs`): `ViewportGuard` with
  bounds, scissor/stencil state. `apply_scissor` and `setup_stencil` for
  hardware enforcement.
- **Hierarchical z-depth** (`security/z_depth.rs`): `HierarchicalZDepth`
  computing z from parent+sibling index, enforcing max depth and modal
  capability for large gaps.

### Phase 9: Performance

- **Double/triple buffering** (`framebuffer.rs`): `FrameBufferSet` with N
  textures, `AtomicUsize` front/back indices, lock-free swap via
  `Ordering::Acquire`/`Release`.
- **Dirty rect tracking** (`inspector/dirty_rect.rs`): automatic merging of
  nearby rects (10px threshold), previous/current frame comparison.
- **Zero-alloc render loop**: pre-allocated buffers; per-frame allocations
  limited to transient graph resources.

### Phase 10: Developer Experience

- **Chora Inspector** (`inspector.rs`): semi-transparent overlay bar,
  per-frame lifecycle (begin/record/end), `InspectorConfig` toggles.
- **GPU timing** (`gpu_timing.rs`): timestamp query management, resolve to
  `Vec<TimingResult>`.
- **Overdraw heatmap** (`heatmap.rs`): 8x8 cell grid, `record_triangle`
  rasterizes bounding box, `to_rgba_texture_data` produces visualization.
- **Color proof** (`color_proof.rs`): 6 modes (None, Protanopia, Deuteranopia,
  Tritanopia, Achromatopsia, HighContrast) with 3x3 simulation matrices.
- **Hot reload** (`hot_reload.rs`): file watcher polling modification times,
  `ReloadEvent` for eidos/shader/asset changes.

### Phase 11: Three-Tier Fallback

- **Tier 1** — wgpu auto-selects software backend (lavapipe/LLVMpipe) when
  no hardware adapter is found.
- **Tier 2** — `HeadlessRenderer` wraps the core renderer, outputs PNG/JPEG/
  raw RGBA via `render_frame` or `render_frame_to_file`.
- **Tier 3** — `DynamicFidelity` with 5 profiles (Ultra/High/Medium/Low/
  Minimum) controlling shadows, post-processing, SDF fonts, FPS caps,
  shadow map sizes, texture limits, MSAA, volumetric lighting, foveated
  rendering. Adapts based on frame time with smoothing score.

### Phase 12: Migration & Adoption

- **Web component** (`web_component.rs`): `ComponentBridge` simulating a
  `<tpt-chora>` custom element with render/event handling.
- **CSS transpiler** (`css_parser.rs` + `eidos_transpiler.rs`): hand-written
  CSS tokenizer producing `ParsedCss`; `EidosTranspiler` converts to
  pseudo-Eidos IR with safety checks (overflow, z-index, position, width/
  height auto, font-size < 10px) and auto-corrections.
- **FFI bridge** (`ffi_bridge.rs`): `FfiBridge` with Wasm module registration
  (magic number check, section parsing for exports/memory) and function
  call dispatch.

### Phase 13: Deployment

- **WebGPU Bootstrap** (`deployment/bootstrap.rs`): generates a JS script
  that creates a canvas, requests WebGPU adapter/device, fetches and
  instantiates a Wasm module.

### Phase 14: v1.0

- **All-subsystems demo** (`all_subsystems.rs`): single `main()` exercising
  every subsystem from Phase 1 through Phase 13.

## Conventions

- `cargo fmt --all -- --check` and `cargo build --workspace` should stay
  clean.
- New crates: add to root `Cargo.toml` `[workspace.members]` and
  `[workspace.dependencies]`, named `tpt-chora-<name>` (matching the
  directory name `crates/tpt-chora-<name>`, per repo convention).
- Do not add comments to code unless the *why* is non-obvious.

## What's still directional

External crate integration: the real sibling `tpt-eidos`, `tpt-archon`,
and `tpt-telos` projects were investigated (2026-07-30) and found to
implement unrelated domains — `tpt-eidos` is a refinement-type verifier
for safety-critical numeric code (flight control, industrial controls,
medical dosing), `tpt-archon` is an embedded storage engine + microkernel
+ SQL query stack meant to replace Postgres/SQLite rather than proxy
them, and `tpt-telos` is a DSL verification/codegen compiler. None
implement UI layout proofs, zero-copy paged GPU-bindable state, or UI
event/state-machine processing. `tpt-chora-runtime`'s local
`telos_stub.rs`/`TelosBackend`, `archon_stub.rs`/`ArchonBackend`, and
`contracts.rs`'s `ChoraVisualNode`/`ChoraSemanticNode` are therefore the
permanent implementations of these roles, not placeholders awaiting a
swap.

Platform-specific hardware integrations now have real, feature-gated,
best-effort implementations behind opt-in Cargo features:

- `tpt-chora-a11y` `native-a11y-backends`: real Windows UI Automation
  (via the `windows` crate, buildable/testable on this environment),
  macOS NSAccessibility (via `objc2`/`objc2-app-kit`, unverified on
  this platform), and Android `AccessibilityNodeInfo` (via `jni`/`ndk`,
  unverified on this platform).
- `tpt-chora-input` `native-haptics-backends`: real CoreHaptics
  (macOS/iOS, via `objc2`, unverified on this platform), Android
  Vibrator (via `jni`, unverified on this platform), and XR rumble
  (documented `HapticNotSupported` stub — no XR session API
  is integrated in this codebase).
- `tpt-chora-media` `native-video-backends`: real Linux VA-API decode
  (via raw FFI to `libva`, CI-verified with `libva-dev`), macOS
  VideoToolbox (via `objc2`, unverified), and Android MediaCodec
  (via `jni` + NDK, unverified).

The zero-allocation render loop target requires profiling under sustained
load to identify and eliminate any remaining per-frame allocations.
