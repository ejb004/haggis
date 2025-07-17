//! # Haggis 3D Engine
//!
//! A modern 3D rendering and simulation engine built on [wgpu](https://wgpu.rs) and [winit](https://github.com/rust-windowing/winit).
//!
//! ## Features
//!
//! - **Modern Graphics Pipeline**: Physically-based rendering (PBR) with shadow mapping
//! - **Flexible Simulation Framework**: Support for both CPU and GPU compute simulations
//! - **Interactive UI**: Runtime controls using Dear ImGui
//! - **Resource Management**: Efficient handling of textures, materials, and GPU buffers
//! - **Camera System**: Orbit camera with smooth controls
//! - **Cross-Platform**: Runs on Windows, macOS, and Linux
//!
//! ## Quick Start
//!
//! ```no_run
//! use haggis;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut app = haggis::default();
//!     
//!     // Add a 3D object to the scene
//!     app.add_object("model.obj")
//!         .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
//!     
//!     // Set up custom UI
//!     app.set_ui(|ui, scene, _selected| {
//!         ui.window("Controls").build(|| {
//!             ui.text("Welcome to Haggis!");
//!         });
//!     });
//!     
//!     // Run the application
//!     app.run();
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The engine is organized into several key modules:
//!
//! - [`app`] - Main application lifecycle and event handling
//! - [`gfx`] - Graphics rendering, camera system, and scene management
//! - [`simulation`] - CPU and GPU simulation framework
//! - [`ui`] - User interface system using Dear ImGui
//! - [`wgpu_utils`] - Utility functions for wgpu resource management

pub mod app;
pub mod gfx;
pub mod simulation;
pub mod ui;
pub mod wgpu_utils;

// Re-export main types for convenience
pub use app::HaggisApp;

/// Creates a default Haggis application instance.
///
/// This is a convenience function that creates a new [`HaggisApp`] with default settings,
/// including an orbit camera positioned 8 units from the origin.
///
/// # Returns
///
/// A [`HaggisApp`] ready for configuration and execution.
///
/// # Examples
///
/// ```no_run
/// let mut app = haggis::default();
/// app.add_object("model.obj");
/// app.run();
/// ```
pub fn default() -> HaggisApp {
    pollster::block_on(HaggisApp::new())
}
