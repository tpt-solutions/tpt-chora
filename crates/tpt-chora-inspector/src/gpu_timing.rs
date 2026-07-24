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
    start_query: Option<wgpu::QuerySet>,
    end_query: Option<wgpu::QuerySet>,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            queries: Vec::new(),
            results: Vec::new(),
            next_id: 0,
        }
    }

    pub fn begin_query(&mut self, device: &wgpu::Device, label: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.queries.push(TimingQuery {
            id,
            label: label.to_string(),
            start_query: None,
            end_query: None,
        });

        id
    }

    pub fn end_query(&mut self, device: &wgpu::Device, id: u32) {
        if let Some(query) = self.queries.iter_mut().find(|q| q.id == id) {
        }
    }

    pub fn readback(&mut self) -> Vec<TimingResult> {
        let results = self.results.clone();
        self.results.clear();
        results
    }

    pub fn resolve_queries(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        query_set: &wgpu::QuerySet,
    ) {
    }
}
