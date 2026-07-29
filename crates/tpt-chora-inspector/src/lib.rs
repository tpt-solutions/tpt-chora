#![forbid(unsafe_code)]

pub mod color_proof;
pub mod dirty_rect;
pub mod error;
pub mod gpu_timing;
pub mod heatmap;
pub mod hot_reload;
pub mod inspector;

pub use color_proof::{ColorBlindnessMode, ColorProof};
pub use dirty_rect::{DirtyRect, DirtyRectTracker};
pub use error::InspectorError;
pub use gpu_timing::{GpuTimer, TimingResult};
pub use heatmap::{HeatmapCell, OverdrawHeatmap};
pub use hot_reload::{HotReloader, ReloadEvent};
pub use inspector::{ChoraInspector, InspectorConfig, InspectorFrameData};
