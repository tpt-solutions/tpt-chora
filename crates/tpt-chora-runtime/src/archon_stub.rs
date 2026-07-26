use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct ArchonPage {
    pub id: u64,
    data: Vec<u8>,
    layout: PageLayout,
}

#[derive(Debug, Clone)]
pub struct PageLayout {
    pub stride: usize,
    pub field_offsets: Vec<(String, usize)>,
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
        self.pages.push(ArchonPage { id, data, layout });
        id
    }

    pub fn get_page(&self, id: u64) -> Option<&ArchonPage> {
        self.pages.get(id as usize)
    }

    pub fn apply_mutation(&mut self, mutation: &crate::telos_stub::StateMutation) {
        if let Some(page) = self.pages.get_mut(mutation.page_id as usize) {
            for (offset, data) in &mutation.field_updates {
                if *offset + data.len() <= page.data.len() {
                    page.data[*offset..*offset + data.len()].copy_from_slice(data);
                }
            }
        }
    }
}

pub struct ChoraRuntime {
    archon: ArchonState,
    gpu_buffers: Vec<Arc<wgpu::Buffer>>,
}

impl ChoraRuntime {
    pub fn new() -> Self {
        Self {
            archon: ArchonState::new(),
            gpu_buffers: Vec::new(),
        }
    }

    pub fn bind_state_to_gpu(
        &mut self,
        device: &wgpu::Device,
        page: &ArchonPage,
    ) -> Arc<wgpu::Buffer> {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-archon-state"),
            contents: &page.data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let idx = self.gpu_buffers.len();
        self.gpu_buffers.push(Arc::new(buffer));
        Arc::clone(&self.gpu_buffers[idx])
    }

    pub fn archon(&self) -> &ArchonState {
        &self.archon
    }

    pub fn archon_mut(&mut self) -> &mut ArchonState {
        &mut self.archon
    }
}
