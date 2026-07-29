#![forbid(unsafe_code)]

//! Runtime benchmarks for tpt-chora (Phase 18): tessellation throughput,
//! frame pacing, steady-state heap-growth, and zero-copy ingestion. This
//! crate has no library surface of its own; see `benches/` and `tests/`.
