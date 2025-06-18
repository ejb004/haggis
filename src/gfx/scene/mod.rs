// src/gfx/scene/mod.rs
//! 3D scene management
//!
//! Contains objects, materials, vertices, and scene organization.

pub mod object;
pub mod scene;
pub mod vertex;

// Re-export main types
pub use object::{DrawObject, Object, ObjectBuilder};
pub use scene::Scene;
pub use vertex::Vertex3D;
