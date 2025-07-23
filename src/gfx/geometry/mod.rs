//! # Procedural Geometry Generation
//!
//! This module provides functions to generate common 3D primitive shapes procedurally,
//! eliminating the need for external model files for basic shapes.
//!
//! ## Supported Primitives
//!
//! - **Cube**: Unit cube with configurable subdivisions
//! - **Sphere**: UV sphere with configurable resolution
//! - **Plane**: Flat plane with configurable size and subdivisions
//!
//! ## Usage
//!
//! ```rust
//! use haggis::gfx::geometry::{generate_cube, generate_sphere, generate_plane};
//!
//! // Generate a unit cube
//! let cube_data = generate_cube();
//!
//! // Generate a sphere with 32 segments
//! let sphere_data = generate_sphere(32, 16);
//!
//! // Generate a 10x10 plane with 4 subdivisions
//! let plane_data = generate_plane(10.0, 10.0, 4, 4);
//! ```

pub mod primitives;

pub use primitives::*;

/// Represents generated geometry data ready for GPU upload
#[derive(Debug, Clone)]
pub struct GeometryData {
    /// Vertex positions (x, y, z)
    pub vertices: Vec<[f32; 3]>,
    /// Texture coordinates (u, v)
    pub tex_coords: Vec<[f32; 2]>,
    /// Normal vectors (x, y, z)
    pub normals: Vec<[f32; 3]>,
    /// Triangle indices (counter-clockwise winding)
    pub indices: Vec<u32>,
}

impl GeometryData {
    /// Create a new empty geometry data structure
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            tex_coords: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Get the number of vertices in this geometry
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of triangles in this geometry
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Convert to the format expected by the existing scene system
    /// This transforms the data into the vertex format used by the renderer
    pub fn to_scene_format(&self) -> (Vec<crate::gfx::scene::vertex::Vertex3D>, Vec<u32>) {
        use crate::gfx::scene::vertex::Vertex3D;
        
        let vertices: Vec<Vertex3D> = (0..self.vertices.len())
            .map(|i| {
                Vertex3D {
                    position: self.vertices[i],
                    normal: self.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                }
            })
            .collect();

        (vertices, self.indices.clone())
    }
}

impl Default for GeometryData {
    fn default() -> Self {
        Self::new()
    }
}