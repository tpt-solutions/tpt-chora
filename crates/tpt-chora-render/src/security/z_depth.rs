pub struct HierarchicalZDepth {
    base_z: f32,
    slice_size: f32,
    max_depth: f32,
}

impl HierarchicalZDepth {
    pub fn new(base_z: f32, slice_size: f32, max_depth: f32) -> Self {
        Self {
            base_z,
            slice_size,
            max_depth,
        }
    }

    pub fn compute_z(
        &self,
        parent_z: f32,
        sibling_index: u32,
        has_modal_capability: bool,
    ) -> Result<f32, ZDepthViolation> {
        let base = parent_z + self.slice_size;
        let offset = sibling_index as f32 * (self.slice_size / 256.0);
        let z = base + offset;

        if z > self.max_depth {
            return Err(ZDepthViolation::ExceedsMaxDepth {
                requested: z,
                max: self.max_depth,
            });
        }

        if !has_modal_capability && z - self.base_z > self.slice_size * 10.0 {
            return Err(ZDepthViolation::ModalRequired { requested_z: z });
        }

        Ok(z)
    }

    pub fn validate_hierarchy(
        &self,
        parent_z: f32,
        child_z: f32,
        has_modal_capability: bool,
    ) -> Result<(), ZDepthViolation> {
        if child_z <= parent_z {
            return Err(ZDepthViolation::ChildBehindParent { parent_z, child_z });
        }

        if child_z - parent_z > self.slice_size * 2.0 && !has_modal_capability {
            return Err(ZDepthViolation::GapExceedsSlice {
                gap: child_z - parent_z,
                slice_size: self.slice_size,
            });
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ZDepthViolation {
    ExceedsMaxDepth { requested: f32, max: f32 },
    ModalRequired { requested_z: f32 },
    ChildBehindParent { parent_z: f32, child_z: f32 },
    GapExceedsSlice { gap: f32, slice_size: f32 },
}

impl std::fmt::Display for ZDepthViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExceedsMaxDepth { requested, max } => {
                write!(f, "z-depth {} exceeds maximum {}", requested, max)
            }
            Self::ModalRequired { requested_z } => {
                write!(f, "z-depth {} requires Modal capability", requested_z,)
            }
            Self::ChildBehindParent { parent_z, child_z } => {
                write!(
                    f,
                    "child z-depth {} is behind parent z-depth {}",
                    child_z, parent_z
                )
            }
            Self::GapExceedsSlice { gap, slice_size } => {
                write!(
                    f,
                    "z-depth gap {} exceeds allowed slice size {}",
                    gap, slice_size
                )
            }
        }
    }
}
