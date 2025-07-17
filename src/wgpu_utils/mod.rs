//! # WGPU Utilities Module
//!
//! This module provides convenient wrappers, builders, and utility functions for common
//! wgpu operations in the Haggis engine. It simplifies GPU resource management and
//! provides a higher-level interface for working with wgpu.
//!
//! ## Key Features
//!
//! - **Binding Group Builders** - Fluent API for creating bind groups and layouts
//! - **Uniform Buffer Management** - Simplified uniform buffer creation and updates
//! - **Binding Type Helpers** - Convenient functions for common binding types
//! - **Resource Management** - Efficient GPU resource creation and management
//!
//! ## Architecture
//!
//! The utilities are organized into several modules:
//!
//! - [`binding_builder`] - Builder pattern for bind groups and layouts
//! - [`binding_types`] - Helper functions for common binding types
//! - [`uniform_buffer`] - Uniform buffer management utilities
//!
//! ## Usage
//!
//! These utilities are used throughout the Haggis engine to simplify GPU programming:
//!
//! ```no_run
//! use haggis::wgpu_utils::{BindGroupLayoutBuilder, UniformBuffer};
//!
//! // Create a bind group layout
//! let layout = BindGroupLayoutBuilder::new()
//!     .uniform_buffer(0, wgpu::ShaderStages::VERTEX)
//!     .texture(1, wgpu::ShaderStages::FRAGMENT)
//!     .sampler(2, wgpu::ShaderStages::FRAGMENT)
//!     .build(&device);
//!
//! // Create a uniform buffer
//! let buffer = UniformBuffer::new(&device, &data);
//! ```
//!
//! ## Design Philosophy
//!
//! The utilities follow these principles:
//! - **Simplicity** - Reduce boilerplate code for common operations
//! - **Safety** - Provide safe abstractions over raw wgpu operations
//! - **Performance** - Maintain high performance while adding convenience
//! - **Flexibility** - Support both simple and complex use cases

pub mod binding_builder;
pub mod binding_types;
pub mod uniform_buffer;

// Re-export main types for convenience
pub use binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc};
pub use binding_types::*;
pub use uniform_buffer::UniformBuffer;
