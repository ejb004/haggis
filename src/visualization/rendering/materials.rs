//! Visualization Materials
//!
//! Materials specifically for visualization components, separate from scene materials.

use crate::gfx::resources::texture_resource::TextureResource;
use wgpu::*;

/// Material for visualization components
#[derive(Clone)]
pub struct VisualizationMaterial {
    pub texture: TextureResource,
    pub bind_group: Option<BindGroup>,
    pub transform_buffer: Option<Buffer>,
}

impl VisualizationMaterial {
    /// Create a new visualization material
    pub fn new(texture: TextureResource) -> Self {
        Self {
            texture,
            bind_group: None,
            transform_buffer: None,
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
                    resource: BindingResource::TextureView(&self.texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.texture.sampler),
                },
            ],
        }));
    }

    /// Create a material from 2D data
    pub fn from_2d_data(
        device: &Device,
        queue: &Queue,
        data: &[f32],
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        // Convert f32 data to RGBA8
        let rgba_data: Vec<u8> = data
            .iter()
            .flat_map(|&value| {
                let normalized = value.clamp(0.0, 1.0);
                let color_val = (normalized * 255.0) as u8;
                [color_val, color_val, color_val, 255u8] // Grayscale
            })
            .collect();

        let texture =
            TextureResource::create_from_rgba_data(device, queue, &rgba_data, width, height, label);

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
            ],
        });

        Self {
            texture,
            bind_group: Some(bind_group),
            transform_buffer: Some(transform_buffer),
        }
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
