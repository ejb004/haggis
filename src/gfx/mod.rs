// src/gfx/mod.rs
//! Graphics rendering system
//!
//! Contains all graphics-related functionality including cameras, rendering,
//! scene management, and resource handling.

pub mod camera;
pub mod rendering;
pub mod resources;
pub mod scene;

// Re-export commonly used types
pub use camera::orbit_camera::OrbitCamera;
pub use rendering::render_engine::RenderEngine;
