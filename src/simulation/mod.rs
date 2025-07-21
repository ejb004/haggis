//! # Simulation Module
//!
//! This module provides a flexible simulation framework that supports both CPU and GPU-based
//! simulations. It enables real-time physics, animation, and procedural content generation
//! integrated with the Haggis 3D engine.
//!
//! ## Architecture
//!
//! The simulation system is built around a trait-based architecture that allows for:
//!
//! - **CPU Simulations** - Traditional Rust-based simulations with threading support
//! - **GPU Simulations** - High-performance compute shader-based simulations
//! - **Hybrid Simulations** - Combinations of CPU and GPU processing
//! - **Dynamic Switching** - Runtime switching between simulation modes
//!
//! ## Key Components
//!
//! - [`traits::Simulation`] - Core simulation trait that all simulations must implement
//! - [`manager::SimulationManager`] - Manages simulation lifecycle and execution
//! - [`cpu`] - CPU-based simulation utilities and examples
//! - [`gpu`] - GPU compute shader simulation utilities and examples
//! - [`examples`] - Ready-to-use simulation examples for both CPU and GPU
//!
//! ## Usage
//!
//! Simulations are typically attached to a [`HaggisApp`] through the simulation manager:
//!
//! ```no_run
//! use haggis::HaggisApp;
//! use haggis::simulation::traits::Simulation;
//!
//! struct MySimulation;
//! impl Simulation for MySimulation {
//!     fn update(&mut self, dt: f32, scene: &mut haggis::gfx::scene::Scene, device: Option<&wgpu::Device>, queue: Option<&wgpu::Queue>) {
//!         // Simulation logic here
//!     }
//!     fn name(&self) -> &str { "My Simulation" }
//!     fn render_ui(&mut self, ui: &imgui::Ui, scene: &mut haggis::gfx::scene::Scene) {
//!         // UI controls here
//!     }
//! }
//!
//! let mut app = haggis::default();
//! app.attach_simulation(MySimulation);
//! app.run();
//! ```
//!
//! ## Examples
//!
//! The module includes several examples:
//! - **CPU Examples**: Simple movement, physics simulations
//! - **GPU Examples**: Particle systems, compute shader simulations
//!
//! [`HaggisApp`]: crate::app::HaggisApp

pub mod base_simulation;
pub mod cpu;
pub mod examples;
pub mod gpu;
pub mod manager;
pub mod traits;

// New API layers
pub mod high_level;
pub mod mid_level;
pub mod low_level;

// Re-export for convenience
pub use base_simulation::BaseSimulation;
pub use high_level::{ParticleSystem, ParticleSimulation, ForceField, Constraint};
pub use mid_level::{ManagedSimulation, SimulationExt, GpuResourceManager};
pub use low_level::{ComputeContext, RawGpuSimulation, GpuParticle};
