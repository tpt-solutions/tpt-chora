use std::sync::atomic::{AtomicBool, Ordering};

pub struct GpuTimer {
    queries: Vec<TimingQuery>,
    results: Vec<TimingResult>,
    next_id: u32,
    query_set: wgpu::QuerySet,
    destination: wgpu::Buffer,
    timestamp_period: f64,
    max_queries: u32,
}

#[derive(Debug, Clone)]
pub struct TimingResult {
    pub label: String,
    pub elapsed_ns: f64,
}

struct TimingQuery {
    id: u32,
    label: String,
    start_query_idx: u32,
    end_query_idx: u32,
    start_recorded: bool,
    end_recorded: bool,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device, max_queries: u32) -> Self {
        let slot_count = max_queries * 2;
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("chora-gpu-timer-queries"),
            count: slot_count,
            ty: wgpu::QueryType::Timestamp,
        });

        let destination = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chora-gpu-timer-readback"),
            size: (slot_count as u64 * 8),
            usage: wgpu::BufferUsages::QUERY_RESOLVE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            queries: Vec::new(),
            results: Vec::new(),
            next_id: 0,
            query_set,
            destination,
            timestamp_period: 0.0,
            max_queries,
        }
    }

    pub fn begin_query(&mut self, _device: &wgpu::Device, label: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let start_idx = id * 2;
        let end_idx = start_idx + 1;

        self.queries.push(TimingQuery {
            id,
            label: label.to_string(),
            start_query_idx: start_idx,
            end_query_idx: end_idx,
            start_recorded: false,
            end_recorded: false,
        });

        id
    }

    pub fn end_query(&mut self, _device: &wgpu::Device, id: u32) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
            query.end_recorded = true;
        }
    }

    pub fn record_begin_timestamp(&mut self, pass: &mut wgpu::RenderPass<'_>, id: u32) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
            pass.write_timestamp(&self.query_set, query.start_query_idx);
            query.start_recorded = true;
        }
    }

    pub fn record_end_timestamp(&mut self, pass: &mut wgpu::RenderPass<'_>, id: u32) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
            pass.write_timestamp(&self.query_set, query.end_query_idx);
            query.end_recorded = true;
        }
    }

    pub fn resolve_queries(&mut self, encoder: &mut wgpu::CommandEncoder, timestamp_period: f64) {
        self.timestamp_period = timestamp_period;

        let count = self.next_id * 2;
        if count == 0 || count > self.max_queries * 2 {
            return;
        }

        encoder.resolve_query_set(&self.query_set, 0..count, &self.destination, 0);
    }

    pub fn readback(&mut self, device: &wgpu::Device) -> Vec<TimingResult> {
        if self.timestamp_period == 0.0 {
            return Vec::new();
        }

        let buffer_slice = self.destination.slice(..);

        let mapped = std::sync::Arc::new(AtomicBool::new(false));
        let mapped_clone = mapped.clone();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if result.is_ok() {
                mapped_clone.store(true, Ordering::Release);
            }
        });

        loop {
            device.poll(wgpu::Maintain::Poll);
            if mapped.load(Ordering::Acquire) {
                break;
            }
            std::thread::yield_now();
        }

        let mapped_data = buffer_slice.get_mapped_range();
        let timestamps: Vec<u64> = mapped_data
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        for query in &self.queries {
            if query.start_recorded && query.end_recorded {
                let start_idx = query.start_query_idx as usize;
                let end_idx = query.end_query_idx as usize;
                if start_idx < timestamps.len() && end_idx < timestamps.len() {
                    let start = timestamps[start_idx];
                    let end = timestamps[end_idx];
                    let elapsed_ns = (end.saturating_sub(start)) as f64 * self.timestamp_period;
                    self.results.push(TimingResult {
                        label: query.label.clone(),
                        elapsed_ns,
                    });
                }
            }
        }

        self.destination.unmap();

        let results = self.results.clone();
        self.results.clear();
        results
    }

    pub fn active_query_count(&self) -> usize {
        self.queries.len()
    }
}
