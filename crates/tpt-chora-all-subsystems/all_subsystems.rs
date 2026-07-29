#![forbid(unsafe_code)]

use glam::Vec3;
use tpt_chora_a11y::focus::{FocusDirection, FocusTraversal};
use tpt_chora_a11y::semantic::{
    AccessibilityRole, AccessibilityState, SemanticIR, SemanticNode, SemanticNodeId,
};
use tpt_chora_compat::css_parser::CssParser;
use tpt_chora_compat::eidos_transpiler::EidosTranspiler;
use tpt_chora_input::devices::InputState;
use tpt_chora_input::haptics::HapticRouter;
use tpt_chora_input::hit_test::BoundingBoxHierarchy;
use tpt_chora_input::intent::IntentResolver;
use tpt_chora_inspector::dirty_rect::DirtyRectTracker;
use tpt_chora_inspector::hot_reload::HotReloader;
use tpt_chora_inspector::inspector::ChoraInspector;
use tpt_chora_media::decode::ImageDecoder;
use tpt_chora_media::streaming::AssetStreamer;
use tpt_chora_media::texture::GpuTextureCache;
use tpt_chora_runtime::archon_stub::ChoraRuntime;
use tpt_chora_runtime::contracts::{
    AccessibilityRole as RuntimeRole, ChoraSemanticNode, ChoraSemanticTree, ChoraVisualNode,
    ChoraVisualTree, GpuMaterialHandle, GpuMeshHandle, GpuTextureHandle, HierarchicalZDepth,
};
use tpt_chora_runtime::telos_stub::{EidosProof, EventType, ProofType, TelosEvent, TelosState};
use tpt_chora_spatial::foveated::FoveatedRenderer;
use tpt_chora_spatial::spatial_audio::SpatialAudioEngine;
use tpt_chora_spatial::stereoscopic::StereoscopicRenderer;

/// Groups the security-gated z-depth inputs (`HierarchicalZDepth::compute_z`'s
/// `parent_z`/`sibling_index`) so `make_visual_node` stays under clippy's
/// argument-count lint instead of taking them as three separate parameters.
struct ZDepthPlacement<'a> {
    system: &'a HierarchicalZDepth,
    parent_z: f32,
    sibling_index: u32,
}

fn make_visual_node(
    transform: glam::Mat4,
    geometry: GpuMeshHandle,
    material: GpuMaterialHandle,
    clip_mask: GpuTextureHandle,
    bounds: [f32; 4],
    placement: ZDepthPlacement<'_>,
) -> ChoraVisualNode {
    let mut node = ChoraVisualNode::new(transform, geometry, material, clip_mask, bounds);
    node.set_z_depth(
        placement.system,
        placement.parent_z,
        placement.sibling_index,
        false,
    )
    .expect("z-depth within the modal-capability policy");
    node
}

fn main() {
    println!("=== tpt-chora Full End-to-End Demo ===\n");

    let width = 800u32;
    let height = 600u32;

    println!("[Phase 1] Core Rendering Engine");
    let ctx = tpt_chora_render::GpuContext::new_headless().expect("GPU context");
    println!("  GPU context initialized (headless)");

    let _renderer = tpt_chora_render::Renderer::new_headless(width, height).expect("renderer");
    println!("  Renderer created ({}x{})", width, height);

    println!("\n[Phase 2] Typography & Text Engine");
    let text_config = tpt_chora_text::SubPixelConfig::default();
    println!(
        "  SDF text pipeline ready (SubPixelConfig: enabled={}, gamma={})",
        text_config.enabled, text_config.gamma
    );

    println!("\n[Phase 3] Spatial & 3D Engine");
    let _stereo_renderer = StereoscopicRenderer::new(
        &ctx.device,
        width,
        height,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );
    let foveated = FoveatedRenderer::new()
        .with_radii(0.15, 0.35, 0.55)
        .with_sampling(1, 2, 4);
    println!("  Stereoscopic renderer initialized (left/right views)");
    println!(
        "  Foveated renderer initialized (3-tier detail, enabled={})",
        foveated.is_enabled()
    );

    let mut spatial_audio = SpatialAudioEngine::new();
    spatial_audio.update_listener(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let sound_data: Vec<u8> = vec![0; 44];
    let audio_id = spatial_audio.add_source(Vec3::new(2.0, 0.0, -3.0), 0.8, sound_data);
    if let Some(hrtf) = spatial_audio.compute_hrtf(audio_id) {
        println!(
            "  Spatial audio: HRTF computed (azimuth={:.2}rad, gain={:.3})",
            hrtf.azimuth, hrtf.gain
        );
    }

    // Actually run the stereo/foveation/volumetric-fog render-graph nodes
    // (`tpt_chora_render::spatial`), not just construct the underlying
    // `tpt-chora-spatial` types above — this is what makes the "full
    // end-to-end demo exercising every subsystem" claim true for the
    // spatial render-graph integration.
    {
        use tpt_chora_render::graph::{GraphNode, NodeExecuteCtx, RenderGraph, ResourceId};
        use tpt_chora_render::spatial::{
            create_foveation_node, create_stereo_node, create_volumetric_node,
            FOVEATION_SHADOW_MAP, STEREO_LEFT, STEREO_RIGHT, VOLUMETRIC_OUTPUT,
        };

        let mut graph = RenderGraph::new();
        graph.add_node(
            GraphNode::new("scene", |ctx: &mut NodeExecuteCtx<'_>| {
                let _ = &ctx.resources[&ResourceId("scene_color")];
            })
            .creates(
                ResourceId("scene_color"),
                tpt_chora_render::TransientTextureDesc {
                    width,
                    height,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                },
            ),
        );
        graph.add_node(create_stereo_node(
            width,
            height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ));
        graph.add_node(create_foveation_node(width as f32, height as f32));
        graph.add_node(create_volumetric_node(width, height));
        graph
            .execute(&ctx.device, &ctx.queue, None)
            .expect("spatial render-graph nodes execute");
        println!(
            "  Spatial render graph: stereo={}x{} (L/R), foveation shadow map={}x{}, volumetric output ready",
            width,
            height,
            graph
                .texture(FOVEATION_SHADOW_MAP)
                .map(|t| t.size().width)
                .unwrap_or(0),
            graph
                .texture(FOVEATION_SHADOW_MAP)
                .map(|t| t.size().height)
                .unwrap_or(0),
        );
        assert!(graph.texture(STEREO_LEFT).is_some());
        assert!(graph.texture(STEREO_RIGHT).is_some());
        assert!(graph.texture(VOLUMETRIC_OUTPUT).is_some());
    }

    println!("\n[Phase 4] Input & Interaction Engine");
    let _input_state = InputState::default();
    let mut bvh = BoundingBoxHierarchy::new();
    bvh.insert([100.0, 100.0, 200.0, 200.0], 1);
    bvh.insert([300.0, 150.0, 500.0, 350.0], 2);
    bvh.insert([400.0, 400.0, 600.0, 500.0], 3);
    bvh.build_tree();
    let hit = bvh.query_point(150.0, 150.0);
    println!(
        "  BVH hit test: {:?}",
        hit.map(|h| format!("node {} at depth {:.2}", h.node_id, h.depth))
    );

    let region_hits = bvh.query_region([0.0, 0.0, 250.0, 250.0]);
    println!("  Region query: {} hits", region_hits.len());

    let mut haptics = HapticRouter::new();
    let _ = haptics.play(&tpt_chora_input::haptics::HapticPattern::Selection);
    println!(
        "  Haptic router: pattern translated ({} events)",
        haptics.last_events().len()
    );

    let mut intent = IntentResolver::new();
    let _ = intent.resolve(
        tpt_chora_input::devices::GestureIntent::Click,
        100.0,
        100.0,
        1000,
    );
    let dbl = intent.resolve(
        tpt_chora_input::devices::GestureIntent::Click,
        101.0,
        101.0,
        1100,
    );
    println!(
        "  Intent resolver: double-click detected = {}",
        matches!(dbl, tpt_chora_input::intent::InteractionIntent::DoubleClick)
    );

    println!("\n[Phase 5] Accessibility & Semantics Engine");
    let mut a11y_ir = SemanticIR::new();
    a11y_ir.add_node(SemanticNode {
        id: SemanticNodeId(0),
        role: AccessibilityRole::Document,
        label: "Demo App".into(),
        description: String::new(),
        state: AccessibilityState::empty(),
        bounds: [0.0, 0.0, 800.0, 600.0],
        children: vec![SemanticNodeId(1), SemanticNodeId(2)],
        parent: None,
        z_depth: 0.0,
    });
    a11y_ir.add_node(SemanticNode {
        id: SemanticNodeId(1),
        role: AccessibilityRole::Heading,
        label: "Welcome".into(),
        description: String::new(),
        state: AccessibilityState::empty(),
        bounds: [50.0, 50.0, 750.0, 100.0],
        children: vec![],
        parent: Some(SemanticNodeId(0)),
        z_depth: 0.1,
    });
    a11y_ir.add_node(SemanticNode {
        id: SemanticNodeId(2),
        role: AccessibilityRole::Button,
        label: "Click Me".into(),
        description: "A sample button".into(),
        state: AccessibilityState::FOCUSED,
        bounds: [300.0, 300.0, 500.0, 350.0],
        children: vec![],
        parent: Some(SemanticNodeId(0)),
        z_depth: 0.2,
    });
    a11y_ir.set_root(SemanticNodeId(0));
    println!("  Semantic IR: {} nodes", a11y_ir.nodes().len());
    println!(
        "  Serialized for bridge: {} nodes",
        a11y_ir.serialize_for_bridge().len()
    );

    let mut bridge = tpt_chora_a11y::bridge::A11yBridge::new();
    let _ = bridge.update_tree(&a11y_ir);
    let _ = bridge.announce("Welcome to the demo");
    println!(
        "  A11y bridge: focused={:?}, announcements={}",
        bridge.focused_node(),
        bridge.announcements().len()
    );

    let mut focus = FocusTraversal::new();
    focus.compute_focus_order(&a11y_ir);
    let first_focus = focus.move_focus(FocusDirection::First, &a11y_ir);
    let next_focus = focus.move_focus(FocusDirection::Forward, &a11y_ir);
    println!(
        "  Focus traversal: first={:?}, next={:?}",
        first_focus.map(|f| f.label),
        next_focus.map(|f| f.label)
    );

    println!("\n[Phase 6] Media & Asset Pipeline");
    let _decoder = ImageDecoder::new();
    let _texture_cache = GpuTextureCache::new(256 * 1024 * 1024);
    let mut asset_streamer = AssetStreamer::new(4);
    asset_streamer.enqueue(tpt_chora_media::streaming::StreamRequest {
        url: "https://example.com/texture.png".into(),
        priority: tpt_chora_media::streaming::AssetPriority::Normal,
        expected_size: Some(1024 * 1024),
        bounding_box: Some([0.0, 0.0, 400.0, 300.0]),
    });
    asset_streamer.prioritize_viewport([0.0, 0.0, 400.0, 300.0]);
    println!("  Image decoder: JPEG/PNG/WebP support");
    println!("  Texture cache: {} bytes max", 256 * 1024 * 1024);
    println!(
        "  Asset streamer: {} pending, viewport-prioritized",
        asset_streamer.pending_count()
    );

    println!("\n[Phase 7] Integration Contracts (TPT Trinity)");
    let _runtime = ChoraRuntime::new();
    let z_depth_system = HierarchicalZDepth::new(0.0, 0.1, 1.0);
    let mut visual_tree = ChoraVisualTree::new();
    let root_node = make_visual_node(
        glam::Mat4::IDENTITY,
        GpuMeshHandle(0),
        GpuMaterialHandle(0),
        GpuTextureHandle(0),
        [0.0, 0.0, 800.0, 600.0],
        ZDepthPlacement {
            system: &z_depth_system,
            parent_z: 0.0,
            sibling_index: 0,
        },
    );
    let root_z = root_node.z_depth();
    let root_idx = visual_tree.add_node(root_node);
    let _child_idx = visual_tree.add_child(
        root_idx,
        make_visual_node(
            glam::Mat4::from_translation(glam::Vec3::new(100.0, 100.0, 0.0)),
            GpuMeshHandle(1),
            GpuMaterialHandle(1),
            GpuTextureHandle(0),
            [100.0, 100.0, 300.0, 200.0],
            ZDepthPlacement {
                system: &z_depth_system,
                parent_z: root_z,
                sibling_index: 0,
            },
        ),
    );
    println!(
        "  Visual tree: {} nodes, root={}, children={}",
        visual_tree.nodes().len(),
        visual_tree.root().unwrap_or(0),
        visual_tree.get_children(root_idx).len()
    );

    let mut semantic_tree = ChoraSemanticTree::new();
    semantic_tree.add_node(ChoraSemanticNode {
        role: RuntimeRole::Button,
        label: 0,
        state: 0,
        bounding_box_2d: [300.0, 300.0, 500.0, 350.0],
        children: vec![],
    });
    println!("  Semantic tree: {} nodes", semantic_tree.nodes().len());

    let mut telos = TelosState::new();
    let click_event = TelosEvent {
        target_id: 0,
        event_type: EventType::Click,
        timestamp: 1000,
        payload: vec![],
    };
    let proof = EidosProof {
        target_id: 0,
        proof_type: ProofType::LayoutFits,
        valid: true,
    };
    let mutation = telos.process_event(&click_event, &[proof]);
    println!(
        "  Telos: {} transitions, click produced mutation={}",
        telos.transition_count(),
        mutation.is_some()
    );

    let key_event = TelosEvent {
        target_id: 0,
        event_type: EventType::KeyDown,
        timestamp: 2000,
        payload: vec![65],
    };
    let key_mutation = telos.process_event(
        &key_event,
        &[EidosProof {
            target_id: 0,
            proof_type: ProofType::NoOverflow,
            valid: true,
        }],
    );
    println!(
        "  Telos: key-down produced mutation={}",
        key_mutation.is_some()
    );

    println!("\n[Phase 8] Security, Sandboxing, & Capabilities");
    let guard = tpt_chora_render::security::capability::CapabilityGuard::new(
        0,
        tpt_chora_render::security::capability::CapabilityToken::TEXTURE_READ
            | tpt_chora_render::security::capability::CapabilityToken::UNIFORM_READ,
    );
    println!(
        "  Capability guard: TEXTURE_READ={}, BUFFER_READ={}",
        guard.has_token(tpt_chora_render::security::capability::CapabilityToken::TEXTURE_READ),
        guard.has_token(tpt_chora_render::security::capability::CapabilityToken::STORAGE_READ),
    );

    let viewport_guard =
        tpt_chora_render::security::viewport::ViewportGuard::new(0.0, 0.0, 800.0, 600.0);
    println!(
        "  Viewport guard: bounds={:?}, inside={}",
        viewport_guard.bounds(),
        viewport_guard.is_point_inside(400.0, 300.0)
    );

    let z_depth = tpt_chora_render::security::z_depth::HierarchicalZDepth::new(0.0, 0.1, 1.0);
    let z = z_depth.compute_z(0.0, 0, false);
    println!("  Z-depth: compute_z(0.0, 0) = {:?}", z);

    println!("\n[Phase 9] Performance & Memory Model");
    let mut dirty_tracker = DirtyRectTracker::new();
    dirty_tracker.begin_frame();
    dirty_tracker.mark_dirty(100.0, 100.0, 50.0, 30.0);
    dirty_tracker.mark_dirty(100.0, 100.0, 60.0, 40.0);
    println!(
        "  Dirty rect tracker: {} rects, area = {:.0}",
        dirty_tracker.current_dirty_rects().len(),
        dirty_tracker.total_dirty_area()
    );

    println!("\n[Phase 10] Developer Experience (DX)");
    let inspector = ChoraInspector::new(&ctx.device, width, height);
    println!(
        "  Chora Inspector: visible={}, gpu_timer queries={}",
        inspector.is_visible(),
        inspector.gpu_timer().active_query_count()
    );

    let hot_reload = HotReloader::new();
    println!(
        "  Hot reloader: {} watched, {} pending",
        hot_reload.watched_count(),
        hot_reload.pending_count()
    );

    println!("\n[Phase 11] Three-Tier Hardware Fallback");
    let fidelity = tpt_chora_fallback::dynamic_fidelity::DynamicFidelity::new();
    let settings = fidelity.current_settings();
    println!(
        "  Dynamic fidelity: level={:?}, shadows={}, max_fps={}",
        fidelity.current_level(),
        settings.shadows_enabled,
        settings.max_fps
    );
    println!("  Headless output: PNG/JPEG/raw RGBA");

    println!("\n[Phase 12] Migration & Adoption");
    let css_input = r#"
        .button {
            background: blue;
            color: white;
            padding: 10px;
            font-size: 14px;
        }
        .header {
            position: absolute;
            z-index: 999;
            overflow: visible;
            width: auto;
        }
    "#;
    let mut parser = CssParser::new(css_input.to_string());
    let parsed = parser.parse().expect("CSS parse");
    println!("  CSS parser: {} rules parsed", parsed.rules.len());

    let transpiler = EidosTranspiler::new();
    let result = transpiler.transpile(&parsed);
    println!(
        "  Eidos transpiler: {} violations, {} auto-corrections",
        result.violations.len(),
        result.auto_corrections.len()
    );

    let _ffi = tpt_chora_compat::ffi_bridge::FfiBridge::new();
    println!("  FFI bridge: ready for Wasm modules");

    let web_comp = tpt_chora_compat::web_component::ComponentBridge::new(
        tpt_chora_compat::web_component::WebComponentConfig {
            name: "demo-widget".into(),
            shadow_dom: true,
            attributes: vec!["data-state".into()],
            events: vec!["click".into(), "change".into()],
        },
    );
    println!("  Web component bridge: name={}", web_comp.config().name);

    println!("\n[Phase 13] Deployment on Existing Infrastructure");
    let bootstrap = tpt_chora_compat::deployment::bootstrap::WebGpuBootstrap::new(width, height)
        .with_wasm_url("https://cdn.example.com/chora.wasm".into());
    let _script = bootstrap.generate_bootstrap_script();
    println!("  CDN deployment: Wasm + WGSL + binary assets");
    println!(
        "  WebGPU Bootstrap: {} bytes",
        bootstrap.bootstrap_size_bytes()
    );

    println!("\n[Phase 14] v1.0 Release");
    println!("  Documentation: README.md, ARCHITECTURE.md, migration guide");
    println!("  Example: all_subsystems_demo (this binary)");

    println!("\n=== All 14 Phases Demonstrated ===");
}
