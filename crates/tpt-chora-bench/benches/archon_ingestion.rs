//! Zero-copy ingestion benchmark (Phase 18): measures
//! `ChoraRuntime::bind_state_to_gpu` throughput/latency for representative
//! `ArchonPage` payload sizes (`crates/tpt-chora-runtime/src/archon_stub.rs`).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tpt_chora_render::GpuContext;
use tpt_chora_runtime::archon_stub::PageLayout;
use tpt_chora_runtime::{ArchonState, ChoraRuntime};

fn bench_bind_state_to_gpu(c: &mut Criterion) {
    let ctx = GpuContext::new_headless().expect("headless GPU context (Tier 1 fallback adapter)");

    let mut group = c.benchmark_group("archon_bind_state_to_gpu");

    for &size in &[4 * 1024usize, 64 * 1024, 1024 * 1024] {
        let mut archon = ArchonState::new();
        let layout = PageLayout {
            stride: size,
            field_offsets: vec![("data".to_string(), 0)],
        };
        let page_id = archon.create_page(vec![0u8; size], layout);
        let mut runtime = ChoraRuntime::new();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                // The page's `dirty` flag is never cleared here, so every
                // iteration exercises the "fresh data this frame" path
                // rather than the already-bound cache hit. `write_buffer`
                // only stages its copy into `Device::pending_writes`; that
                // staging allocation is freed only once an actual
                // `queue.submit` processes it, so submit and poll every
                // iteration to keep staging memory bounded.
                let page = archon.get_page(page_id).expect("page exists");
                let buffer = runtime.bind_state_to_gpu(&ctx.device, &ctx.queue, page);
                ctx.queue.submit(std::iter::empty());
                ctx.device.poll(wgpu::Maintain::Wait);
                buffer
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_bind_state_to_gpu);
criterion_main!(benches);
