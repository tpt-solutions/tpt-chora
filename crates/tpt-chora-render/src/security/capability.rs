use std::collections::HashSet;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CapabilityToken: u32 {
        const TEXTURE_READ = 0b0000_0001;
        const TEXTURE_WRITE = 0b0000_0010;
        const UNIFORM_READ = 0b0000_0100;
        const STORAGE_READ = 0b0000_1000;
        const STORAGE_WRITE = 0b0001_0000;
        const SAMPLER = 0b0010_0000;
        const RENDER_TARGET = 0b0100_0000;
        const MODAL = 0b1000_0000;
    }
}

pub struct CapabilityGuard {
    owner_id: u64,
    allowed_tokens: CapabilityToken,
    granted_textures: HashSet<u64>,
    granted_buffers: HashSet<u64>,
}

impl CapabilityGuard {
    pub fn new(owner_id: u64, tokens: CapabilityToken) -> Self {
        Self {
            owner_id,
            allowed_tokens: tokens,
            granted_textures: HashSet::new(),
            granted_buffers: HashSet::new(),
        }
    }

    pub fn can_access_texture(&self, texture_id: u64) -> bool {
        self.granted_textures.contains(&texture_id)
    }

    pub fn can_access_buffer(&self, buffer_id: u64) -> bool {
        self.granted_buffers.contains(&buffer_id)
    }

    pub fn has_token(&self, token: CapabilityToken) -> bool {
        self.allowed_tokens.contains(token)
    }

    pub fn grant_texture(&mut self, texture_id: u64) {
        self.granted_textures.insert(texture_id);
    }

    pub fn grant_buffer(&mut self, buffer_id: u64) {
        self.granted_buffers.insert(buffer_id);
    }

    pub fn revoke_texture(&mut self, texture_id: u64) {
        self.granted_textures.remove(&texture_id);
    }

    pub fn revoke_buffer(&mut self, buffer_id: u64) {
        self.granted_buffers.remove(&buffer_id);
    }

    pub fn validate_shader_access(
        &self,
        bind_group_textures: &[u64],
        bind_group_buffers: &[u64],
    ) -> Result<(), ShaderAccessViolation> {
        for &tex in bind_group_textures {
            if !self.can_access_texture(tex) {
                return Err(ShaderAccessViolation::TextureDenied {
                    owner: self.owner_id,
                    texture: tex,
                });
            }
        }
        for &buf in bind_group_buffers {
            if !self.can_access_buffer(buf) {
                return Err(ShaderAccessViolation::BufferDenied {
                    owner: self.owner_id,
                    buffer: buf,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ShaderAccessViolation {
    TextureDenied { owner: u64, texture: u64 },
    BufferDenied { owner: u64, buffer: u64 },
}

impl std::fmt::Display for ShaderAccessViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextureDenied { owner, texture } => {
                write!(
                    f,
                    "component {} denied access to texture {}",
                    owner, texture
                )
            }
            Self::BufferDenied { owner, buffer } => {
                write!(f, "component {} denied access to buffer {}", owner, buffer)
            }
        }
    }
}
