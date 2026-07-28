//! Bezier tessellation throughput benchmark (Phase 18): tessellates
//! 10k/50k/100k cubic Beziers via the existing GPU compute-shader path
//! (`tpt_chora_render::tessellate_cubics_gpu`) and reports curves/sec.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tpt_chora_render::{circle_path, tessellate_cubics_gpu, CubicBezier, GpuContext};

const SEGMENTS_PER_CURVE: u32 = 32;

fn build_curves(count: usize) -> Vec<CubicBezier> {
    let unit_circle = circle_path([0.0, 0.0], 1.0);
    unit_circle.into_iter().cycle().take(count).collect()
}

fn bench_tessellation(c: &mut Criterion) {
    let ctx = GpuContext::new_headless().expect("headless GPU context (Tier 1 fallback adapter)");

    let mut group = c.benchmark_group("bezier_tessellation_gpu");
    group.sample_size(20);

    for &curve_count in &[10_000usize, 50_000, 100_000] {
        let curves = build_curves(curve_count);
        group.throughput(Throughput::Elements(curve_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(curve_count),
            &curves,
            |b, curves| {
                b.iter(|| {
                    tessellate_cubics_gpu(&ctx.device, &ctx.queue, curves, SEGMENTS_PER_CURVE)
                        .expect("tessellate_cubics_gpu")
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_tessellation);
criterion_main!(benches);
