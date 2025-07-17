//! # Simulation Traits
//!
//! This module defines the core traits that all simulations must implement
//! to integrate with the Haggis simulation system.

use crate::gfx::scene::Scene;
use imgui::Ui;
use wgpu::{Device, Queue};

/// Core trait for user-defined simulations in the Haggis engine.
///
/// This trait defines the interface that all simulations must implement to work
/// with the Haggis simulation system. It supports both CPU-only and GPU-accelerated
/// simulations through a unified interface.
///
/// ## Lifecycle
///
/// The simulation lifecycle follows this pattern:
/// 1. **Initialize** - Set up simulation state and resources
/// 2. **Update Loop** - Called every frame to update simulation state
/// 3. **UI Rendering** - Render simulation controls and debug info
/// 4. **Cleanup** - Clean up resources when simulation is detached
///
/// ## GPU Support
///
/// The trait includes optional GPU methods that can be implemented for
/// compute shader-based simulations:
/// - [`initialize_gpu`] - Set up GPU resources
/// - [`update_gpu`] - Run GPU compute shaders
/// - [`apply_gpu_results_to_scene`] - Apply GPU results to scene objects
///
/// ## Examples
///
/// ### CPU Simulation
/// ```no_run
/// use haggis::simulation::traits::Simulation;
/// use haggis::gfx::scene::Scene;
/// use imgui::Ui;
///
/// struct MySimulation {
///     running: bool,
///     time: f32,
/// }
///
/// impl Simulation for MySimulation {
///     fn initialize(&mut self, scene: &mut Scene) {
///         self.running = true;
///         self.time = 0.0;
///     }
///
///     fn update(&mut self, delta_time: f32, scene: &mut Scene) {
///         if self.running {
///             self.time += delta_time;
///             // Update scene objects here
///         }
///     }
///
///     fn render_ui(&mut self, ui: &Ui) {
///         ui.window("My Simulation").build(|| {
///             ui.text(format!("Time: {:.2}", self.time));
///             if ui.button("Reset") {
///                 self.time = 0.0;
///             }
///         });
///     }
///
///     fn name(&self) -> &str { "My Simulation" }
///     fn is_running(&self) -> bool { self.running }
///     fn set_running(&mut self, running: bool) { self.running = running; }
///     fn reset(&mut self, scene: &mut Scene) { self.time = 0.0; }
/// }
/// ```
///
/// ### GPU Simulation
/// ```no_run
/// use haggis::simulation::traits::Simulation;
/// use haggis::gfx::scene::Scene;
/// use imgui::Ui;
/// use wgpu::{Device, Queue};
///
/// struct MyGpuSimulation {
///     running: bool,
///     gpu_ready: bool,
///     // GPU resources would be stored here
/// }
///
/// impl Simulation for MyGpuSimulation {
///     fn initialize(&mut self, scene: &mut Scene) {
///         self.running = true;
///     }
///
///     fn update(&mut self, delta_time: f32, scene: &mut Scene) {
///         // CPU update logic
///     }
///
///     fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
///         // Set up compute pipelines, buffers, etc.
///         self.gpu_ready = true;
///     }
///
///     fn update_gpu(&mut self, device: &Device, queue: &Queue, delta_time: f32) {
///         // Run compute shaders
///     }
///
///     fn apply_gpu_results_to_scene(&mut self, device: &Device, scene: &mut Scene) {
///         // Read GPU results and update scene objects
///     }
///
///     fn is_gpu_ready(&self) -> bool { self.gpu_ready }
///
///     fn render_ui(&mut self, ui: &Ui) {
///         ui.window("GPU Simulation").build(|| {
///             ui.text("GPU simulation running");
///         });
///     }
///
///     fn name(&self) -> &str { "GPU Simulation" }
///     fn is_running(&self) -> bool { self.running }
///     fn set_running(&mut self, running: bool) { self.running = running; }
///     fn reset(&mut self, scene: &mut Scene) { /* Reset simulation */ }
/// }
/// ```
///
/// [`initialize_gpu`]: Simulation::initialize_gpu
/// [`update_gpu`]: Simulation::update_gpu
/// [`apply_gpu_results_to_scene`]: Simulation::apply_gpu_results_to_scene
pub trait Simulation {
    /// Initialize the simulation with the given scene.
    ///
    /// This method is called once when the simulation is attached to the engine.
    /// Use this to set up initial state, configure objects, and prepare resources.
    ///
    /// # Arguments
    ///
    /// * `scene` - Mutable reference to the scene for initial setup
    fn initialize(&mut self, scene: &mut Scene);

    /// Update the simulation state for the current frame.
    ///
    /// This method is called every frame and should contain the main simulation logic.
    /// It should update object positions, velocities, and other properties based on
    /// the elapsed time.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since the last frame in seconds
    /// * `scene` - Mutable reference to the scene for object updates
    fn update(&mut self, delta_time: f32, scene: &mut Scene);

    /// Render the simulation's user interface.
    ///
    /// This method is called every frame to render simulation-specific UI controls,
    /// debug information, and parameter adjustments.
    ///
    /// # Arguments
    ///
    /// * `ui` - ImGui UI context for rendering interface elements
    fn render_ui(&mut self, ui: &Ui);

    /// Get the name of the simulation.
    ///
    /// This name is used for display purposes in the UI and debugging.
    ///
    /// # Returns
    ///
    /// A string slice containing the simulation name
    fn name(&self) -> &str;

    /// Check if the simulation is currently running.
    ///
    /// # Returns
    ///
    /// `true` if the simulation is active and updating, `false` if paused
    fn is_running(&self) -> bool;

    /// Set the running state of the simulation.
    ///
    /// # Arguments
    ///
    /// * `running` - `true` to start/resume the simulation, `false` to pause it
    fn set_running(&mut self, running: bool);

    /// Reset the simulation to its initial state.
    ///
    /// This method should restore all simulation parameters and object states
    /// to their initial values.
    ///
    /// # Arguments
    ///
    /// * `scene` - Mutable reference to the scene for state reset
    fn reset(&mut self, scene: &mut Scene);

    /// Clean up simulation resources when detached.
    ///
    /// This method is called when the simulation is detached from the engine.
    /// Override this method to clean up any resources or restore scene state.
    ///
    /// # Arguments
    ///
    /// * `_scene` - Mutable reference to the scene for cleanup
    fn cleanup(&mut self, _scene: &mut Scene) {}

    /// Initialize GPU resources for compute shader simulations.
    ///
    /// This method is called after the simulation is attached if GPU resources
    /// are available. Override this method to set up compute pipelines, buffers,
    /// and other GPU resources.
    ///
    /// # Arguments
    ///
    /// * `_device` - GPU device for resource creation
    /// * `_queue` - GPU queue for command submission
    fn initialize_gpu(&mut self, _device: &Device, _queue: &Queue) {
        // Default: no GPU initialization needed
    }

    /// Update GPU compute shader simulations.
    ///
    /// This method is called every frame for GPU simulations. Override this
    /// method to dispatch compute shaders and update GPU-side simulation state.
    ///
    /// # Arguments
    ///
    /// * `_device` - GPU device for resource access
    /// * `_queue` - GPU queue for command submission
    /// * `_delta_time` - Time elapsed since the last frame in seconds
    fn update_gpu(&mut self, _device: &Device, _queue: &Queue, _delta_time: f32) {
        // Default: no GPU update needed
    }

    /// Apply GPU simulation results to the scene.
    ///
    /// This method is called after GPU updates to transfer results from GPU
    /// buffers back to scene objects. Override this method to read GPU results
    /// and update object transforms or other properties.
    ///
    /// # Arguments
    ///
    /// * `_device` - GPU device for resource access
    /// * `_scene` - Mutable reference to the scene for updates
    fn apply_gpu_results_to_scene(&mut self, _device: &Device, _scene: &mut Scene) {
        // Default: no GPU results to apply
    }

    /// Check if GPU resources are ready for use.
    ///
    /// # Returns
    ///
    /// `true` if GPU resources are initialized and ready, `false` otherwise
    fn is_gpu_ready(&self) -> bool {
        false // Default: not GPU-ready
    }
}
