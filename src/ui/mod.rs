//! # User Interface Module
//!
//! This module provides a Dear ImGui-based user interface system for the Haggis engine.
//! It handles UI rendering, input management, and provides default UI panels for
//! common engine operations.
//!
//! ## Architecture
//!
//! The UI system is built around the [`UiManager`] which handles:
//! - ImGui integration with winit and wgpu
//! - UI input capture and event handling
//! - Frame-by-frame UI rendering
//! - Font management and scaling
//!
//! ## Key Components
//!
//! - [`UiManager`] - Core UI manager that handles ImGui integration
//! - [`panel`] - Pre-built UI panels for common operations
//! - [`default_transform_panel`] - Default object transform editor
//!
//! ## Usage
//!
//! The UI system is typically used through the main [`HaggisApp`] interface:
//!
//! ```no_run
//! use haggis::HaggisApp;
//!
//! let mut app = haggis::default();
//! app.set_ui(|ui, scene, selected| {
//!     // Custom UI code here
//!     ui.window("My Panel").build(|| {
//!         ui.text("Hello, World!");
//!     });
//! });
//! ```
//!
//! ## Default Panels
//!
//! The module provides several default panels:
//! - **Transform Panel** - Edit object position, rotation, and scale
//! - **Material Panel** - Adjust material properties
//! - **Simulation Panel** - Control simulation parameters
//!
//! ## Input Handling
//!
//! The UI system properly handles input capture to prevent conflicts with
//! camera controls. When the UI is focused, camera movement is disabled.
//!
//! [`HaggisApp`]: crate::app::HaggisApp

pub mod manager;
pub mod panel;

// Re-export main types
pub use manager::UiManager;
pub use panel::default_transform_panel;
