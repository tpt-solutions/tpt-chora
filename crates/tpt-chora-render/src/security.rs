pub mod capability;
pub mod viewport;
pub mod z_depth;

pub use capability::{CapabilityGuard, CapabilityToken, ShaderAccessViolation};
pub use viewport::ViewportGuard;
pub use z_depth::{HierarchicalZDepth, ZDepthViolation};

use crate::error::RenderError;

pub struct SecurityContext {
    pub capability: CapabilityGuard,
    pub viewport: ViewportGuard,
    pub z_depth: HierarchicalZDepth,
}

impl SecurityContext {
    pub fn new(
        owner_id: u64,
        tokens: capability::CapabilityToken,
        viewport_bounds: [f32; 4],
        z_depth: HierarchicalZDepth,
    ) -> Self {
        Self {
            capability: CapabilityGuard::new(owner_id, tokens),
            viewport: ViewportGuard::from_bounds(viewport_bounds),
            z_depth,
        }
    }

    pub fn validate_node(
        &self,
        texture_ids: &[u64],
        buffer_ids: &[u64],
    ) -> Result<(), RenderError> {
        self.capability
            .validate_shader_access(texture_ids, buffer_ids)
            .map_err(|e| RenderError::SecurityViolation(e.to_string()))
    }

    pub fn validate_z_depth(
        &self,
        parent_z: f32,
        child_z: f32,
        has_modal: bool,
    ) -> Result<(), RenderError> {
        self.z_depth
            .validate_hierarchy(parent_z, child_z, has_modal)
            .map_err(|e| RenderError::SecurityViolation(e.to_string()))
    }
}
