use std::collections::HashMap;

use crate::decode::DecodedImage;

pub struct GpuTextureCache {
    textures: HashMap<u64, CachedTexture>,
    next_id: u64,
    max_size_bytes: usize,
    current_size_bytes: usize,
}

pub struct CachedTexture {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size_bytes: usize,
    pub last_used_frame: u64,
}

impl GpuTextureCache {
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            textures: HashMap::new(),
            next_id: 0,
            max_size_bytes,
            current_size_bytes: 0,
        }
    }

    pub fn insert(
        &mut self,
        device: &wgpu::Device,
        image: &DecodedImage,
    ) -> Result<u64, crate::MediaError> {
        let size_bytes = (image.width * image.height * 4) as usize;

        while self.current_size_bytes + size_bytes > self.max_size_bytes {
            self.evict_oldest();
        }

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("chora-media-texture"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let id = self.next_id;
        self.next_id += 1;

        self.textures.insert(
            id,
            CachedTexture {
                id,
                width: image.width,
                height: image.height,
                format,
                texture,
                view,
                size_bytes,
                last_used_frame: 0,
            },
        );

        self.current_size_bytes += size_bytes;
        Ok(id)
    }

    pub fn insert_to_queue(
        &mut self,
        queue: &wgpu::Queue,
        id: u64,
        image: &DecodedImage,
    ) {
        if let Some(cached) = self.textures.get(&id) {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &cached.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &image.data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width * 4),
                    rows_per_image: Some(image.height),
                },
                wgpu::Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn get(&mut self, id: u64, current_frame: u64) -> Option<&CachedTexture> {
        self.textures.get_mut(&id).map(|t| {
            t.last_used_frame = current_frame;
            &*t
        })
    }

    pub fn remove(&mut self, id: u64) -> Option<CachedTexture> {
        let removed = self.textures.remove(&id);
        if let Some(ref tex) = removed {
            self.current_size_bytes -= tex.size_bytes;
        }
        removed
    }

    fn evict_oldest(&mut self) {
        if let Some((&oldest_id, _)) = self
            .textures
            .iter()
            .min_by_key(|(_, t)| t.last_used_frame)
        {
            self.remove(oldest_id);
        }
    }

    pub fn current_size_bytes(&self) -> usize {
        self.current_size_bytes
    }

    pub fn count(&self) -> usize {
        self.textures.len()
    }
}
