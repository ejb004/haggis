//! # Gizmo Traits
//!
//! This module defines the core traits that all gizmo implementations must implement
//! to integrate with the Haggis gizmo system.

use crate::gfx::scene::Scene;
use imgui::Ui;
use std::any::Any;
use wgpu::{Device, Queue};

/// Core trait for gizmo components in the Haggis engine.
///
/// This trait defines the interface that all gizmo implementations must implement
/// to work with the gizmo system. Gizmos are visual aids that can represent
/// positions, orientations, paths, bounds, and other spatial information.
///
/// ## Lifecycle
///
/// The gizmo lifecycle follows this pattern:
/// 1. **Initialize** - Set up gizmo state and create initial scene objects
/// 2. **Update Loop** - Called every frame to update gizmo visualization
/// 3. **UI Rendering** - Render gizmo-specific controls and information
/// 4. **Cleanup** - Clean up resources when gizmo is removed
///
/// ## Examples
///
/// ```no_run
/// use haggis::gfx::gizmos::traits::Gizmo;
/// use haggis::gfx::scene::Scene;
/// use imgui::Ui;
/// use wgpu::{Device, Queue};
/// use cgmath::Vector3;
///
/// struct PositionGizmo {
///     enabled: bool,
///     position: Vector3<f32>,
///     gizmo_id: Option<String>,
/// }
///
/// impl Gizmo for PositionGizmo {
///     fn initialize(&mut self, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>) {
///         self.enabled = true;
///         // Create initial gizmo object
///     }
///
///     fn update(&mut self, delta_time: f32, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>) {
///         if self.enabled {
///             // Update gizmo visualization
///         }
///     }
///
///     fn render_ui(&mut self, ui: &Ui, scene: &mut Scene) {
///         ui.window("Position Gizmo").build(|| {
///             ui.checkbox("Enabled", &mut self.enabled);
///             // Render gizmo controls
///         });
///     }
///
///     fn name(&self) -> &str { "Position Gizmo" }
///     fn is_enabled(&self) -> bool { self.enabled }
///     fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
/// }
/// ```
pub trait Gizmo {
    /// Initialize the gizmo component.
    ///
    /// This method is called once when the gizmo is added to the system.
    /// Use this to set up initial state and create any required scene objects.
    ///
    /// # Arguments
    ///
    /// * `scene` - Mutable reference to the scene for object creation
    /// * `device` - Optional GPU device for resource creation
    /// * `queue` - Optional GPU queue for command submission
    fn initialize(&mut self, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>);

    /// Update the gizmo for the current frame.
    ///
    /// This method is called every frame and should contain the main gizmo logic.
    /// It should update positions, colors, visibility, and other properties based
    /// on the current scene state and elapsed time.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since the last frame in seconds
    /// * `scene` - Mutable reference to the scene for object access
    /// * `device` - Optional GPU device for resource access
    /// * `queue` - Optional GPU queue for command submission
    fn update(&mut self, delta_time: f32, scene: &mut Scene, device: Option<&Device>, queue: Option<&Queue>);

    /// Render the gizmo's user interface controls.
    ///
    /// This method is called every frame to render gizmo-specific UI controls,
    /// settings, and information displays.
    ///
    /// # Arguments
    ///
    /// * `ui` - ImGui UI context for rendering interface elements
    /// * `scene` - Mutable reference to the scene for property access
    fn render_ui(&mut self, ui: &Ui, scene: &mut Scene);

    /// Get the name of the gizmo.
    ///
    /// This name is used for display purposes in the UI and debugging.
    ///
    /// # Returns
    ///
    /// A string slice containing the gizmo name
    fn name(&self) -> &str;

    /// Check if the gizmo is currently enabled.
    ///
    /// # Returns
    ///
    /// `true` if the gizmo is active and rendering, `false` if disabled
    fn is_enabled(&self) -> bool;

    /// Set the enabled state of the gizmo.
    ///
    /// When disabled, gizmos should hide their visual elements but may
    /// continue processing data for when they're re-enabled.
    ///
    /// # Arguments
    ///
    /// * `enabled` - `true` to enable the gizmo, `false` to disable it
    fn set_enabled(&mut self, enabled: bool);

    /// Clean up gizmo resources when removed.
    ///
    /// This method is called when the gizmo is removed from the system.
    /// Override this method to clean up scene objects and resources.
    ///
    /// # Arguments
    ///
    /// * `scene` - Mutable reference to the scene for object removal
    fn cleanup(&mut self, _scene: &mut Scene) {
        // Default: no cleanup needed
    }

    /// Get the priority of this gizmo for rendering order.
    ///
    /// Gizmos with higher priority values are rendered later (on top).
    /// Default priority is 0.
    ///
    /// # Returns
    ///
    /// Priority value for rendering order
    fn get_priority(&self) -> i32 {
        0
    }

    /// Check if this gizmo should be visible in the current context.
    ///
    /// This can be used to implement context-sensitive visibility,
    /// such as only showing certain gizmos when objects are selected.
    ///
    /// # Arguments
    ///
    /// * `scene` - Reference to the current scene state
    ///
    /// # Returns
    ///
    /// `true` if the gizmo should be visible, `false` otherwise
    fn should_be_visible(&self, _scene: &Scene) -> bool {
        self.is_enabled()
    }

    /// Get the preferred UI window position.
    ///
    /// Returns the preferred position for this gizmo's UI window.
    /// Default is top-left corner.
    ///
    /// # Returns
    ///
    /// (x, y) position in screen coordinates
    fn get_ui_position(&self) -> (f32, f32) {
        (20.0, 20.0)
    }

    /// Get the preferred UI window size.
    ///
    /// Returns the preferred size for this gizmo's UI window.
    ///
    /// # Returns
    ///
    /// (width, height) size in screen coordinates
    fn get_ui_size(&self) -> (f32, f32) {
        (300.0, 200.0)
    }

    /// Support for downcasting to concrete types
    fn as_any(&self) -> &dyn Any;

    /// Support for mutable downcasting to concrete types
    fn as_any_mut(&mut self) -> &mut dyn Any;
}