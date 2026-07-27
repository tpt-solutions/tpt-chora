#[derive(Debug)]
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

impl std::error::Error for ZDepthViolation {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_z_within_bounds() {
        let hz = HierarchicalZDepth::new(10.0, 1.0, 100.0);
        let z = hz.compute_z(10.0, 0, false).unwrap();
        assert_eq!(z, 11.0);
    }

    #[test]
    fn compute_z_exceeds_max() {
        let hz = HierarchicalZDepth::new(0.0, 10.0, 50.0);
        let err = hz.compute_z(45.0, 0, false).unwrap_err();
        match err {
            ZDepthViolation::ExceedsMaxDepth { requested, max } => {
                assert!(requested > max);
                assert_eq!(max, 50.0);
            }
            _ => panic!("expected ExceedsMaxDepth"),
        }
    }

    #[test]
    fn compute_z_modal_required() {
        let hz = HierarchicalZDepth::new(0.0, 1.0, 200.0);
        let err = hz.compute_z(11.0, 0, false).unwrap_err();
        match err {
            ZDepthViolation::ModalRequired { .. } => {}
            _ => panic!("expected ModalRequired"),
        }
    }

    #[test]
    fn compute_z_modal_allowed() {
        let hz = HierarchicalZDepth::new(0.0, 1.0, 200.0);
        let z = hz.compute_z(11.0, 0, true).unwrap();
        assert_eq!(z, 12.0);
    }

    #[test]
    fn validate_hierarchy_valid() {
        let hz = HierarchicalZDepth::new(0.0, 5.0, 100.0);
        assert!(hz.validate_hierarchy(10.0, 15.0, false).is_ok());
    }

    #[test]
    fn validate_hierarchy_child_behind_parent() {
        let hz = HierarchicalZDepth::new(0.0, 5.0, 100.0);
        let err = hz.validate_hierarchy(20.0, 15.0, false).unwrap_err();
        match err {
            ZDepthViolation::ChildBehindParent { parent_z, child_z } => {
                assert_eq!(parent_z, 20.0);
                assert_eq!(child_z, 15.0);
            }
            _ => panic!("expected ChildBehindParent"),
        }
    }

    #[test]
    fn validate_hierarchy_child_equal_to_parent() {
        let hz = HierarchicalZDepth::new(0.0, 5.0, 100.0);
        let err = hz.validate_hierarchy(10.0, 10.0, false).unwrap_err();
        assert!(matches!(err, ZDepthViolation::ChildBehindParent { .. }));
    }

    #[test]
    fn validate_hierarchy_gap_exceeds_slice() {
        let hz = HierarchicalZDepth::new(0.0, 5.0, 200.0);
        let err = hz.validate_hierarchy(0.0, 11.0, false).unwrap_err();
        match err {
            ZDepthViolation::GapExceedsSlice { gap, slice_size } => {
                assert!((gap - 11.0).abs() < f32::EPSILON);
                assert_eq!(slice_size, 5.0);
            }
            _ => panic!("expected GapExceedsSlice"),
        }
    }

    #[test]
    fn validate_hierarchy_gap_with_modal_allowed() {
        let hz = HierarchicalZDepth::new(0.0, 5.0, 200.0);
        assert!(hz.validate_hierarchy(0.0, 11.0, true).is_ok());
    }

    #[test]
    fn z_depth_violation_is_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(ZDepthViolation::ModalRequired { requested_z: 5.0 });
        assert!(err.to_string().contains("Modal"));
    }
}
