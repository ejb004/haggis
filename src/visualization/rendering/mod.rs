//! Visualization Rendering Module
//!
//! This module provides a dedicated rendering system for visualization components,
//! completely separate from the regular scene object rendering pipeline.

pub mod bridge;
pub mod materials;
pub mod renderer;
pub mod shaders;

pub use bridge::{collect_visualization_planes, ToVisualizationPlane};
pub use materials::VisualizationMaterial;
pub use renderer::VisualizationRenderer;
