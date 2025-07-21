//! # Visualization Module
//!
//! This module provides a modular visualization system for the Haggis engine.
//! It allows for adding various visualization components that can be attached to
//! simulations or used independently to visualize 3D data.
//!
//! ## Architecture
//!
//! The visualization system is built around the [`VisualizationComponent`] trait
//! which provides a common interface for all visualization types. Each component
//! manages its own rendering and UI controls.
//!
//! ## Key Components
//!
//! - [`CutPlane2D`] - 2D cross-section visualization of 3D data
//! - [`VisualizationManager`] - Manages multiple visualization components
//! - [`ui`] - UI panels for visualization controls
//!
//! ## Usage
//!
//! ```no_run
//! use haggis::visualization::{CutPlane2D, VisualizationManager};
//!
//! let mut viz_manager = VisualizationManager::new();
//! let cut_plane = CutPlane2D::new();
//! viz_manager.add_component("cut_plane", Box::new(cut_plane));
//! ```

pub mod cut_plane_2d;
pub mod manager;
pub mod traits;
pub mod ui;
pub mod rendering;

// Re-export main types
pub use cut_plane_2d::CutPlane2D;
pub use manager::VisualizationManager;
pub use traits::VisualizationComponent;
pub use rendering::{VisualizationRenderer, VisualizationMaterial};