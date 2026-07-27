use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub struct AssetStreamer {
    requests: BinaryHeap<PrioritizedRequest>,
    max_concurrent: usize,
    in_flight: usize,
    next_sequence: u64,
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
            next_sequence: 0,
        }
    }

    pub fn enqueue(&mut self, request: StreamRequest) {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        self.requests.push(PrioritizedRequest {
            request,
            sequence: seq,
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
        self.requests.extend(remaining);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(url: &str, priority: AssetPriority) -> StreamRequest {
        StreamRequest {
            url: url.to_string(),
            priority,
            expected_size: None,
            bounding_box: None,
        }
    }

    fn make_request_bbox(url: &str, priority: AssetPriority, bbox: [f32; 4]) -> StreamRequest {
        StreamRequest {
            url: url.to_string(),
            priority,
            expected_size: None,
            bounding_box: Some(bbox),
        }
    }

    #[test]
    fn new_creates_empty_streamer() {
        let s = AssetStreamer::new(4);
        assert_eq!(s.max_concurrent, 4);
        assert_eq!(s.pending_count(), 0);
        assert_eq!(s.in_flight_count(), 0);
        assert!(s.requests.is_empty());
    }

    #[test]
    fn enqueue_dequeue_priority_order() {
        let mut s = AssetStreamer::new(10);
        s.enqueue(make_request("low", AssetPriority::Low));
        s.enqueue(make_request("high", AssetPriority::High));
        s.enqueue(make_request("normal", AssetPriority::Normal));

        let first = s.dequeue_next().unwrap();
        assert_eq!(first.url, "low");

        let second = s.dequeue_next().unwrap();
        assert_eq!(second.url, "normal");

        let third = s.dequeue_next().unwrap();
        assert_eq!(third.url, "high");

        assert!(s.dequeue_next().is_none());
    }

    #[test]
    fn sequence_numbers_unique() {
        let mut s = AssetStreamer::new(10);
        for i in 0..5 {
            s.enqueue(make_request(&format!("req_{i}"), AssetPriority::Normal));
        }
        let mut urls = Vec::new();
        for _ in 0..5 {
            let r = s.dequeue_next().unwrap();
            urls.push(r.url);
        }
        urls.sort();
        let expected: Vec<String> = (0..5).map(|i| format!("req_{i}")).collect();
        assert_eq!(urls, expected);
    }

    #[test]
    fn max_concurrent_limits_dequeue() {
        let mut s = AssetStreamer::new(2);
        s.enqueue(make_request("a", AssetPriority::Normal));
        s.enqueue(make_request("b", AssetPriority::Normal));
        s.enqueue(make_request("c", AssetPriority::Normal));

        assert!(s.dequeue_next().is_some());
        assert!(s.dequeue_next().is_some());
        assert!(s.dequeue_next().is_none());
    }

    #[test]
    fn complete_allows_another_dequeue() {
        let mut s = AssetStreamer::new(2);
        s.enqueue(make_request("a", AssetPriority::Normal));
        s.enqueue(make_request("b", AssetPriority::Normal));
        s.enqueue(make_request("c", AssetPriority::Normal));

        assert!(s.dequeue_next().is_some());
        assert!(s.dequeue_next().is_some());
        assert!(s.dequeue_next().is_none());

        s.complete();
        assert_eq!(s.in_flight_count(), 1);
        assert!(s.dequeue_next().is_some());
    }

    #[test]
    fn prioritize_viewport_promotes_overlapping() {
        let mut s = AssetStreamer::new(10);
        s.enqueue(make_request_bbox(
            "item",
            AssetPriority::Normal,
            [10.0, 10.0, 20.0, 20.0],
        ));

        s.prioritize_viewport([0.0, 0.0, 15.0, 15.0]);

        let req = s.dequeue_next().unwrap();
        assert_eq!(req.priority, AssetPriority::High);
    }

    #[test]
    fn prioritize_viewport_preserves_non_overlapping() {
        let mut s = AssetStreamer::new(10);
        s.enqueue(make_request_bbox(
            "far",
            AssetPriority::Normal,
            [100.0, 100.0, 200.0, 200.0],
        ));

        s.prioritize_viewport([0.0, 0.0, 15.0, 15.0]);

        let req = s.dequeue_next().unwrap();
        assert_eq!(req.priority, AssetPriority::Normal);
    }

    #[test]
    fn prioritize_viewport_preserves_no_bounding_box() {
        let mut s = AssetStreamer::new(10);
        s.enqueue(make_request("nobbox", AssetPriority::Normal));

        s.prioritize_viewport([0.0, 0.0, 15.0, 15.0]);

        let req = s.dequeue_next().unwrap();
        assert_eq!(req.priority, AssetPriority::Normal);
    }

    #[test]
    fn enqueue_viewport_prefetch_creates_low_priority() {
        let mut s = AssetStreamer::new(10);
        s.enqueue_viewport_prefetch("prefetch".to_string(), [10.0, 20.0, 30.0, 40.0], 5.0);

        let req = s.dequeue_next().unwrap();
        assert_eq!(req.url, "prefetch");
        assert_eq!(req.priority, AssetPriority::Low);
        assert_eq!(req.bounding_box, Some([5.0, 15.0, 35.0, 45.0]));
    }
}
