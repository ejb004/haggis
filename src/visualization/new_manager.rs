//! New Visualization Manager
//!
//! Manages visualization components using the new rendering architecture
//! Separate from scene objects, preserves simulation data

use std::collections::HashMap;
use wgpu::{Device, Queue};
use imgui::Ui;
use crate::gfx::{scene::Scene, rendering::VisualizationPlane};
use super::{
    traits::VisualizationComponent,
    rendering::{ToVisualizationPlane, collect_visualization_planes},
};

/// Enhanced visualization manager using the new rendering architecture
pub struct NewVisualizationManager {
    components: HashMap<String, Box<dyn VisualizationComponent>>,
    needs_update: bool,
}

impl NewVisualizationManager {
    /// Create a new visualization manager
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            needs_update: false,
        }
    }

    /// Add a visualization component
    pub fn add_component(&mut self, name: String, component: Box<dyn VisualizationComponent>) {
        println!("Adding new-style visualization component: {}", name);
        self.components.insert(name, component);
        self.needs_update = true;
    }

    /// Update all visualization components
    pub fn update(&mut self, delta_time: f32, device: Option<&Device>, queue: Option<&Queue>) {
        for component in self.components.values_mut() {
            component.update(delta_time, device, queue);
        }
    }

    /// Get visualization planes for rendering (bypasses scene objects)
    pub fn get_visualization_planes(&self) -> Vec<VisualizationPlane> {
        let mut planes = Vec::new();
        
        for (name, component) in &self.components {
            // Try to convert the component to a visualization plane
            if let Some(cut_plane) = component.as_any().downcast_ref::<super::cut_plane_2d::CutPlane2D>() {
                if let Some(plane) = cut_plane.to_visualization_plane() {
                    println!("Converting component '{}' to visualization plane", name);
                    planes.push(plane);
                }
            }
        }
        
        planes
    }

    /// Render UI for all components
    pub fn render_ui(&mut self, ui: &Ui) {
        for component in self.components.values_mut() {
            component.render_ui(ui);
        }
    }

    /// Initialize all components
    pub fn initialize_components(&mut self, device: Option<&Device>, queue: Option<&Queue>) {
        for component in self.components.values_mut() {
            component.initialize(device, queue);
        }
    }

    /// Check if there are any enabled components
    pub fn has_enabled_components(&self) -> bool {
        self.components.values().any(|c| c.is_enabled())
    }

    /// Legacy methods for compatibility (these now do nothing since we bypass scene objects)
    pub fn update_scene_objects(&mut self, _scene: &mut Scene) {
        // No-op: We no longer use scene objects for visualization
        println!("Legacy update_scene_objects called - bypassed in new architecture");
    }

    pub fn update_material_textures(&mut self, _scene: &mut Scene, _device: &Device, _queue: &Queue) {
        // No-op: Materials are handled directly by visualization renderer
        println!("Legacy update_material_textures called - bypassed in new architecture");
    }
}