#![forbid(unsafe_code)]

//! `tpt-chora-render`: the Core Rendering Engine (spec.txt §2.1, "The
//! Canvas") — the foundational GPU pipeline responsible for rasterizing
//! geometry, managing framebuffers, and executing shaders via wgpu.
//!
//! This crate currently implements:
//! - [`graph`]: the frame-scoped, dependency-tracked render graph.
//! - [`vector`]: GPU-compute-shader Bezier/path tessellation.
//! - [`postprocess`]: the built-in post-processing pipeline (color grading).
//! - [`renderer`]: wires the above together into a headless, off-screen
//!   `Renderer`.

pub mod error;
pub mod framebuffer;
pub mod graph;
pub mod postprocess;
pub mod renderer;
pub mod security;
#[cfg(feature = "spatial")]
pub mod spatial;
pub mod vector;

pub use error::RenderError;
pub use framebuffer::FrameBufferSet;
pub use graph::{GraphNode, NodeExecuteCtx, RenderGraph, ResourceId, TransientTextureDesc};
pub use postprocess::{ColorGradeParams, PostProcessPipeline};
pub use renderer::{GpuContext, Renderer};
pub use security::capability::{CapabilityGuard, CapabilityToken, ShaderAccessViolation};
pub use security::viewport::ViewportGuard;
pub use security::z_depth::{HierarchicalZDepth, ZDepthViolation};
pub use security::SecurityContext;
pub use vector::{circle_path, tessellate_cubics_gpu, CubicBezier};
