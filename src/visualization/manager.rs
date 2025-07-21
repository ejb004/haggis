//! Visualization manager for the Haggis engine
//!
//! Manages multiple visualization components and integrates them with
//! the main engine loop and UI system.

use super::traits::VisualizationComponent;
use crate::gfx::{scene::Scene, rendering::VisualizationPlane};
use imgui::Ui;
use std::collections::HashMap;
use wgpu::{Device, Queue};

/// Manages visualization components within the Haggis engine
pub struct VisualizationManager {
    components: HashMap<String, Box<dyn VisualizationComponent>>,
    enabled: bool,
}

impl VisualizationManager {
    /// Create a new visualization manager
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            enabled: true,
        }
    }

    /// Add a visualization component to the manager
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the component
    /// * `component` - The visualization component to add
    pub fn add_component(&mut self, name: String, mut component: Box<dyn VisualizationComponent>) {
        // Initialize the component
        component.initialize(None, None);
        self.components.insert(name.clone(), component);
    }

    /// Remove a visualization component
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the component to remove
    pub fn remove_component(&mut self, name: &str) {
        if let Some(mut component) = self.components.remove(name) {
            component.cleanup();
        }
    }

    /// Initialize GPU resources for all components
    pub fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        for component in self.components.values_mut() {
            component.initialize(Some(device), Some(queue));
        }
    }

    /// Update all visualization components
    pub fn update(&mut self, delta_time: f32, device: Option<&Device>, queue: Option<&Queue>) {
        if !self.enabled {
            return;
        }

        for component in self.components.values_mut() {
            if component.is_enabled() {
                component.update(delta_time, device, queue);
            }
        }
    }

    /// Update both visualization components and their material textures
    pub fn update_with_scene(&mut self, delta_time: f32, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>) {
        // Update components first
        self.update(delta_time, device, queue);
        
        // Update scene objects
        self.update_scene_objects(scene);
        
        // Update material textures if device and queue are available
        if let (Some(device), Some(queue)) = (device, queue) {
            self.update_material_textures(scene, device, queue);
        }
    }

    /// Render UI for all visualization components
    pub fn render_ui(&mut self, ui: &Ui) {
        if !self.enabled {
            return;
        }

        // Calculate positions for panels on the right side
        let display_size = ui.io().display_size;
        let mut y_offset = 20.0;
        let panel_width = 400.0;
        let x_position = display_size[0] - panel_width - 20.0;

        for (_name, component) in self.components.iter_mut() {
            if component.is_enabled() {
                // Set position for this component's panel
                let window_name = format!("{} Visualization", component.name());
                
                ui.window(&window_name)
                    .size([panel_width, 300.0], imgui::Condition::FirstUseEver)
                    .position([x_position, y_offset], imgui::Condition::FirstUseEver)
                    .resizable(true)
                    .collapsible(true)
                    .build(|| {
                        component.render_ui(ui);
                    });

                y_offset += 320.0; // Space between panels
            }
        }

        // Master control panel
        self.render_master_panel(ui);
    }

    /// Render master control panel for the visualization system
    fn render_master_panel(&mut self, ui: &Ui) {
        let display_size = ui.io().display_size;
        let panel_width = 250.0;
        let x_position = display_size[0] - panel_width - 20.0;

        ui.window("Visualization Manager")
            .size([panel_width, 200.0], imgui::Condition::FirstUseEver)
            .position([x_position, display_size[1] - 220.0], imgui::Condition::FirstUseEver)
            .resizable(true)
            .collapsible(true)
            .build(|| {
                ui.checkbox("Enable Visualizations", &mut self.enabled);
                ui.separator();

                ui.text("Components:");
                for (name, component) in self.components.iter_mut() {
                    let mut enabled = component.is_enabled();
                    if ui.checkbox(name, &mut enabled) {
                        component.set_enabled(enabled);
                    }
                }
            });
    }

    /// Check if the visualization system is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set the enabled state of the visualization system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get a reference to a specific component
    pub fn get_component(&self, name: &str) -> Option<&dyn VisualizationComponent> {
        self.components.get(name).map(|c| c.as_ref())
    }

    /// Get a mutable reference to a specific component
    pub fn get_component_mut(&mut self, name: &str) -> Option<&mut Box<dyn VisualizationComponent>> {
        self.components.get_mut(name)
    }

    /// Get the names of all components
    pub fn get_component_names(&self) -> Vec<&String> {
        self.components.keys().collect()
    }

    /// Check if any components are enabled
    pub fn has_enabled_components(&self) -> bool {
        self.enabled && self.components.values().any(|c| c.is_enabled())
    }

    /// Update scene objects for all enabled visualization components
    pub fn update_scene_objects(&mut self, scene: &mut Scene) {
        if !self.enabled {
            return;
        }

        for component in self.components.values_mut() {
            if component.is_enabled() {
                component.update_scene_objects(scene);
            }
        }
    }

    /// Update material textures for all enabled visualization components
    pub fn update_material_textures(&mut self, scene: &mut Scene, device: &Device, queue: &Queue) {
        if !self.enabled {
            return;
        }

        for component in self.components.values_mut() {
            if component.is_enabled() {
                component.update_material_texture(scene, device, queue);
            }
        }
    }

    /// Get visualization planes for rendering (bypasses scene objects)
    pub fn get_visualization_planes(&self) -> Vec<VisualizationPlane> {
        let mut planes = Vec::new();
        
        if !self.enabled {
            return planes;
        }
        
        for (_name, component) in &self.components {
            if component.is_enabled() {
                // Try to convert the component to a visualization plane
                if let Some(cut_plane) = component.as_any().downcast_ref::<super::cut_plane_2d::CutPlane2D>() {
                    if let Some(plane) = cut_plane.to_visualization_plane() {
                        planes.push(plane);
                    }
                }
            }
        }
        planes
    }
}