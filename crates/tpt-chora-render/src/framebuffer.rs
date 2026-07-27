use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::RenderError;

#[derive(Debug)]
pub struct FrameBufferSet {
    buffers: Vec<FrameBuffer>,
    front_index: AtomicUsize,
    count: usize,
}

#[derive(Debug)]
struct FrameBuffer {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    staging: Option<wgpu::Buffer>,
    width: u32,
    height: u32,
}

impl FrameBufferSet {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, count: usize) -> Self {
        let count = count.clamp(2, 3);
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * height) as u64;

        let buffers = (0..count)
            .map(|_| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("chora-fb-set"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let staging = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("chora-fb-staging"),
                    size: buffer_size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                FrameBuffer {
                    texture,
                    view,
                    staging: Some(staging),
                    width,
                    height,
                }
            })
            .collect();

        Self {
            buffers,
            front_index: AtomicUsize::new(0),
            count,
        }
    }

    fn back_index(&self) -> usize {
        let front = self.front_index.load(Ordering::Acquire);
        if self.count == 3 {
            (front + 1) % 3
        } else {
            1 - front
        }
    }

    pub fn front_view(&self) -> &wgpu::TextureView {
        let idx = self.front_index.load(Ordering::Acquire);
        &self.buffers[idx].view
    }

    pub fn back_view(&self) -> &wgpu::TextureView {
        &self.buffers[self.back_index()].view
    }

    pub fn back_texture(&self) -> &wgpu::Texture {
        &self.buffers[self.back_index()].texture
    }

    pub fn swap(&self) {
        let back = self.back_index();
        self.front_index.store(back, Ordering::Release);
    }

    pub fn swap_next_triple(&self) {
        if self.count < 3 {
            self.swap();
            return;
        }
        let current_back = self.back_index();
        self.front_index.store(current_back, Ordering::Release);
    }

    pub fn read_back_rgba(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<u8>, RenderError> {
        let front = self.front_index.load(Ordering::Acquire);
        let fb = &self.buffers[front];
        let width = fb.width;
        let height = fb.height;

        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

        let staging = fb
            .staging
            .as_ref()
            .ok_or_else(|| RenderError::Readback("staging buffer missing".into()))?;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chora-fb-readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &fb.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: staging,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| RenderError::Readback(format!("channel closed: {e}")))?
            .map_err(|e| RenderError::Readback(format!("map failed: {e}")))?;

        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        staging.unmap();
        Ok(pixels)
    }

    pub fn buffer_count(&self) -> usize {
        self.count
    }
}
