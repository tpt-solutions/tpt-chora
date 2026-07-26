use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub struct AssetStreamer {
    requests: BinaryHeap<PrioritizedRequest>,
    max_concurrent: usize,
    in_flight: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

impl AssetPriority {
    pub fn promoted(self) -> Self {
        match self {
            AssetPriority::Background => AssetPriority::Low,
            AssetPriority::Low => AssetPriority::Normal,
            AssetPriority::Normal => AssetPriority::High,
            AssetPriority::High => AssetPriority::Critical,
            AssetPriority::Critical => AssetPriority::Critical,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamRequest {
    pub url: String,
    pub priority: AssetPriority,
    pub expected_size: Option<usize>,
    pub bounding_box: Option<[f32; 4]>,
}

struct PrioritizedRequest {
    request: StreamRequest,
    sequence: u64,
}

impl PartialEq for PrioritizedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.request.priority == other.request.priority && self.sequence == other.sequence
    }
}

impl Eq for PrioritizedRequest {}

impl PartialOrd for PrioritizedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.request
            .priority
            .cmp(&other.request.priority)
            .then_with(|| self.sequence.cmp(&other.sequence).reverse())
    }
}

impl AssetStreamer {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            requests: BinaryHeap::new(),
            max_concurrent,
            in_flight: 0,
        }
    }

    pub fn enqueue(&mut self, request: StreamRequest) {
        self.requests.push(PrioritizedRequest {
            request,
            sequence: self.requests.len() as u64,
        });
    }

    pub fn enqueue_viewport_prefetch(
        &mut self,
        url: String,
        viewport_bounds: [f32; 4],
        prefetch_margin: f32,
    ) {
        let expanded_bounds = [
            viewport_bounds[0] - prefetch_margin,
            viewport_bounds[1] - prefetch_margin,
            viewport_bounds[2] + prefetch_margin,
            viewport_bounds[3] + prefetch_margin,
        ];

        self.enqueue(StreamRequest {
            url,
            priority: AssetPriority::Low,
            expected_size: None,
            bounding_box: Some(expanded_bounds),
        });
    }

    pub fn dequeue_next(&mut self) -> Option<StreamRequest> {
        if self.in_flight >= self.max_concurrent {
            return None;
        }
        let req = self.requests.pop().map(|p| p.request);
        if req.is_some() {
            self.in_flight += 1;
        }
        req
    }

    pub fn complete(&mut self) {
        if self.in_flight > 0 {
            self.in_flight -= 1;
        }
    }

    pub fn pending_count(&self) -> usize {
        self.requests.len()
    }

    pub fn in_flight_count(&self) -> usize {
        self.in_flight
    }

    pub fn prioritize_viewport(&mut self, viewport_bounds: [f32; 4]) {
        let mut promoted = Vec::new();
        let mut remaining = BinaryHeap::new();

        while let Some(req) = self.requests.pop() {
            if let Some(bbox) = &req.request.bounding_box {
                let overlaps = viewport_bounds[0] < bbox[2]
                    && viewport_bounds[2] > bbox[0]
                    && viewport_bounds[1] < bbox[3]
                    && viewport_bounds[3] > bbox[1];

                if overlaps && req.request.priority as u32 > AssetPriority::Critical as u32 {
                    let mut new_req = req.request;
                    new_req.priority = new_req.priority.promoted();
                    promoted.push(PrioritizedRequest {
                        request: new_req,
                        sequence: req.sequence,
                    });
                } else {
                    remaining.push(req);
                }
            } else {
                remaining.push(req);
            }
        }

        for req in promoted {
            self.requests.push(req);
        }
        self.requests = remaining;
    }
}
