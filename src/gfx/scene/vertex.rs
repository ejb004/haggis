//! # Vertex Data Structures
//!
//! This module defines vertex data structures used for 3D mesh rendering
//! in the Haggis engine. It provides GPU-compatible vertex formats.

/// A 3D vertex with position and normal data.
///
/// This structure represents a single vertex in 3D space with its position
/// and normal vector. It's designed to be efficiently passed to GPU shaders
/// for rendering.
///
/// # Memory Layout
///
/// The `#[repr(C)]` attribute ensures the struct has a C-compatible memory
/// layout, which is required for GPU buffer operations.
///
/// # Fields
///
/// - `position`: 3D position coordinates [x, y, z]
/// - `normal`: 3D normal vector [nx, ny, nz] for lighting calculations
///
/// # Examples
///
/// ```no_run
/// use haggis::gfx::scene::vertex::Vertex3D;
///
/// let vertex = Vertex3D {
///     position: [0.0, 1.0, 0.0],
///     normal: [0.0, 1.0, 0.0],
/// };
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    /// 3D position coordinates [x, y, z]
    pub position: [f32; 3],
    /// 3D normal vector [nx, ny, nz] for lighting calculations
    pub normal: [f32; 3],
}

impl Vertex3D {
    /// Returns the vertex buffer layout for wgpu rendering.
    ///
    /// This method provides the vertex attribute layout that describes
    /// how the vertex data should be interpreted by the GPU shaders.
    ///
    /// # Returns
    ///
    /// A [`wgpu::VertexBufferLayout`] that describes:
    /// - Attribute 0: Position (Float32x3) at shader location 0
    /// - Attribute 1: Normal (Float32x3) at shader location 1
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use haggis::gfx::scene::vertex::Vertex3D;
    ///
    /// let layout = Vertex3D::desc();
    /// // Use layout in render pipeline creation
    /// ```
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex3D>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
