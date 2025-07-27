//! Instanced Grid Rendering System
//!
//! High-performance system for rendering large grids of identical objects using GPU instancing.
//! Designed for visualizing 3D cellular automata, voxel grids, particle systems, etc.
//! 
//! Features:
//! - Single draw call for thousands of instances
//! - Direct GPU buffer updates
//! - Configurable instance data (position, scale, color)
//! - Independent of Scene system for maximum performance

use wgpu::{Device, Queue, Buffer, RenderPass, BindGroup, RenderPipeline};
use wgpu::util::DeviceExt;
use cgmath::{Vector3, Vector4};
use bytemuck::{Pod, Zeroable};

use crate::gfx::{
    scene::vertex::Vertex3D,
    resources::global_bindings::GlobalBindings,
};

/// Instance data for a single cube in the grid
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GridInstanceData {
    /// World position [x, y, z, scale]
    pub position_scale: [f32; 4],
    /// Color [r, g, b, a]
    pub color: [f32; 4],
}

impl GridInstanceData {
    pub fn new(position: Vector3<f32>, scale: f32, color: Vector4<f32>) -> Self {
        Self {
            position_scale: [position.x, position.y, position.z, scale],
            color: color.into(),
        }
    }
}

/// Simple unit cube mesh optimized for instancing
pub struct UnitCube {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl UnitCube {
    pub fn new(device: &Device) -> Self {
        // Standard cube vertices - let Haggis handle coordinate system conversion
        let vertices = vec![
            // Front face
            Vertex3D { position: [-0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            Vertex3D { position: [ 0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            Vertex3D { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            Vertex3D { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0] },
            // Back face
            Vertex3D { position: [ 0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            Vertex3D { position: [-0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            Vertex3D { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            Vertex3D { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0] },
            // Left face
            Vertex3D { position: [-0.5, -0.5, -0.5], normal: [-1.0,  0.0,  0.0] },
            Vertex3D { position: [-0.5, -0.5,  0.5], normal: [-1.0,  0.0,  0.0] },
            Vertex3D { position: [-0.5,  0.5,  0.5], normal: [-1.0,  0.0,  0.0] },
            Vertex3D { position: [-0.5,  0.5, -0.5], normal: [-1.0,  0.0,  0.0] },
            // Right face
            Vertex3D { position: [ 0.5, -0.5,  0.5], normal: [ 1.0,  0.0,  0.0] },
            Vertex3D { position: [ 0.5, -0.5, -0.5], normal: [ 1.0,  0.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5, -0.5], normal: [ 1.0,  0.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5,  0.5], normal: [ 1.0,  0.0,  0.0] },
            // Bottom face
            Vertex3D { position: [-0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0] },
            Vertex3D { position: [ 0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0] },
            Vertex3D { position: [ 0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0] },
            Vertex3D { position: [-0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0] },
            // Top face
            Vertex3D { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0] },
            Vertex3D { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0] },
            Vertex3D { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0] },
        ];

        let indices: Vec<u32> = vec![
            // Front face
            0, 1, 2,  2, 3, 0,
            // Back face
            4, 5, 6,  6, 7, 4,
            // Left face
            8, 9, 10,  10, 11, 8,
            // Right face
            12, 13, 14,  14, 15, 12,
            // Bottom face
            16, 17, 18,  18, 19, 16,
            // Top face
            20, 21, 22,  22, 23, 20,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Cube Index Buffer"),
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

/// High-performance instanced grid renderer
pub struct InstancedGrid {
    // Rendering resources
    cube_mesh: UnitCube,
    instance_buffer: Buffer,
    render_pipeline: Option<RenderPipeline>,
    
    // Grid configuration
    max_instances: u32,
    current_instance_count: u32,
    
    // Instance data
    instances: Vec<GridInstanceData>,
    
    // Rendering state
    enabled: bool,
}

impl InstancedGrid {
    /// Create a new instanced grid renderer
    pub fn new(device: &Device, max_instances: u32) -> Self {
        let cube_mesh = UnitCube::new(device);
        
        // Create instance buffer
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Instance Buffer"),
            size: (max_instances as u64) * std::mem::size_of::<GridInstanceData>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            cube_mesh,
            instance_buffer,
            render_pipeline: None,
            max_instances,
            current_instance_count: 0,
            instances: Vec::new(),
            enabled: true,
        }
    }

    /// Initialize rendering pipeline (call this after creating global bindings)
    pub fn initialize_pipeline(&mut self, device: &Device, surface_format: wgpu::TextureFormat, global_bindings: &GlobalBindings) {
        // Create instanced rendering pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Instanced Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(INSTANCED_GRID_SHADER.into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Instanced Grid Pipeline Layout"),
            bind_group_layouts: &[global_bindings.bind_group_layouts()],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Instanced Grid Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex3D::desc(),
                    // Instance buffer layout
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GridInstanceData>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // position_scale
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // color
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Enable alpha blending for transparency
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.render_pipeline = Some(render_pipeline);
    }

    /// Update the grid with new instance data
    pub fn update(&mut self, queue: &Queue, grid_instances: &[(Vector3<f32>, f32, Vector4<f32>)]) {
        // Clear previous instances
        self.instances.clear();
        
        // Convert to instance data
        for &(position, scale, color) in grid_instances.iter().take(self.max_instances as usize) {
            self.instances.push(GridInstanceData::new(position, scale, color));
        }
        
        self.current_instance_count = self.instances.len() as u32;
        
        // Upload to GPU
        if !self.instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&self.instances));
        }
    }

    /// Render the instanced grid
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, global_bind_group: &'a BindGroup) {
        if !self.enabled || self.current_instance_count == 0 {
            return;
        }

        let Some(ref pipeline) = self.render_pipeline else {
            #[cfg(debug_assertions)]
            println!("❌ Instanced grid render pipeline not found!");
            return;
        };

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, global_bind_group, &[]);

        // Set vertex buffers
        render_pass.set_vertex_buffer(0, self.cube_mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.cube_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Single draw call for all instances!
        render_pass.draw_indexed(
            0..self.cube_mesh.index_count,
            0,
            0..self.current_instance_count,
        );
    }

    /// Enable/disable rendering
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get current instance count
    pub fn instance_count(&self) -> u32 {
        self.current_instance_count
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// Instanced grid shader
const INSTANCED_GRID_SHADER: &str = r#"
struct GlobalUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    light_position: vec3<f32>,
    _padding1: f32,
    light_color: vec3<f32>,
    light_intensity: f32,
    light_view_proj: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct InstanceInput {
    @location(2) position_scale: vec4<f32>, // xyz = position, w = scale
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> global: GlobalUniform;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // Create instance model matrix (scale + translation)
    let scale = instance.position_scale.w;
    let translation = instance.position_scale.xyz;
    
    // Build model matrix for this instance (row-major as it was working)
    let model_matrix = mat4x4<f32>(
        scale, 0.0,   0.0,   0.0,
        0.0,   scale, 0.0,   0.0,
        0.0,   0.0,   scale, 0.0,
        translation.x, translation.y, translation.z, 1.0
    );
    
    // Transform vertex to world space using model matrix (like standard Haggis objects)
    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    
    // Transform to clip space using same approach as PBR shader
    let clip_position = global.view_proj * world_position;
    
    // Transform normal using model matrix (3x3 part)
    let normal_matrix = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz, 
        model_matrix[2].xyz
    );
    let world_normal = normalize(normal_matrix * vertex.normal);
    
    return VertexOutput(
        clip_position,
        world_position.xyz,
        world_normal,
        instance.color,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(global.light_position - in.world_position);
    let normal = normalize(in.world_normal);
    
    // Simple lighting
    let ndotl = max(dot(normal, light_dir), 0.2); // Minimum ambient
    
    let lit_color = in.color.rgb * ndotl * global.light_color * global.light_intensity;
    
    return vec4<f32>(lit_color, in.color.a);
}
"#;