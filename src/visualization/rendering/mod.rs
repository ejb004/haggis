//! Visualization Rendering Module
//!
//! This module provides a dedicated rendering system for visualization components,
//! completely separate from the regular scene object rendering pipeline.

pub mod renderer;
pub mod shaders;
pub mod materials;
pub mod bridge;

pub use renderer::VisualizationRenderer;
pub use materials::VisualizationMaterial;
pub use bridge::{ToVisualizationPlane, collect_visualization_planes};