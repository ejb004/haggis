// src/gfx/rendering/mod.rs
//! Core rendering functionality
//!
//! Handles render pipelines, GPU resource management, and frame rendering.

pub mod pipeline_manager;
pub mod render_engine;

// Re-export main types
pub use pipeline_manager::{PipelineConfig, PipelineManager, PipelineStats};
pub use render_engine::RenderEngine;
