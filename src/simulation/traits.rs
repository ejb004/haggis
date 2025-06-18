//! Core simulation traits for the Haggis engine
//!
//! Defines the interface that user simulations must implement to integrate
//! with the Haggis rendering and UI systems.

use crate::gfx::scene::Scene;
use imgui::Ui;

/// Core trait for user-defined simulations
///
/// This trait defines the lifecycle methods that Haggis will call to run
/// the simulation. Users implement this trait to define their simulation logic.
pub trait Simulation {
    /// Initialize the simulation
    ///
    /// Called once when the simulation is first attached to Haggis.
    /// Use this to set up initial state, load data, create objects, etc.
    ///
    /// # Arguments
    /// * `scene` - Mutable reference to the scene for adding objects
    fn initialize(&mut self, scene: &mut Scene);

    /// Update simulation state
    ///
    /// Called every frame to advance the simulation by one time step.
    /// This is where the main simulation logic goes.
    ///
    /// # Arguments
    /// * `delta_time` - Time elapsed since last update in seconds
    /// * `scene` - Mutable reference to scene for updating object positions/properties
    fn update(&mut self, delta_time: f32, scene: &mut Scene);

    /// Render custom UI controls
    ///
    /// Called during UI rendering to allow simulations to add their own
    /// control panels, parameter sliders, visualization controls, etc.
    ///
    /// # Arguments
    /// * `ui` - ImGui UI context for building interface elements
    fn render_ui(&mut self, ui: &Ui);

    /// Get simulation name for UI display
    fn name(&self) -> &str;

    /// Whether simulation is currently running
    fn is_running(&self) -> bool;

    /// Start/pause simulation
    fn set_running(&mut self, running: bool);

    /// Reset simulation to initial state
    fn reset(&mut self, scene: &mut Scene);

    /// Optional: Custom cleanup when simulation is removed
    fn cleanup(&mut self, _scene: &mut Scene) {
        // Default: no cleanup needed
    }
}
