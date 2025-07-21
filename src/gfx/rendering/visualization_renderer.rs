//! Dedicated Visualization Renderer
//!
//! Handles rendering of visualization planes separately from scene objects,
//! ensuring simulation data is preserved and not overwritten by default materials.

use wgpu::*;
use cgmath::{Matrix4, Vector3};
use super::render_pass_ext::RenderPassExt;
use crate::gfx::camera::camera_utils::CameraUniform;
use crate::visualization::rendering::VisualizationMaterial;

/// Manages visualization-specific rendering separate from scene rendering
pub struct VisualizationRenderer {
    pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    quad_vertex_buffer: Buffer,
    quad_index_buffer: Buffer,
}

/// Represents a visualization plane with its simulation data
pub struct VisualizationPlane {
    pub position: Vector3<f32>,
    pub size: Vector3<f32>,
    pub material: VisualizationMaterial,
    pub data_buffer: Option<Buffer>, // For compute shader data
    pub texture: Option<TextureView>, // For texture-based data
}

impl VisualizationRenderer {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        // Create visualization-specific shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Visualization Shader"),
            source: ShaderSource::Wgsl(include_str!("../../visualization/rendering/shaders/visualization.wgsl").into()),
        });

        // Create camera resources
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Visualization Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Visualization Camera Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Visualization Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Create material bind group layout for visualization data
        let material_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Visualization Material Layout"),
            entries: &[
                // Texture binding
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // Sampler binding
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Transform uniform buffer binding
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Storage buffer for compute data
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create render pipeline
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Visualization Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &material_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Visualization Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VisualizationVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None, // Important: No culling for visualization planes
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create quad geometry for visualization planes
        let (quad_vertex_buffer, quad_index_buffer) = Self::create_quad_geometry(device);

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            quad_vertex_buffer,
            quad_index_buffer,
        }
    }

    /// Render all visualization planes with their simulation data
    pub fn render_visualization_pass(
        &self,
        encoder: &mut CommandEncoder,
        color_view: &TextureView,
        depth_view: &TextureView,
        planes: &[VisualizationPlane],
        queue: &Queue,
    ) {
        if planes.is_empty() {
            return;
        }

        // Update transform matrices for all planes before rendering
        for plane in planes.iter() {
            plane.material.update_transform(queue, plane.position, plane.size);
        }

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Visualization Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // Load existing scene
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load, // Use existing depth
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Set visualization pipeline (NOT scene pipeline)
        render_pass.set_pipeline(&self.pipeline);
        
        // Set camera binding
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

        // Set quad geometry
        render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.quad_index_buffer.slice(..), IndexFormat::Uint16);

        // Render each visualization plane with its simulation data
        for plane in planes.iter() {
            self.render_single_plane(&mut render_pass, plane);
        }
    }

    /// Update camera uniforms for visualization rendering
    pub fn update_camera(&self, queue: &Queue, view_proj_matrix: Matrix4<f32>) {
        let camera_uniform = CameraUniform {
            view_position: [0.0, 0.0, 0.0, 1.0], // Placeholder for view position
            view_proj: view_proj_matrix.into(),
        };
        
        
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }

    /// Render a single visualization plane (private)
    fn render_single_plane<'a>(
        &self,
        render_pass: &mut RenderPass<'a>,
        plane: &'a VisualizationPlane,
    ) {
        // Bind the plane's simulation data material (NOT scene material)
        if let Some(bind_group) = &plane.material.bind_group {
            render_pass.set_visualization_bindings(bind_group, 1);
            
            // Draw the plane geometry
            render_pass.draw_indexed(0..6, 0, 0..1);
        } else {
            println!("Warning: Visualization plane material has no bind group");
        }
    }

    /// Create quad geometry for visualization planes
    fn create_quad_geometry(device: &Device) -> (Buffer, Buffer) {
        use wgpu::util::DeviceExt;

        // Simple quad vertices
        let vertices = [
            VisualizationVertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] },
            VisualizationVertex { position: [ 1.0, -1.0, 0.0], tex_coords: [1.0, 1.0] },
            VisualizationVertex { position: [ 1.0,  1.0, 0.0], tex_coords: [1.0, 0.0] },
            VisualizationVertex { position: [-1.0,  1.0, 0.0], tex_coords: [0.0, 0.0] },
        ];

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Visualization Quad Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Visualization Quad Indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer)
    }
}

/// Vertex structure for visualization planes
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VisualizationVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl VisualizationVertex {
    const ATTRIBUTES: [VertexAttribute; 2] = [
        VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
            shader_location: 1,
            format: VertexFormat::Float32x2,
        },
    ];

    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<VisualizationVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}