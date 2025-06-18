// Updated src/ui/mod.rs
//! User interface system
//!
//! ImGui-based UI management for engine controls and debugging.

pub mod manager;
pub mod panel;

// Re-export main types
pub use manager::UiManager;
pub use panel::default_transform_panel;
