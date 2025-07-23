//! # Haggis Prelude
//!
//! This module provides a convenient way to import commonly used types and traits
//! from the Haggis engine. It's designed to reduce boilerplate imports in typical
//! Haggis applications and simulations.
//!
//! ## Usage
//!
//! ```rust
//! use haggis::prelude::*;
//! ```
//!
//! This brings all essential types into scope, allowing you to write:
//!
//! ```no_run
//! use haggis::prelude::*;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut app = haggis::default();
//!     
//!     // Scene and simulation types are now available
//!     app.attach_simulation(MySimulation::new());
//!     app.run();
//!     Ok(())
//! }
//!
//! struct MySimulation {
//!     running: bool,
//! }
//!
//! impl Simulation for MySimulation {
//!     fn update(&mut self, delta_time: f32, scene: &mut Scene, _device: Option<&wgpu::Device>, _queue: Option<&wgpu::Queue>) {
//!         // Simulation logic here
//!     }
//!     
//!     fn render_ui(&mut self, ui: &Ui, _scene: &mut Scene) {
//!         ui.text("Custom simulation UI");
//!     }
//!     
//!     fn name(&self) -> &str { "My Simulation" }
//!     fn is_running(&self) -> bool { self.running }
//!     fn set_running(&mut self, running: bool) { self.running = running; }
//! }
//! ```

// Re-export core application types
pub use crate::app::HaggisApp;
pub use crate::default;

// Re-export graphics and scene types
pub use crate::gfx::scene::Scene;
pub use crate::gfx::camera::CameraManager;
pub use crate::gfx::geometry::{GeometryData, generate_cube, generate_sphere, generate_plane, generate_cylinder};

// Re-export simulation framework 
pub use crate::simulation::traits::Simulation;
pub use crate::simulation::manager::SimulationManager;

// Re-export UI types and utilities
pub use crate::ui::{UiFont, UiStyle, default_transform_panel};

// Re-export visualization types
pub use crate::visualization::{
    CutPlane2D, 
    VisualizationComponent, 
    VisualizationManager
};

// Re-export performance monitoring
pub use crate::performance::{PerformanceMonitor, PerformanceMetrics};

// Re-export common external dependencies
pub use cgmath::{Vector3, InnerSpace, Zero};
pub use imgui::Ui;

// Re-export common standard library types
pub use std::collections::VecDeque;
pub use std::time::Instant;

// Re-export wgpu types commonly used in GPU simulations
pub use wgpu::{Device, Queue};