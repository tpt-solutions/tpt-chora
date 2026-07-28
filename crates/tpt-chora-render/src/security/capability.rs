use std::cell::RefCell;
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

/// `granted_textures`/`granted_buffers` use interior mutability so
/// `RenderGraph::execute` can grant a node access to the transient
/// resources it `creates` (see `graph.rs`) through a shared
/// `&SecurityContext`, without requiring `Renderer::render_frame` to take
/// `&mut self` just to thread a mutable capability guard through the whole
/// per-frame call chain.
#[derive(Debug)]
pub struct CapabilityGuard {
    owner_id: u64,
    allowed_tokens: CapabilityToken,
    granted_textures: RefCell<HashSet<u64>>,
    granted_buffers: RefCell<HashSet<u64>>,
}

impl CapabilityGuard {
    pub fn new(owner_id: u64, tokens: CapabilityToken) -> Self {
        Self {
            owner_id,
            allowed_tokens: tokens,
            granted_textures: RefCell::new(HashSet::new()),
            granted_buffers: RefCell::new(HashSet::new()),
        }
    }

    pub fn can_access_texture(&self, texture_id: u64) -> bool {
        self.granted_textures.borrow().contains(&texture_id)
    }

    pub fn can_access_buffer(&self, buffer_id: u64) -> bool {
        self.granted_buffers.borrow().contains(&buffer_id)
    }

    pub fn has_token(&self, token: CapabilityToken) -> bool {
        self.allowed_tokens.contains(token)
    }

    pub fn grant_texture(&self, texture_id: u64) -> Result<(), ShaderAccessViolation> {
        if !self.has_token(CapabilityToken::TEXTURE_READ) {
            return Err(ShaderAccessViolation::TextureDenied {
                owner: self.owner_id,
                texture: texture_id,
            });
        }
        self.granted_textures.borrow_mut().insert(texture_id);
        Ok(())
    }

    pub fn grant_buffer(&self, buffer_id: u64) -> Result<(), ShaderAccessViolation> {
        if !self.has_token(CapabilityToken::UNIFORM_READ)
            && !self.has_token(CapabilityToken::STORAGE_READ)
        {
            return Err(ShaderAccessViolation::BufferDenied {
                owner: self.owner_id,
                buffer: buffer_id,
            });
        }
        self.granted_buffers.borrow_mut().insert(buffer_id);
        Ok(())
    }

    pub fn revoke_texture(&self, texture_id: u64) {
        self.granted_textures.borrow_mut().remove(&texture_id);
    }

    pub fn revoke_buffer(&self, buffer_id: u64) {
        self.granted_buffers.borrow_mut().remove(&buffer_id);
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

impl std::error::Error for ShaderAccessViolation {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn guard() -> CapabilityGuard {
        CapabilityGuard::new(
            1,
            CapabilityToken::TEXTURE_READ
                | CapabilityToken::TEXTURE_WRITE
                | CapabilityToken::STORAGE_READ,
        )
    }

    #[test]
    fn grant_and_revoke_texture() {
        let g = guard();
        assert!(!g.can_access_texture(42));
        let _ = g.grant_texture(42);
        assert!(g.can_access_texture(42));
        g.revoke_texture(42);
        assert!(!g.can_access_texture(42));
    }

    #[test]
    fn grant_and_revoke_buffer() {
        let g = guard();
        assert!(!g.can_access_buffer(7));
        let _ = g.grant_buffer(7);
        assert!(g.can_access_buffer(7));
        g.revoke_buffer(7);
        assert!(!g.can_access_buffer(7));
    }

    #[test]
    fn has_token() {
        let g = guard();
        assert!(g.has_token(CapabilityToken::TEXTURE_READ));
        assert!(g.has_token(CapabilityToken::STORAGE_READ));
        assert!(!g.has_token(CapabilityToken::MODAL));
        assert!(!g.has_token(CapabilityToken::RENDER_TARGET));
    }

    #[test]
    fn validate_shader_access_pass() {
        let g = guard();
        let _ = g.grant_texture(10);
        let _ = g.grant_texture(20);
        let _ = g.grant_buffer(30);

        assert!(g.validate_shader_access(&[10, 20], &[30]).is_ok());
    }

    #[test]
    fn validate_shader_access_texture_denied() {
        let g = guard();
        let _ = g.grant_buffer(30);

        let err = g.validate_shader_access(&[10], &[30]).unwrap_err();
        match err {
            ShaderAccessViolation::TextureDenied { owner, texture } => {
                assert_eq!(owner, 1);
                assert_eq!(texture, 10);
            }
            _ => panic!("expected TextureDenied"),
        }
    }

    #[test]
    fn validate_shader_access_buffer_denied() {
        let g = guard();
        let _ = g.grant_texture(10);

        let err = g.validate_shader_access(&[10], &[30]).unwrap_err();
        match err {
            ShaderAccessViolation::BufferDenied { owner, buffer } => {
                assert_eq!(owner, 1);
                assert_eq!(buffer, 30);
            }
            _ => panic!("expected BufferDenied"),
        }
    }

    #[test]
    fn shader_access_violation_is_error() {
        let err: Box<dyn std::error::Error> = Box::new(ShaderAccessViolation::TextureDenied {
            owner: 1,
            texture: 2,
        });
        assert!(err.to_string().contains("denied access to texture"));
    }

    proptest::proptest! {
        #[test]
        fn grant_texture_succeeds_when_token_present(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::TEXTURE_READ);
            prop_assert!(g.grant_texture(id).is_ok());
            prop_assert!(g.can_access_texture(id));
        }

        #[test]
        fn grant_texture_fails_without_token(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::empty());
            prop_assert!(g.grant_texture(id).is_err());
            prop_assert!(!g.can_access_texture(id));
        }

        #[test]
        fn grant_buffer_succeeds_with_uniform_read(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::UNIFORM_READ);
            prop_assert!(g.grant_buffer(id).is_ok());
            prop_assert!(g.can_access_buffer(id));
        }

        #[test]
        fn grant_buffer_succeeds_with_storage_read(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::STORAGE_READ);
            prop_assert!(g.grant_buffer(id).is_ok());
            prop_assert!(g.can_access_buffer(id));
        }

        #[test]
        fn grant_buffer_fails_without_any_read_token(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::empty());
            prop_assert!(g.grant_buffer(id).is_err());
            prop_assert!(!g.can_access_buffer(id));
        }

        #[test]
        fn revoke_texture_removes_access(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::TEXTURE_READ);
            let _ = g.grant_texture(id);
            g.revoke_texture(id);
            prop_assert!(!g.can_access_texture(id));
        }

        #[test]
        fn revoke_buffer_removes_access(id in 0u64..u64::MAX) {
            let g = CapabilityGuard::new(0, CapabilityToken::UNIFORM_READ);
            let _ = g.grant_buffer(id);
            g.revoke_buffer(id);
            prop_assert!(!g.can_access_buffer(id));
        }

        #[test]
        fn validate_shader_access_ok_after_grant(
            tex_id in 0u64..100u64,
            buf_id in 0u64..100u64,
        ) {
            let g = CapabilityGuard::new(
                0,
                CapabilityToken::TEXTURE_READ | CapabilityToken::UNIFORM_READ,
            );
            let _ = g.grant_texture(tex_id);
            let _ = g.grant_buffer(buf_id);
            prop_assert!(g.validate_shader_access(&[tex_id], &[buf_id]).is_ok());
        }

        #[test]
        fn validate_shader_access_denies_ungranted(
            tex_id in 0u64..100u64,
            buf_id in 0u64..100u64,
        ) {
            let g = CapabilityGuard::new(
                0,
                CapabilityToken::TEXTURE_READ | CapabilityToken::UNIFORM_READ,
            );
            prop_assert!(g.validate_shader_access(&[tex_id], &[buf_id]).is_err());
        }

        #[test]
        fn has_token_respects_bitflags(tokens in 0u32..=255u32) {
            let token = CapabilityToken::from_bits_truncate(tokens);
            let g = CapabilityGuard::new(0, token);
            for bit in 0..8 {
                let flag = CapabilityToken::from_bits_truncate(1 << bit);
                prop_assert_eq!(g.has_token(flag), tokens & (1 << bit) != 0);
            }
        }
    }
}
