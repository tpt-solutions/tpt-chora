pub mod inspector;
pub mod gpu_timing;
pub mod dirty_rect;
pub mod heatmap;
pub mod color_proof;
pub mod hot_reload;
pub mod error;

pub use inspector::{ChoraInspector, InspectorConfig};
pub use gpu_timing::{GpuTimer, TimingResult};
pub use dirty_rect::{DirtyRectTracker, DirtyRect};
pub use heatmap::{OverdrawHeatmap, HeatmapCell};
pub use color_proof::{ColorBlindnessMode, ColorProof};
pub use hot_reload::{HotReloader, ReloadEvent};
pub use error::InspectorError;
