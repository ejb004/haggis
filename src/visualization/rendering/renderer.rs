//! Visualization Renderer
//!
//! Dedicated rendering system for visualization components, independent of scene objects.

use wgpu::*;
use wgpu::util::DeviceExt;
use cgmath::{Matrix4, Vector3};
use super::materials::VisualizationMaterial;

/// Vertex data for visualization quads
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

/// Camera uniform for visualization rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VisualizationCameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

/// Visualization item to be rendered
pub struct VisualizationItem {
    pub vertices: Vec<VisualizationVertex>,
    pub indices: Vec<u16>,
    pub material: VisualizationMaterial,
    pub transform: Matrix4<f32>,
}

impl VisualizationItem {
    /// Create a quad for 2D visualization
    pub fn create_quad(
        position: Vector3<f32>,
        size: f32,
        material: VisualizationMaterial,
    ) -> Self {
        let half_size = size * 0.5;
        
        let vertices = vec![
            VisualizationVertex {
                position: [position.x - half_size, position.y - half_size, position.z],
                tex_coords: [0.0, 1.0],
            },
            VisualizationVertex {
                position: [position.x + half_size, position.y - half_size, position.z],
                tex_coords: [1.0, 1.0],
            },
            VisualizationVertex {
                position: [position.x + half_size, position.y + half_size, position.z],
                tex_coords: [1.0, 0.0],
            },
            VisualizationVertex {
                position: [position.x - half_size, position.y + half_size, position.z],
                tex_coords: [0.0, 0.0],
            },
        ];

        let indices = vec![0, 1, 2, 2, 3, 0];

        Self {
            vertices,
            indices,
            material,
            transform: Matrix4::from_translation(position),
        }
    }
}

/// Dedicated renderer for visualization components
pub struct VisualizationRenderer {
    render_pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    vertex_count: u32,
    index_count: u32,
}

impl VisualizationRenderer {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        // Create shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Visualization Shader"),
            source: ShaderSource::Wgsl(super::shaders::VISUALIZATION_SHADER.into()),
        });

        // Create camera buffer
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Visualization Camera Buffer"),
            size: std::mem::size_of::<VisualizationCameraUniform>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Visualization Camera Bind Group Layout"),
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

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Visualization Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Create material bind group layout
        let material_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Visualization Material Bind Group Layout"),
            entries: &[
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
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create render pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Visualization Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &material_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Visualization Render Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                cull_mode: None, // No culling for visualization
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
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            vertex_buffer: None,
            index_buffer: None,
            vertex_count: 0,
            index_count: 0,
        }
    }

    /// Update camera uniform
    pub fn update_camera(&self, queue: &Queue, view_proj_matrix: Matrix4<f32>) {
        let camera_uniform = VisualizationCameraUniform {
            view_proj: view_proj_matrix.into(),
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }

    /// Update vertex and index buffers with visualization items
    pub fn update_buffers(&mut self, device: &Device, items: &[VisualizationItem]) {
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();
        let mut index_offset = 0;

        for item in items {
            all_vertices.extend_from_slice(&item.vertices);
            for &index in &item.indices {
                all_indices.push(index + index_offset);
            }
            index_offset += item.vertices.len() as u16;
        }

        if !all_vertices.is_empty() {
            // Create vertex buffer
            self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Visualization Vertex Buffer"),
                contents: bytemuck::cast_slice(&all_vertices),
                usage: BufferUsages::VERTEX,
            }));
            self.vertex_count = all_vertices.len() as u32;

            // Create index buffer
            self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Visualization Index Buffer"),
                contents: bytemuck::cast_slice(&all_indices),
                usage: BufferUsages::INDEX,
            }));
            self.index_count = all_indices.len() as u32;
        }
    }

    /// Render visualization items
    pub fn render(
        &self,
        encoder: &mut CommandEncoder,
        color_attachment: &TextureView,
        depth_attachment: &TextureView,
        materials: &[&VisualizationMaterial],
    ) {
        if self.vertex_buffer.is_none() || self.index_buffer.is_none() {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Visualization Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color_attachment,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_attachment,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        
        if let (Some(vertex_buffer), Some(index_buffer)) = (&self.vertex_buffer, &self.index_buffer) {
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint16);

            // Render each material group
            for material in materials {
                if let Some(bind_group) = &material.bind_group {
                    render_pass.set_bind_group(1, bind_group, &[]);
                    render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                }
            }
        }
    }
}