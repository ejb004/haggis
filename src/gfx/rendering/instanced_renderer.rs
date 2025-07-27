//! Instanced rendering system for high-performance batch rendering
//!
//! Provides efficient rendering of large numbers of similar objects using GPU instancing.
//! Designed for use cases like particle systems, Conway's Game of Life visualization,
//! and other scenarios requiring thousands of similar objects.

use wgpu::{Device, Queue, Buffer, RenderPass};
use wgpu::util::DeviceExt;
use cgmath::{Matrix4, Vector3, Vector4};
use bytemuck::{Pod, Zeroable};

use crate::gfx::scene::vertex::Vertex3D;

/// Instance data for a single rendered instance
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceData {
    /// Transform matrix (4x4) for the instance
    pub transform: [[f32; 4]; 4],
    /// Color multiplier (RGBA)
    pub color: [f32; 4],
}

impl InstanceData {
    /// Create new instance data with position, scale, and color
    pub fn new(position: Vector3<f32>, scale: f32, color: Vector4<f32>) -> Self {
        let transform = Matrix4::from_translation(position) * Matrix4::from_scale(scale);
        Self {
            transform: transform.into(),
            color: color.into(),
        }
    }

    /// Create new instance data from a transform matrix and color
    pub fn from_transform(transform: Matrix4<f32>, color: Vector4<f32>) -> Self {
        Self {
            transform: transform.into(),
            color: color.into(),
        }
    }

    /// Get vertex buffer layout for instance data
    pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Transform matrix (4 vec4s)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2, // After position(0) and normal(1)
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Color (vec4)
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Simple cube mesh for instanced rendering
pub struct CubeMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl CubeMesh {
    /// Create a unit cube mesh (1x1x1) centered at origin
    pub fn new(device: &Device) -> Self {
        // Define cube vertices (positions and normals)
        let vertices = vec![
            // Front face
            Vertex3D { position: [-0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            Vertex3D { position: [ 0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            Vertex3D { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            Vertex3D { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            
            // Back face
            Vertex3D { position: [-0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            Vertex3D { position: [ 0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            Vertex3D { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            Vertex3D { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            
            // Left face
            Vertex3D { position: [-0.5, -0.5, -0.5], normal: [-1.0,  0.0,  0.0] },
            Vertex3D { position: [-0.5, -0.5,  0.5], normal: [-1.0,  0.0,  0.0] },
            Vertex3D { position: [-0.5,  0.5,  0.5], normal: [-1.0,  0.0,  0.0] },
            Vertex3D { position: [-0.5,  0.5, -0.5], normal: [-1.0,  0.0,  0.0] },
            
            // Right face
            Vertex3D { position: [ 0.5, -0.5, -0.5], normal: [ 1.0,  0.0,  0.0] },
            Vertex3D { position: [ 0.5, -0.5,  0.5], normal: [ 1.0,  0.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5,  0.5], normal: [ 1.0,  0.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5, -0.5], normal: [ 1.0,  0.0,  0.0] },
            
            // Bottom face
            Vertex3D { position: [-0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0] },
            Vertex3D { position: [ 0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0] },
            Vertex3D { position: [ 0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0] },
            Vertex3D { position: [-0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0] },
            
            // Top face
            Vertex3D { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0] },
            Vertex3D { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0] },
        ];

        // Define cube indices (2 triangles per face)
        let indices: Vec<u32> = vec![
            // Front face
            0, 1, 2,  2, 3, 0,
            // Back face
            4, 6, 5,  6, 4, 7,
            // Left face
            8, 9, 10,  10, 11, 8,
            // Right face
            12, 14, 13,  14, 12, 15,
            // Bottom face
            16, 17, 18,  18, 19, 16,
            // Top face
            20, 22, 21,  22, 20, 23,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instanced Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instanced Cube Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

/// Instanced renderer for efficiently rendering many similar objects
pub struct InstancedRenderer {
    cube_mesh: CubeMesh,
    instance_buffer: Buffer,
    max_instances: u32,
    current_instance_count: u32,
}

impl InstancedRenderer {
    /// Create a new instanced renderer
    pub fn new(device: &Device, max_instances: u32) -> Self {
        let cube_mesh = CubeMesh::new(device);

        // Create instance buffer with maximum capacity
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (max_instances as u64) * std::mem::size_of::<InstanceData>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            cube_mesh,
            instance_buffer,
            max_instances,
            current_instance_count: 0,
        }
    }

    /// Update instance data
    pub fn update_instances(&mut self, queue: &Queue, instances: &[InstanceData]) {
        self.current_instance_count = instances.len().min(self.max_instances as usize) as u32;
        
        if self.current_instance_count > 0 {
            let data_slice = &instances[0..self.current_instance_count as usize];
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(data_slice));
        }
    }

    /// Render all instances in a single draw call
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.current_instance_count == 0 {
            return;
        }

        // Set vertex buffers (mesh + instances)
        render_pass.set_vertex_buffer(0, self.cube_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.cube_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Single draw call for all instances
        render_pass.draw_indexed(
            0..self.cube_mesh.index_count,
            0,
            0..self.current_instance_count,
        );
    }

    /// Get current instance count
    pub fn instance_count(&self) -> u32 {
        self.current_instance_count
    }
}