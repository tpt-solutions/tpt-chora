//! Zero-allocation steady-state proof (Phase 18): wraps the steady-state
//! render loop with `dhat`'s counting heap profiler (behind the
//! `dhat-heap` feature) and asserts the live block count doesn't grow
//! across a further batch of frames once the loop has warmed up. This
//! catches per-frame *leaks* (unbounded growth) automatically; it is not a
//! claim that individual frames allocate nothing — `Renderer::render_frame`
//! still builds per-frame CPU-side vertex/index `Vec`s and wgpu buffers
//! that are freed once the frame's command buffer completes, which is
//! exactly the pattern this test is designed to let through.
//!
//! Run with: `cargo test -p tpt-chora-bench --features dhat-heap --test zero_alloc`

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// Measured empirically: wgpu's internal resource pools (pipeline/bind-group
// caches, etc.) allocate a handful of long-lived blocks over the first
// ~35 frames as various lazily-initialized caches get touched for the
// first time, then flatten out completely. 40 warmup frames comfortably
// clears that ramp-up; the tolerance absorbs the +/-1 block jitter
// dhat's own bookkeping introduces between arbitrary sample points.
#[cfg(feature = "dhat-heap")]
const WARMUP_FRAMES: usize = 40;
#[cfg(feature = "dhat-heap")]
const STEADY_STATE_FRAMES: usize = 40;
#[cfg(feature = "dhat-heap")]
const GROWTH_TOLERANCE_BLOCKS: usize = 2;

#[cfg(feature = "dhat-heap")]
#[test]
fn steady_state_loop_has_no_heap_growth() {
    let _profiler = dhat::Profiler::builder().build();
    let renderer = tpt_chora_render::Renderer::new_headless(64, 64).expect("headless renderer");

    for _ in 0..WARMUP_FRAMES {
        renderer.render_frame().expect("render_frame (warmup)");
    }
    let after_warmup = dhat::HeapStats::get().curr_blocks;

    for _ in 0..STEADY_STATE_FRAMES {
        renderer
            .render_frame()
            .expect("render_frame (steady state)");
    }
    let after_steady_state = dhat::HeapStats::get().curr_blocks;

    let growth = after_steady_state.saturating_sub(after_warmup);
    assert!(
        growth <= GROWTH_TOLERANCE_BLOCKS,
        "live heap block count grew from {after_warmup} to {after_steady_state} \
         ({growth} blocks) across {STEADY_STATE_FRAMES} steady-state frames -- \
         this indicates a per-frame leak"
    );
}

#[cfg(not(feature = "dhat-heap"))]
#[test]
fn steady_state_loop_has_no_heap_growth() {
    eprintln!("skipped: rerun with `--features dhat-heap` to enable the dhat heap-growth check");
}
