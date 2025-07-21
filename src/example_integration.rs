//! Example Integration of New Visualization Architecture
//!
//! This shows how to use the refactored visualization system that properly
//! separates visualization rendering from scene object rendering.

use crate::{
    gfx::rendering::{RenderEngine, VisualizationPlane},
    visualization::{
        cut_plane_2d::CutPlane2D,
        rendering::{ToVisualizationPlane, VisualizationMaterial},
        new_manager::NewVisualizationManager,
    },
};
use cgmath::Vector3;
use wgpu::{Device, Queue};

/// Example demonstrating the new visualization architecture
pub struct ExampleVisualizationIntegration {
    render_engine: RenderEngine,
    visualization_manager: NewVisualizationManager,
    scene: crate::gfx::scene::Scene,
}

impl ExampleVisualizationIntegration {
    pub fn new(render_engine: RenderEngine) -> Self {
        Self {
            render_engine,
            visualization_manager: NewVisualizationManager::new(),
            scene: crate::gfx::scene::Scene::new(),
        }
    }

    /// Add a 2D data visualization plane 
    pub fn add_data_plane(&mut self, name: &str, data: Vec<f32>, width: u32, height: u32) {
        let mut cut_plane = CutPlane2D::new();
        cut_plane.update_data(data, width, height);
        cut_plane.set_position(Vector3::new(0.0, 0.0, 1.0));
        cut_plane.set_size(2.0);
        
        self.visualization_manager.add_component(
            name.to_string(),
            Box::new(cut_plane)
        );
    }

    /// Render frame using the new architecture
    pub fn render_frame(&mut self) {
        // Update visualization components
        self.visualization_manager.update(
            0.016, // 60 FPS
            Some(self.render_engine.device()),
            Some(self.render_engine.queue()),
        );

        // Get visualization planes (bypasses scene objects entirely)
        let visualization_planes = self.visualization_manager.get_visualization_planes();

        // Render with separated visualization pass
        self.render_engine.render_frame_with_visualizations_and_ui(
            &self.scene,
            &visualization_planes,
            |device, queue, encoder, surface_view| {
                // UI rendering callback
                // ImGui UI would be rendered here
            }
        );
    }

    /// The old problematic way (for comparison)
    #[allow(dead_code)]
    fn old_problematic_render(&mut self) {
        // OLD WAY: Visualization → Scene Objects → Material Reset → Data Lost
        // self.visualization_manager.update_scene_objects(&mut self.scene);
        // self.scene.update_materials(); // ← This resets materials!
        // self.render_engine.render_frame_simple(&self.scene); // ← Default materials
    }

    /// The new correct way
    fn new_correct_render(&mut self) {
        // NEW WAY: Visualization → Direct Rendering → Data Preserved
        let visualization_planes = self.visualization_manager.get_visualization_planes();
        self.render_engine.render_frame_with_visualizations(&self.scene, &visualization_planes);
        // ✅ Simulation data is preserved and rendered correctly
    }
}

/// Key Benefits of the Refactored Architecture:
///
/// 1. **Separation of Concerns**: Visualization rendering is completely separate from scene objects
/// 2. **Data Preservation**: Simulation data is never overwritten by default materials  
/// 3. **Clean Extension**: RenderPassExt provides visualization-specific rendering methods
/// 4. **Performance**: Dedicated visualization pipeline optimized for data rendering
/// 5. **Extensibility**: Easy to add new visualization types without affecting scene rendering

/// Migration Guide:
///
/// **Before (Problematic)**:
/// ```rust
/// // Visualization creates scene objects
/// visualization.update_scene_objects(&mut scene);
/// scene.update_materials(); // ← Resets materials!
/// render_engine.render_frame(&scene);
/// ```
///
/// **After (Fixed)**:
/// ```rust  
/// // Visualization creates dedicated planes
/// let planes = visualization_manager.get_visualization_planes();
/// render_engine.render_frame_with_visualizations(&scene, &planes);
/// ```