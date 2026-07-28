//! Frame pacing benchmark (Phase 18): drives `Renderer::render_frame` in a
//! loop, swapping a `FrameBufferSet` front/back index each frame the way a
//! real presentation loop would, and reports mean/p95/p99 frame times.
//!
//! This is a plain timed loop rather than a `criterion` harness: percentile
//! frame-time reporting is the point of the benchmark, and criterion's own
//! statistics (mean/median/slope) don't include p95/p99 in its console
//! output.

use std::time::{Duration, Instant};

use tpt_chora_render::{FrameBufferSet, GpuContext, Renderer};

const WARMUP_FRAMES: usize = 10;
const MEASURED_FRAMES: usize = 200;
const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

fn percentile(sorted_ns: &[u128], p: f64) -> Duration {
    let idx = ((sorted_ns.len() - 1) as f64 * p).round() as usize;
    Duration::from_nanos(sorted_ns[idx] as u64)
}

fn main() {
    let renderer = Renderer::new_headless(WIDTH, HEIGHT).expect("headless renderer");
    let ctx = GpuContext::new_headless().expect("headless GPU context");
    let frame_buffers = FrameBufferSet::new(&ctx.device, WIDTH, HEIGHT, 3);

    for _ in 0..WARMUP_FRAMES {
        renderer.render_frame().expect("render_frame (warmup)");
        frame_buffers.swap_next_triple();
    }

    let mut samples_ns = Vec::with_capacity(MEASURED_FRAMES);
    for _ in 0..MEASURED_FRAMES {
        let start = Instant::now();
        renderer.render_frame().expect("render_frame");
        frame_buffers.swap_next_triple();
        samples_ns.push(start.elapsed().as_nanos());
    }

    samples_ns.sort_unstable();
    let mean_ns: u128 = samples_ns.iter().sum::<u128>() / samples_ns.len() as u128;
    let mean = Duration::from_nanos(mean_ns as u64);
    let p50 = percentile(&samples_ns, 0.50);
    let p95 = percentile(&samples_ns, 0.95);
    let p99 = percentile(&samples_ns, 0.99);
    let min = Duration::from_nanos(samples_ns[0] as u64);
    let max = Duration::from_nanos(samples_ns[samples_ns.len() - 1] as u64);

    println!(
        "frame_pacing: {WIDTH}x{HEIGHT}, {MEASURED_FRAMES} frames (after {WARMUP_FRAMES} warmup)"
    );
    println!("  min:  {min:?}");
    println!("  mean: {mean:?}");
    println!("  p50:  {p50:?}");
    println!("  p95:  {p95:?}");
    println!("  p99:  {p99:?}");
    println!("  max:  {max:?}");
}
