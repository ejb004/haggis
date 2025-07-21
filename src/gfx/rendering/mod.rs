// src/gfx/rendering/mod.rs
//! Core rendering functionality
//!
//! Handles render pipelines, GPU resource management, and frame rendering.

pub mod pipeline_manager;
pub mod render_engine;
pub mod render_pass_ext;
pub mod visualization_renderer;

// Re-export main types
pub use pipeline_manager::{PipelineConfig, PipelineManager, PipelineStats};
pub use render_engine::RenderEngine;
pub use render_pass_ext::RenderPassExt;
pub use visualization_renderer::{VisualizationRenderer, VisualizationPlane};
