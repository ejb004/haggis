//! Global uniform bindings for camera and scene data
//!
//! Manages GPU uniform buffers and bind groups for global rendering state
//! that is shared across all objects in a scene, primarily camera matrices.

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
/// to shaders, currently just camera view/projection matrices.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GlobalUBOContent {
    camera: CameraUniform,
}

unsafe impl bytemuck::Pod for GlobalUBOContent {}
unsafe impl bytemuck::Zeroable for GlobalUBOContent {}

/// Type alias for the global uniform buffer
pub type GlobalUBO = UniformBuffer<GlobalUBOContent>;

/// Updates the global uniform buffer with new camera data
///
/// Should be called each frame with updated camera matrices to ensure
/// correct rendering of all objects in the scene.
///
/// # Arguments
/// * `ubo` - The global uniform buffer to update
/// * `queue` - WGPU command queue for buffer updates
/// * `camera` - Updated camera uniform data
pub fn update_global_ubo(ubo: &mut GlobalUBO, queue: &wgpu::Queue, camera: CameraUniform) {
    ubo.update_content(queue, GlobalUBOContent { camera });
}

/// Manages bind group layouts and bind groups for global uniforms
///
/// Handles the creation and management of WGPU bind groups that contain
/// global scene data like camera matrices. This is bound to slot 0
/// in all render pipelines.
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
            .next_binding_rendering(binding_types::uniform())
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
