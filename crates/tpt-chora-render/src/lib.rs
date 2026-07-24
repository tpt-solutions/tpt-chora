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
pub mod graph;
pub mod postprocess;
pub mod renderer;
pub mod vector;

pub use error::RenderError;
pub use graph::{GraphNode, NodeExecuteCtx, RenderGraph, ResourceId, TransientTextureDesc};
pub use postprocess::{ColorGradeParams, PostProcessPipeline};
pub use renderer::{GpuContext, Renderer};
pub use vector::{circle_path, tessellate_cubics_gpu, CubicBezier};
