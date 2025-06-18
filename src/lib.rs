// src/lib.rs
//! Haggis 3D Engine
//!
//! A flexible 3D rendering and simulation engine built on wgpu and winit.

pub mod app;
pub mod gfx;
pub mod simulation;
pub mod ui;
pub mod wgpu_utils;

// Re-export main types for convenience
pub use app::HaggisApp;

/// Creates a default Haggis application instance
pub fn default() -> HaggisApp {
    pollster::block_on(HaggisApp::new())
}
