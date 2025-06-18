// src/wgpu_utils/mod.rs
//! WGPU utility functions and helpers
//!
//! Provides convenient wrappers and builders for common wgpu operations.

pub mod binding_builder;
pub mod binding_types;
pub mod uniform_buffer;

// Re-export main types
pub use binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc};
pub use binding_types::*;
pub use uniform_buffer::UniformBuffer;
