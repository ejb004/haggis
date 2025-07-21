//! Base simulation with visualization support
//!
//! Provides a base implementation for simulations that need visualization components.

use crate::{
    gfx::scene::Scene,
    simulation::traits::Simulation,
    visualization::{manager::VisualizationManager, traits::VisualizationComponent},
};
use imgui::Ui;
use std::any::Any;
use wgpu::{Device, Queue};

/// Base simulation that supports adding visualization components
///
/// This struct provides a foundation for simulations that need to manage
/// visualization components alongside their core simulation logic.
pub struct BaseSimulation {
    name: String,
    visualization_manager: VisualizationManager,
    running: bool,
}

impl BaseSimulation {
    /// Create a new base simulation
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visualization_manager: VisualizationManager::new(),
            running: false,
        }
    }

    /// Add a visualization component to this simulation
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for the visualization component
    /// * `component` - The visualization component to add
    pub fn add_visualization<T: VisualizationComponent + 'static>(
        &mut self,
        name: &str,
        component: T,
    ) {
        self.visualization_manager
            .add_component(name.to_string(), Box::new(component));
    }

    /// Remove a visualization component
    pub fn remove_visualization(&mut self, name: &str) {
        self.visualization_manager.remove_component(name);
    }

    /// Initialize GPU resources for all visualizations
    pub fn initialize_gpu_visualizations(&mut self, device: &Device, queue: &Queue) {
        self.visualization_manager.initialize_gpu(device, queue);
    }

    /// Get visualization planes from this simulation
    pub fn get_visualization_planes(&self) -> Vec<crate::gfx::rendering::VisualizationPlane> {
        self.visualization_manager.get_visualization_planes()
    }

    /// Update all visualization components
    pub fn update_visualizations(
        &mut self,
        delta_time: f32,
        device: Option<&Device>,
        queue: Option<&Queue>,
    ) {
        self.visualization_manager.update(delta_time, device, queue);
    }

    /// Update scene objects for all visualizations
    pub fn update_visualization_scene_objects(&mut self, scene: &mut Scene) {
        self.visualization_manager.update_scene_objects(scene);
    }

    /// Update material textures for all visualizations  
    pub fn update_visualization_material_textures(
        &mut self,
        scene: &mut Scene,
        device: &Device,
        queue: &Queue,
    ) {
        self.visualization_manager
            .update_material_textures(scene, device, queue);
    }

    /// Get a reference to the visualization manager
    pub fn get_visualization_manager(&self) -> &VisualizationManager {
        &self.visualization_manager
    }

    /// Get a mutable reference to the visualization manager
    pub fn get_visualization_manager_mut(&mut self) -> &mut VisualizationManager {
        &mut self.visualization_manager
    }

    /// Update visualization material textures (public method for external calls)
    pub fn update_viz_textures(&mut self, scene: &mut Scene, device: &Device, queue: &Queue) {
        self.update_visualization_material_textures(scene, device, queue);
    }

    /// Render UI for all visualization components
    pub fn render_visualization_ui(&mut self, ui: &Ui) {
        self.visualization_manager.render_ui(ui);
    }

    /// Check if the simulation is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Set the running state
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }
}

impl Simulation for BaseSimulation {
    fn initialize(&mut self, _scene: &mut Scene) {
        self.running = true;
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if self.running {
            // Update visualizations
            self.update_visualizations(delta_time, None, None);
            self.update_visualization_scene_objects(scene);
        }
    }

    fn render_ui(&mut self, ui: &Ui) {
        ui.window(&format!("{} Simulation", self.name)).build(|| {
            ui.checkbox("Running", &mut self.running);
            ui.separator();

            // Render visualization UI
            self.render_visualization_ui(ui);
        });
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, _scene: &mut Scene) {
        self.running = false;
        // Reset visualizations if needed
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.initialize_gpu_visualizations(device, queue);
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, delta_time: f32) {
        self.update_visualizations(delta_time, Some(device), Some(queue));
    }

    fn apply_gpu_results_to_scene(&mut self, _device: &Device, scene: &mut Scene) {
        // Scene object updates
        self.update_visualization_scene_objects(scene);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
