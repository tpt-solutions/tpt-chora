use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub trait ArchonBackend {
    fn get_page(&self, id: u64) -> Option<&ArchonPage>;
    fn get_page_mut(&mut self, id: u64) -> Option<&mut ArchonPage>;
    fn apply_mutation(&mut self, mutation: &crate::telos_stub::StateMutation);
    fn dirty_page_ids(&self) -> Vec<u64>;
    fn page_count(&self) -> usize;
}

pub struct ArchonPage {
    pub id: u64,
    data: Vec<u8>,
    layout: PageLayout,
    dirty: bool,
}

impl ArchonPage {
    pub fn layout(&self) -> &PageLayout {
        &self.layout
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

#[derive(Debug, Clone)]
pub struct PageLayout {
    pub stride: usize,
    pub field_offsets: Vec<(String, usize)>,
}

impl PageLayout {
    pub fn field_offset(&self, name: &str) -> Option<usize> {
        self.field_offsets
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, o)| *o)
    }

    pub fn total_size(&self) -> usize {
        self.field_offsets
            .iter()
            .map(|(_, offset)| offset)
            .max()
            .copied()
            .unwrap_or(0)
            + self.stride
    }
}

pub struct ArchonState {
    pages: Vec<ArchonPage>,
}

impl ArchonState {
    pub fn new() -> Self {
        Self { pages: Vec::new() }
    }

    pub fn create_page(&mut self, data: Vec<u8>, layout: PageLayout) -> u64 {
        let id = self.pages.len() as u64;
        self.pages.push(ArchonPage {
            id,
            data,
            layout,
            dirty: true,
        });
        id
    }

    pub fn get_page(&self, id: u64) -> Option<&ArchonPage> {
        self.pages.get(id as usize)
    }

    pub fn get_page_mut(&mut self, id: u64) -> Option<&mut ArchonPage> {
        self.pages.get_mut(id as usize)
    }

    pub fn apply_mutation(&mut self, mutation: &crate::telos_stub::StateMutation) {
        if let Some(page) = self.pages.get_mut(mutation.page_id as usize) {
            for (offset, data) in &mutation.field_updates {
                if let Some(end) = offset.checked_add(data.len()) {
                    if end <= page.data.len() {
                        page.data[*offset..end].copy_from_slice(data);
                        page.dirty = true;
                    }
                }
            }
        }
    }

    pub fn dirty_page_ids(&self) -> Vec<u64> {
        self.pages
            .iter()
            .filter(|p| p.dirty)
            .map(|p| p.id)
            .collect()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

impl Default for ArchonState {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchonBackend for ArchonState {
    fn get_page(&self, id: u64) -> Option<&ArchonPage> {
        self.get_page(id)
    }

    fn get_page_mut(&mut self, id: u64) -> Option<&mut ArchonPage> {
        self.get_page_mut(id)
    }

    fn apply_mutation(&mut self, mutation: &crate::telos_stub::StateMutation) {
        ArchonState::apply_mutation(self, mutation);
    }

    fn dirty_page_ids(&self) -> Vec<u64> {
        ArchonState::dirty_page_ids(self)
    }

    fn page_count(&self) -> usize {
        ArchonState::page_count(self)
    }
}

pub struct ChoraRuntime {
    archon: ArchonState,
    gpu_buffers: Vec<Arc<wgpu::Buffer>>,
    bound_pages: HashSet<u64>,
    page_to_buffer: HashMap<u64, usize>,
}

impl ChoraRuntime {
    pub fn new() -> Self {
        Self {
            archon: ArchonState::new(),
            gpu_buffers: Vec::new(),
            bound_pages: HashSet::new(),
            page_to_buffer: HashMap::new(),
        }
    }

    pub fn bind_state_to_gpu(
        &mut self,
        device: &wgpu::Device,
        page: &ArchonPage,
    ) -> Arc<wgpu::Buffer> {
        if page.dirty || !self.bound_pages.contains(&page.id) {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chora-archon-state"),
                contents: &page.data,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
            let idx = self.gpu_buffers.len();
            self.gpu_buffers.push(Arc::new(buffer));
            self.bound_pages.insert(page.id);
            self.page_to_buffer.insert(page.id, idx);
            Arc::clone(&self.gpu_buffers[idx])
        } else if let Some(&idx) = self.page_to_buffer.get(&page.id) {
            Arc::clone(&self.gpu_buffers[idx])
        } else {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chora-archon-state"),
                contents: &page.data,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
            let idx = self.gpu_buffers.len();
            self.gpu_buffers.push(Arc::new(buffer));
            self.page_to_buffer.insert(page.id, idx);
            Arc::clone(&self.gpu_buffers[idx])
        }
    }

    pub fn bind_state_to_gpu_direct(
        &mut self,
        device: &wgpu::Device,
        page: &ArchonPage,
    ) -> Arc<wgpu::Buffer> {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-archon-state"),
            contents: &page.data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let arc_buf = Arc::new(buffer);
        let idx = self.gpu_buffers.len();
        self.gpu_buffers.push(Arc::clone(&arc_buf));
        self.bound_pages.insert(page.id);
        self.page_to_buffer.insert(page.id, idx);
        arc_buf
    }

    pub fn unbind_page(&mut self, page_id: u64) {
        self.bound_pages.remove(&page_id);
    }

    pub fn archon(&self) -> &ArchonState {
        &self.archon
    }

    pub fn archon_mut(&mut self) -> &mut ArchonState {
        &mut self.archon
    }

    pub fn bound_page_count(&self) -> usize {
        self.bound_pages.len()
    }
}

impl Default for ChoraRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_layout() -> PageLayout {
        PageLayout {
            stride: 4,
            field_offsets: vec![("alpha".to_string(), 0), ("beta".to_string(), 8)],
        }
    }

    #[test]
    fn archon_state_new_empty() {
        let s = ArchonState::new();
        assert_eq!(s.page_count(), 0);
    }

    #[test]
    fn create_page_returns_zero_id() {
        let mut s = ArchonState::new();
        let id = s.create_page(vec![0u8; 16], test_layout());
        assert_eq!(id, 0);
        assert_eq!(s.page_count(), 1);
    }

    #[test]
    fn create_page_sequential_ids() {
        let mut s = ArchonState::new();
        let id0 = s.create_page(vec![0u8; 4], test_layout());
        let id1 = s.create_page(vec![0u8; 4], test_layout());
        let id2 = s.create_page(vec![0u8; 4], test_layout());
        assert_eq!((id0, id1, id2), (0, 1, 2));
        assert_eq!(s.page_count(), 3);
    }

    #[test]
    fn get_page_returns_data() {
        let mut s = ArchonState::new();
        let data = vec![10, 20, 30, 40];
        let id = s.create_page(data.clone(), test_layout());
        let page = s.get_page(id).unwrap();
        assert_eq!(page.data(), &data[..]);
        assert_eq!(page.id, id);
    }

    #[test]
    fn get_page_mut_modifies_data() {
        let mut s = ArchonState::new();
        let id = s.create_page(vec![0u8; 8], test_layout());
        let page = s.get_page_mut(id).unwrap();
        page.data[0] = 42;
        page.data[1] = 99;
        assert_eq!(s.get_page(id).unwrap().data(), &[42, 99, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn is_dirty_and_clear_dirty() {
        let mut s = ArchonState::new();
        let id = s.create_page(vec![0u8; 4], test_layout());
        assert!(s.get_page(id).unwrap().is_dirty());
        s.get_page_mut(id).unwrap().clear_dirty();
        assert!(!s.get_page(id).unwrap().is_dirty());
    }

    #[test]
    fn dirty_page_ids() {
        let mut s = ArchonState::new();
        let _id0 = s.create_page(vec![0u8; 4], test_layout());
        let id1 = s.create_page(vec![0u8; 4], test_layout());
        let _id2 = s.create_page(vec![0u8; 4], test_layout());
        s.get_page_mut(id1).unwrap().clear_dirty();
        let dirty = s.dirty_page_ids();
        assert_eq!(dirty, vec![0, 2]);
    }

    #[test]
    fn apply_mutation_updates_data() {
        let mut s = ArchonState::new();
        let id = s.create_page(vec![0u8; 16], test_layout());
        s.get_page_mut(id).unwrap().clear_dirty();
        let mutation = crate::telos_stub::StateMutation {
            page_id: id,
            field_updates: vec![(0, vec![7u8, 8u8])],
        };
        s.apply_mutation(&mutation);
        assert_eq!(s.get_page(id).unwrap().data()[0..2], [7, 8]);
        assert!(s.get_page(id).unwrap().is_dirty());
    }

    #[test]
    fn page_layout_field_offset() {
        let layout = test_layout();
        assert_eq!(layout.field_offset("alpha"), Some(0));
        assert_eq!(layout.field_offset("beta"), Some(8));
        assert_eq!(layout.field_offset("gamma"), None);
    }

    #[test]
    fn page_layout_total_size() {
        let layout = test_layout();
        assert_eq!(layout.total_size(), 8 + 4);
    }
}
