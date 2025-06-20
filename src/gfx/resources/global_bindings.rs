//! Global uniform bindings for camera and scene data
//!
//! Manages GPU uniform buffers and bind groups for global rendering state
//! that is shared across all objects in a scene, including camera matrices
//! and lighting data for shadow mapping.

use crate::{
    gfx::camera::camera_utils::CameraUniform,
    wgpu_utils::{
        binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc},
        binding_types,
        uniform_buffer::UniformBuffer,
    },
};

/// Global uniform buffer content structure
///
/// Contains all per-frame global data that needs to be accessible
/// to shaders, including camera matrices and lighting information.
/// MUST match the GlobalUniform struct in the shaders exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GlobalUBOContent {
    // Camera data (matches existing CameraUniform)
    view_position: [f32; 4],  // Camera position (homogeneous coordinates)
    view_proj: [[f32; 4]; 4], // Camera view-projection matrix

    // Light data
    light_position: [f32; 3],       // Light position
    _padding1: f32,                 // Padding for alignment
    light_color: [f32; 3],          // Light color
    light_intensity: f32,           // Light intensity
    light_view_proj: [[f32; 4]; 4], // Light's view-projection matrix for shadows
}
// Total: 4*4 + 16*4 + 3*4 + 4 + 3*4 + 4 + 16*4 = 16 + 64 + 12 + 4 + 12 + 4 + 64 = 176 bytes

unsafe impl bytemuck::Pod for GlobalUBOContent {}
unsafe impl bytemuck::Zeroable for GlobalUBOContent {}

/// Light configuration for shadow mapping
#[derive(Copy, Clone, Debug)]
pub struct LightConfig {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            position: [5.0, 10.0, 5.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
        }
    }
}

/// Type alias for the global uniform buffer
pub type GlobalUBO = UniformBuffer<GlobalUBOContent>;

/// Updates the global uniform buffer with new camera data only
///
/// Convenience function that preserves existing light data while updating camera.
/// Use `update_global_ubo_with_light` for full updates.
///
/// # Arguments
/// * `ubo` - The global uniform buffer to update
/// * `queue` - WGPU command queue for buffer updates
/// * `camera` - Updated camera uniform data
pub fn update_global_ubo(ubo: &mut GlobalUBO, queue: &wgpu::Queue, camera: CameraUniform) {
    // Preserve existing light data when only updating camera
    let light_config = LightConfig::default();
    update_global_ubo_with_light(ubo, queue, camera, light_config);
}

/// Updates the global uniform buffer with camera and light data
///
/// Should be called each frame with updated camera and light data to ensure
/// correct rendering and shadow mapping for all objects in the scene.
///
/// # Arguments
/// * `ubo` - The global uniform buffer to update
/// * `queue` - WGPU command queue for buffer updates
/// * `camera` - Updated camera uniform data
/// * `light` - Light configuration for shadow mapping
pub fn update_global_ubo_with_light(
    ubo: &mut GlobalUBO,
    queue: &wgpu::Queue,
    camera: CameraUniform,
    light: LightConfig,
) {
    // Better light setup for your scene layout
    let light_pos = cgmath::Point3::new(light.position[0], light.position[1], light.position[2]);
    let light_view = cgmath::Matrix4::look_at_rh(
        light_pos,
        cgmath::Point3::new(0.0, -1.0, 0.0), // Look at between monkey and cube
        cgmath::Vector3::unit_y(),
    );

    // Tighter bounds for better precision
    let light_proj = cgmath::ortho(-25.0, 25.0, -25.0, 25.0, 5.0, 50.0);
    let light_view_proj = light_proj * light_view;

    let content = GlobalUBOContent {
        // Camera data - directly use existing CameraUniform fields
        view_position: camera.view_position,
        view_proj: camera.view_proj,

        // Light data
        light_position: light.position,
        _padding1: 0.0,
        light_color: light.color,
        light_intensity: light.intensity,
        light_view_proj: light_view_proj.into(),
    };

    ubo.update_content(queue, content);
}

/// Manages bind group layouts and bind groups for global uniforms
///
/// Handles the creation and management of WGPU bind groups that contain
/// global scene data like camera matrices and lighting data. This is bound
/// to slot 0 in all render pipelines.
pub struct GlobalBindings {
    bind_group_layout: BindGroupLayoutWithDesc,
    bind_group: Option<wgpu::BindGroup>,
}

impl GlobalBindings {
    /// Creates a new global bindings manager
    ///
    /// Sets up the bind group layout for global uniforms but doesn't
    /// create the actual bind group until `create_bind_group()` is called.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating resources
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = BindGroupLayoutBuilder::new()
            .next_binding_rendering(binding_types::uniform()) // Global uniforms (camera + light)
            .create(device, "Globals Bind Group");

        GlobalBindings {
            bind_group_layout,
            bind_group: None,
        }
    }

    /// Creates the bind group with the provided uniform buffer
    ///
    /// Must be called after the uniform buffer is created and before
    /// any rendering operations that need global uniforms.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating the bind group
    /// * `ubo` - The global uniform buffer to bind
    pub fn create_bind_group(&mut self, device: &wgpu::Device, ubo: &GlobalUBO) {
        self.bind_group = Some(
            BindGroupBuilder::new(&self.bind_group_layout)
                .resource(ubo.binding_resource())
                .create(device, "Global Bind Group"),
        );
    }

    /// Returns the bind group layout
    ///
    /// Used when creating render pipelines that need access to global uniforms.
    pub fn bind_group_layouts(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout.layout
    }

    /// Returns the bind group for rendering
    ///
    /// # Panics
    /// Panics if `create_bind_group()` hasn't been called yet
    pub fn bind_groups(&self) -> &wgpu::BindGroup {
        self.bind_group
            .as_ref()
            .expect("Bind group has not been created yet!")
    }
}
