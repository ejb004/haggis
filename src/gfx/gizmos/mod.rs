//! # Gizmo System
//!
//! This module provides a modular gizmo system for the Haggis engine.
//! Gizmos are visual aids that help with debugging, visualization, and interaction
//! in 3D space. They can represent positions, orientations, paths, bounds, and more.
//!
//! ## Architecture
//!
//! The gizmo system is built around the [`Gizmo`] trait which provides a common
//! interface for all gizmo types. The [`GizmoManager`] handles the lifecycle
//! and rendering of multiple gizmos.
//!
//! ## Key Components
//!
//! - [`Gizmo`] - Base trait for all gizmo implementations
//! - [`GizmoManager`] - Manages multiple gizmo instances
//! - [`CameraGizmo`] - Shows camera positions and movement history
//!
//! ## Usage
//!
//! ```no_run
//! use haggis::gfx::gizmos::{GizmoManager, CameraGizmo};
//!
//! let mut gizmo_manager = GizmoManager::new();
//! let camera_gizmo = CameraGizmo::new();
//! gizmo_manager.add_gizmo("camera", Box::new(camera_gizmo));
//! ```

pub mod camera_gizmo;
pub mod manager;
pub mod traits;
pub mod viewport_gizmo;

#[cfg(test)]
mod test_viewport;

// Re-export main types
pub use camera_gizmo::CameraGizmo;
pub use manager::GizmoManager;
pub use traits::Gizmo;
pub use viewport_gizmo::{ViewportGizmo, ViewDirection};