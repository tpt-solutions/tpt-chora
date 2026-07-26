pub struct GpuTimer {
    queries: Vec<TimingQuery>,
    results: Vec<TimingResult>,
    next_id: u32,
}

#[derive(Debug, Clone)]
pub struct TimingResult {
    pub label: String,
    pub elapsed_ns: f64,
}

struct TimingQuery {
    id: u32,
    label: String,
    start_query_idx: Option<u32>,
    end_query_idx: Option<u32>,
    start_recorded: bool,
    end_recorded: bool,
}

impl GpuTimer {
    pub fn new(_device: &wgpu::Device) -> Self {
        Self {
            queries: Vec::new(),
            results: Vec::new(),
            next_id: 0,
        }
    }

    pub fn begin_query(&mut self, _device: &wgpu::Device, label: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let query_idx = self.next_id - 1;

        self.queries.push(TimingQuery {
            id,
            label: label.to_string(),
            start_query_idx: Some(query_idx),
            end_query_idx: None,
            start_recorded: false,
            end_recorded: false,
        });

        id
    }

    pub fn end_query(&mut self, _device: &wgpu::Device, id: u32) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
            if query.end_query_idx.is_none() {
                let next_idx = self.next_id;
                self.next_id += 1;
                query.end_query_idx = Some(next_idx);
            }
            query.end_recorded = true;
        }
    }

    pub fn record_begin_timestamp(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        query_set: &wgpu::QuerySet,
        id: u32,
    ) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
            if let Some(idx) = query.start_query_idx {
                pass.write_timestamp(query_set, idx);
                query.start_recorded = true;
            }
        }
    }

    pub fn record_end_timestamp(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        query_set: &wgpu::QuerySet,
        id: u32,
    ) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
            if let Some(idx) = query.end_query_idx {
                pass.write_timestamp(query_set, idx);
                query.end_recorded = true;
            }
        }
    }

    pub fn resolve_queries(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        query_set: &wgpu::QuerySet,
        timestamp_period: f64,
        destination: &wgpu::Buffer,
    ) {
        let count = self.next_id;
        if count == 0 {
            return;
        }

        encoder.resolve_query_set(query_set, 0..count, destination, 0);

        for query in &self.queries {
            if query.start_recorded && query.end_recorded {
                if let (Some(start_idx), Some(end_idx)) =
                    (query.start_query_idx, query.end_query_idx)
                {
                    let diff = (end_idx as f64 - start_idx as f64).max(1.0);
                    let elapsed_ns = diff * timestamp_period;
                    self.results.push(TimingResult {
                        label: query.label.clone(),
                        elapsed_ns,
                    });
                }
            }
        }
    }

    pub fn readback(&mut self) -> Vec<TimingResult> {
        let results = self.results.clone();
        self.results.clear();
        results
    }

    pub fn active_query_count(&self) -> usize {
        self.queries.len()
    }
}
