//! Render Pass Extensions for Visualization
//!
//! Provides extended rendering capabilities specifically for visualization components,
//! separate from regular scene object rendering.

use wgpu::*;

/// Extension trait for RenderPass to support visualization-specific rendering
pub trait RenderPassExt<'a> {
    /// Set visualization-specific bind groups (separate from scene materials)
    fn set_visualization_bindings(&mut self, material_bind_group: &'a BindGroup, slot: u32);
}

impl<'a> RenderPassExt<'a> for RenderPass<'a> {
    fn set_visualization_bindings(&mut self, material_bind_group: &'a BindGroup, slot: u32) {
        self.set_bind_group(slot, material_bind_group, &[]);
    }
}
