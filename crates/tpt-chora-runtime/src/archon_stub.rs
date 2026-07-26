use std::collections::HashSet;
use std::sync::Arc;
use wgpu::util::DeviceExt;

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
                if *offset + data.len() <= page.data.len() {
                    page.data[*offset..*offset + data.len()].copy_from_slice(data);
                    page.dirty = true;
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

pub struct ChoraRuntime {
    archon: ArchonState,
    gpu_buffers: Vec<Arc<wgpu::Buffer>>,
    bound_pages: HashSet<u64>,
}

impl ChoraRuntime {
    pub fn new() -> Self {
        Self {
            archon: ArchonState::new(),
            gpu_buffers: Vec::new(),
            bound_pages: HashSet::new(),
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
            Arc::clone(&self.gpu_buffers[idx])
        } else {
            let page_id = page.id;
            self.gpu_buffers
                .iter()
                .find(|_| self.bound_pages.contains(&page_id))
                .cloned()
                .unwrap_or_else(|| {
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("chora-archon-state"),
                        contents: &page.data,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    });
                    Arc::new(buffer)
                })
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
        self.gpu_buffers.push(Arc::clone(&arc_buf));
        self.bound_pages.insert(page.id);
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
