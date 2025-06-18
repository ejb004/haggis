// src/gfx/resources/mod.rs
//! GPU resource management
//!
//! Handles textures, buffers, and bind groups for rendering.

pub mod global_bindings;
pub mod material;
pub mod texture_resource;

// Re-export main types
pub use global_bindings::{update_global_ubo, GlobalBindings, GlobalUBO};
pub use texture_resource::TextureResource;
