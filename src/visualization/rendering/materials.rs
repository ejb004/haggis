//! Visualization Materials
//!
//! Materials specifically for visualization components, separate from scene materials.
//! Supports both traditional texture-based rendering and direct GPU buffer access.

use crate::gfx::resources::texture_resource::TextureResource;
use crate::visualization::cut_plane_2d::BufferFormat;
use crate::visualization::ui::cut_plane_controls::VisualizationMode;
use std::sync::Arc;
use wgpu::*;

/// Material for visualization components
#[derive(Clone)]
pub struct VisualizationMaterial {
    pub texture: Option<TextureResource>,        // For CPU data
    pub data_buffer: Option<Arc<Buffer>>,        // For GPU data
    pub buffer_format: Option<BufferFormat>,     // GPU buffer format
    pub visualization_mode: VisualizationMode,   // How to render the data
    pub bind_group: Option<BindGroup>,
    pub transform_buffer: Option<Buffer>,
    pub filter_uniform_buffer: Option<Buffer>,   // For GPU filter mode
}

impl VisualizationMaterial {
    /// Create a new visualization material from texture (legacy)
    pub fn new(texture: TextureResource) -> Self {
        Self {
            texture: Some(texture),
            data_buffer: None,
            buffer_format: None,
            visualization_mode: VisualizationMode::Heatmap,
            bind_group: None,
            transform_buffer: None,
            filter_uniform_buffer: None,
        }
    }

    /// Create a material from GPU buffer (high-performance path)
    pub fn from_gpu_buffer(
        device: &Device,
        queue: &Queue,
        buffer: Arc<Buffer>,
        format: BufferFormat,
        mode: VisualizationMode,
        label: &str,
    ) -> Self {
        // Create transform buffer
        let _identity_matrix: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let transform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("{} Transform Buffer", label)),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Note: Transform will be updated later via update_transform method

        // Create bind group layout for GPU buffer access - MUST match shader bindings
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&format!("{} GPU Buffer Layout", label)),
            entries: &[
                // Dummy texture binding (binding 0) - required by pipeline but unused
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
                // Dummy sampler binding (binding 1) - required by pipeline but unused
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Transform uniform buffer (binding 2) - matches shader
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
                // GPU data buffer (binding 3) - matches shader storage buffer binding
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
                // Filter uniforms buffer (binding 4) - new filtering configuration
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create dummy texture and sampler for bindings 0 and 1
        let dummy_texture = device.create_texture(&TextureDescriptor {
            label: Some("GPU Material Dummy Texture"),
            size: Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let dummy_texture_view = dummy_texture.create_view(&TextureViewDescriptor::default());
        
        let dummy_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("GPU Material Dummy Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // Create and initialize filter uniform buffer with default sharp filtering
        let filter_uniform_data = [
            0u32,                    // filter_mode: 0 = sharp (default)
            format.width,            // grid_width
            format.height,           // grid_height  
            0u32,                    // padding
        ];
        
        let filter_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("{} Filter Uniform Buffer", label)),
            size: (4 * std::mem::size_of::<u32>()) as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize the buffer with default values
        queue.write_buffer(&filter_uniform_buffer, 0, bytemuck::cast_slice(&filter_uniform_data));

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("{} GPU Buffer Bind Group", label)),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&dummy_texture_view), // Dummy texture
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&dummy_sampler), // Dummy sampler
                },
                BindGroupEntry {
                    binding: 2,
                    resource: transform_buffer.as_entire_binding(), // Transform buffer
                },
                BindGroupEntry {
                    binding: 3,
                    resource: buffer.as_entire_binding(), // GPU data buffer
                },
                BindGroupEntry {
                    binding: 4,
                    resource: filter_uniform_buffer.as_entire_binding(), // Filter uniform buffer
                },
            ],
        });

        Self {
            texture: None,
            data_buffer: Some(buffer),
            buffer_format: Some(format),
            visualization_mode: mode,
            bind_group: Some(bind_group),
            transform_buffer: Some(transform_buffer),
            filter_uniform_buffer: Some(filter_uniform_buffer),
        }
    }

    /// Create the bind group for this material
    pub fn create_bind_group(&mut self, device: &Device, layout: &BindGroupLayout) {
        self.bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("Visualization Material Bind Group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.texture.as_ref().unwrap().view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.texture.as_ref().unwrap().sampler),
                },
            ],
        }));
    }

    /// Create a material from 2D data with configurable filtering
    pub fn from_2d_data_with_filter(
        device: &Device,
        queue: &Queue,
        data: &[f32],
        width: u32,
        height: u32,
        label: &str,
        filter_mode: wgpu::FilterMode,
    ) -> Self {
        // Convert f32 data to RGBA8 with proper row alignment
        let expected_size = (width * height) as usize;
        if data.len() != expected_size {
            eprintln!("Warning: Data size mismatch. Expected {}, got {}", expected_size, data.len());
        }
        
        let rgba_data: Vec<u8> = data
            .iter()
            .take(expected_size) // Ensure we don't exceed expected size
            .flat_map(|&value| {
                let normalized = value.clamp(0.0, 1.0);
                let color_val = (normalized * 255.0) as u8;
                [color_val, color_val, color_val, 255u8] // Grayscale RGBA
            })
            .collect();

        // Verify final data size
        let expected_rgba_size = (width * height * 4) as usize;
        assert_eq!(rgba_data.len(), expected_rgba_size, 
            "RGBA data size mismatch: expected {}, got {}", expected_rgba_size, rgba_data.len());

        let texture =
            TextureResource::create_from_rgba_data_with_filter(device, queue, &rgba_data, width, height, label, filter_mode);

        // Create a dummy storage buffer for the material bind group
        let dummy_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Dummy Storage Buffer"),
            size: 16, // Minimum size
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // Create transform buffer (identity matrix initially)
        let identity_matrix: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let transform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Visualization Transform Buffer"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Write identity matrix to buffer
        queue.write_buffer(
            &transform_buffer,
            0,
            bytemuck::cast_slice(&[identity_matrix]),
        );

        // Create dummy filter uniform buffer for consistency with GPU path
        let dummy_filter_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Dummy Filter Uniform Buffer"),
            size: (4 * std::mem::size_of::<u32>()) as BufferAddress,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create the material bind group layout
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                // Storage buffer binding
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
                // Filter uniform buffer binding (matches GPU path)
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Visualization Material Bind Group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: transform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: dummy_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: dummy_filter_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            texture: Some(texture),
            data_buffer: None,
            buffer_format: None,
            visualization_mode: VisualizationMode::Heatmap,
            bind_group: Some(bind_group),
            transform_buffer: Some(transform_buffer),
            filter_uniform_buffer: Some(dummy_filter_buffer),
        }
    }

    /// Create a material from 2D data with default smooth filtering (backward compatibility)
    pub fn from_2d_data(
        device: &Device,
        queue: &Queue,
        data: &[f32],
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        Self::from_2d_data_with_filter(
            device,
            queue,
            data,
            width,
            height,
            label,
            wgpu::FilterMode::Linear, // Default to smooth for backward compatibility
        )
    }

    /// Create a checkerboard material
    pub fn create_checkerboard(
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        checker_size: u32,
    ) -> Self {
        let mut data = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let checker_x = (x / checker_size) % 2;
                let checker_y = (y / checker_size) % 2;
                let value = if (checker_x + checker_y) % 2 == 0 {
                    1.0
                } else {
                    0.0
                };
                data.push(value);
            }
        }

        Self::from_2d_data(device, queue, &data, width, height, "Checkerboard Material")
    }

    /// Update the filter mode for GPU materials
    pub fn update_filter_mode(&self, queue: &Queue, filter_mode: crate::visualization::ui::cut_plane_controls::FilterMode) {
        if let (Some(filter_buffer), Some(format)) = (&self.filter_uniform_buffer, &self.buffer_format) {
            let filter_mode_value = match filter_mode {
                crate::visualization::ui::cut_plane_controls::FilterMode::Sharp => 0u32,
                crate::visualization::ui::cut_plane_controls::FilterMode::Smooth => 1u32,
            };
            
            let filter_uniform_data = [
                filter_mode_value,   // filter_mode
                format.width,        // grid_width
                format.height,       // grid_height
                0u32,                // padding
            ];
            
            queue.write_buffer(filter_buffer, 0, bytemuck::cast_slice(&filter_uniform_data));
        }
    }

    /// Update the transform matrix for this material
    pub fn update_transform(
        &self,
        queue: &Queue,
        position: cgmath::Vector3<f32>,
        size: cgmath::Vector3<f32>,
    ) {
        if let Some(transform_buffer) = &self.transform_buffer {
            // Create transform matrix exactly like regular objects: T * R * S
            let translation_matrix = cgmath::Matrix4::from_translation(position);
            let rotation_matrix = cgmath::Matrix4::from_angle_y(cgmath::Deg(0.0)); // No rotation for now
            let scale_matrix = cgmath::Matrix4::from_scale(size.x); // Use uniform scale like regular objects
            let model_matrix = translation_matrix * rotation_matrix * scale_matrix;

            // Convert to the format expected by wgsl (column-major)
            let matrix_array: [[f32; 4]; 4] = model_matrix.into();
            queue.write_buffer(transform_buffer, 0, bytemuck::cast_slice(&[matrix_array]));
        }
    }
}
