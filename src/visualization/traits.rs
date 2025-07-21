//! # Visualization Traits
//!
//! This module defines the core traits that all visualization components must implement
//! to integrate with the Haggis visualization system.

use crate::gfx::scene::Scene;
use imgui::Ui;
use std::any::Any;
use wgpu::{Device, Queue};

/// Core trait for visualization components in the Haggis engine.
///
/// This trait defines the interface that all visualization components must implement
/// to work with the visualization system. Components can render to textures, immediate
/// mode GUI, or directly to the framebuffer.
///
/// ## Lifecycle
///
/// The visualization component lifecycle follows this pattern:
/// 1. **Initialize** - Set up visualization state and resources
/// 2. **Update Loop** - Called every frame to update visualization data
/// 3. **UI Rendering** - Render visualization controls and display
/// 4. **Cleanup** - Clean up resources when component is removed
///
/// ## Examples
///
/// ```no_run
/// use haggis::visualization::traits::VisualizationComponent;
/// use imgui::Ui;
/// use wgpu::{Device, Queue};
///
/// struct MyVisualization {
///     enabled: bool,
///     data: Vec<f32>,
/// }
///
/// impl VisualizationComponent for MyVisualization {
///     fn initialize(&mut self, device: Option<&Device>, queue: Option<&Queue>) {
///         self.enabled = true;
///         // Initialize GPU resources if needed
///     }
///
///     fn update(&mut self, delta_time: f32, device: Option<&Device>, queue: Option<&Queue>) {
///         if self.enabled {
///             // Update visualization data
///         }
///     }
///
///     fn render_ui(&mut self, ui: &Ui) {
///         ui.window("My Visualization").build(|| {
///             ui.checkbox("Enabled", &mut self.enabled);
///             // Render visualization display here
///         });
///     }
///
///     fn name(&self) -> &str { "My Visualization" }
///     fn is_enabled(&self) -> bool { self.enabled }
///     fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
/// }
/// ```
pub trait VisualizationComponent {
    /// Initialize the visualization component.
    ///
    /// This method is called once when the component is added to the system.
    /// Use this to set up initial state and GPU resources if needed.
    ///
    /// # Arguments
    ///
    /// * `device` - Optional GPU device for resource creation
    /// * `queue` - Optional GPU queue for command submission
    fn initialize(&mut self, device: Option<&Device>, queue: Option<&Queue>);

    /// Update the visualization component for the current frame.
    ///
    /// This method is called every frame and should contain the main visualization logic.
    /// It should update data, textures, and other resources based on the elapsed time.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since the last frame in seconds
    /// * `device` - Optional GPU device for resource access
    /// * `queue` - Optional GPU queue for command submission
    fn update(&mut self, delta_time: f32, device: Option<&Device>, queue: Option<&Queue>);

    /// Render the visualization's user interface.
    ///
    /// This method is called every frame to render visualization-specific UI controls,
    /// the visualization display, and parameter adjustments.
    ///
    /// # Arguments
    ///
    /// * `ui` - ImGui UI context for rendering interface elements
    fn render_ui(&mut self, ui: &Ui);

    /// Get the name of the visualization component.
    ///
    /// This name is used for display purposes in the UI and debugging.
    ///
    /// # Returns
    ///
    /// A string slice containing the component name
    fn name(&self) -> &str;

    /// Check if the visualization component is currently enabled.
    ///
    /// # Returns
    ///
    /// `true` if the component is active and rendering, `false` if disabled
    fn is_enabled(&self) -> bool;

    /// Set the enabled state of the visualization component.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable the component, `false` to disable it
    fn set_enabled(&mut self, enabled: bool);

    /// Clean up visualization resources when removed.
    ///
    /// This method is called when the component is removed from the system.
    /// Override this method to clean up any resources.
    fn cleanup(&mut self) {}

    /// Set 3D data for visualization.
    ///
    /// This method allows external systems to provide 3D data for visualization.
    /// The default implementation does nothing - override for data-driven visualizations.
    ///
    /// # Arguments
    ///
    /// * `_data` - 3D data array in row-major order (z, y, x)
    /// * `_dimensions` - Size of the data array (width, height, depth)
    fn set_data(&mut self, _data: &[f32], _dimensions: (u32, u32, u32)) {
        // Default: no data handling
    }

    /// Get the position for the UI panel.
    ///
    /// Returns the preferred position for this component's UI panel.
    /// Default is right side of screen.
    ///
    /// # Returns
    ///
    /// (x, y) position in screen coordinates
    fn get_ui_position(&self) -> (f32, f32) {
        (20.0, 20.0) // Default to top-left
    }

    /// Get the size for the UI panel.
    ///
    /// Returns the preferred size for this component's UI panel.
    ///
    /// # Returns
    ///
    /// (width, height) size in screen coordinates
    fn get_ui_size(&self) -> (f32, f32) {
        (400.0, 300.0) // Default size
    }

    /// Update scene objects for this visualization.
    ///
    /// This method is called when the visualization needs to add, remove, or modify
    /// 3D objects in the scene. For example, cut planes need to create plane objects.
    ///
    /// # Arguments
    ///
    /// * `scene` - Mutable reference to the scene for object manipulation
    fn update_scene_objects(&mut self, _scene: &mut Scene) {
        // Default: no scene objects to update
    }

    /// Update material textures with visualization data.
    ///
    /// This method is called when GPU resources are available to update material textures
    /// with the current visualization data. For example, cut planes update their texture
    /// to display the current slice data.
    ///
    /// # Arguments
    ///
    /// * `scene` - Mutable reference to the scene for material access
    /// * `device` - GPU device for resource creation
    /// * `queue` - GPU queue for data upload
    fn update_material_texture(&mut self, _scene: &mut Scene, _device: &Device, _queue: &Queue) {
        // Default: no material textures to update
    }

    /// Support for downcasting to concrete types
    fn as_any(&self) -> &dyn Any;

    /// Support for mutable downcasting to concrete types
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
