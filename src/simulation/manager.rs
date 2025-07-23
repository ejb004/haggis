//! Simulation manager for the Haggis engine
//!
//! Manages the lifecycle of user simulations and integrates them with
//! the main engine loop.

use super::{base_simulation::BaseSimulation, traits::Simulation};
use crate::gfx::scene::Scene;
use imgui::Ui;
use wgpu::{Device, Queue};

/// Manages user simulations within the Haggis engine
pub struct SimulationManager {
    simulation: Option<Box<dyn Simulation>>,
    is_paused: bool,
    time_scale: f32,
    accumulated_time: f32,
    fixed_timestep: Option<f32>,
}

impl SimulationManager {
    /// Create a new simulation manager
    pub fn new() -> Self {
        Self {
            simulation: None,
            is_paused: false,
            time_scale: 1.0,
            accumulated_time: 0.0,
            fixed_timestep: None,
        }
    }

    /// Attach a user simulation to the engine
    pub fn attach_simulation(&mut self, mut simulation: Box<dyn Simulation>, scene: &mut Scene) {
        // Clean up previous simulation if any
        if let Some(mut old_sim) = self.simulation.take() {
            old_sim.cleanup(scene);
        }

        // Initialize new simulation
        simulation.initialize(scene);
        self.simulation = Some(simulation);
        self.is_paused = false;
    }

    /// Initialize GPU resources for current simulation
    /// Called when device/queue become available (e.g., on WindowEvent::Resumed)
    pub fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        if let Some(simulation) = &mut self.simulation {
            simulation.initialize_gpu(device, queue);
        }
    }

    /// Remove current simulation
    pub fn detach_simulation(&mut self, scene: &mut Scene) {
        if let Some(mut sim) = self.simulation.take() {
            sim.cleanup(scene);
        }
    }

    /// Update simulation (called every frame)
    pub fn update(
        &mut self,
        delta_time: f32,
        scene: &mut Scene,
        device: Option<&Device>,
        queue: Option<&Queue>,
    ) {
        if self.is_paused {
            return;
        }

        if let Some(simulation) = &mut self.simulation {
            let scaled_delta = delta_time * self.time_scale;

            if let Some(fixed_dt) = self.fixed_timestep {
                // Fixed timestep simulation for deterministic results
                self.accumulated_time += scaled_delta;

                while self.accumulated_time >= fixed_dt {
                    simulation.update(fixed_dt, scene);

                    // GPU update if available
                    if let (Some(device), Some(queue)) = (device, queue) {
                        simulation.update_gpu(device, queue, fixed_dt);
                        simulation.apply_gpu_results_to_scene(device, scene);

                        // Material texture updates for visualizations will be handled separately
                    }

                    self.accumulated_time -= fixed_dt;
                }
            } else {
                // Variable timestep
                simulation.update(scaled_delta, scene);

                // GPU update if available
                if let (Some(device), Some(queue)) = (device, queue) {
                    simulation.update_gpu(device, queue, scaled_delta);
                    simulation.apply_gpu_results_to_scene(device, scene);

                    // Material texture updates for visualizations will be handled separately
                }
            }
        }
    }

    /// Render simulation UI controls
    pub fn render_ui(&mut self, ui: &Ui, scene: &mut Scene) {
        let display_size = ui.io().display_size;
        let panel_width = 300.0;
        let panel_x = display_size[0] - panel_width - 20.0; // Position on right side

        if let Some(simulation) = &mut self.simulation {
            // Main simulation controls
            ui.window("Simulation Control")
                .size([panel_width, 200.0], imgui::Condition::FirstUseEver)
                .position([panel_x, 240.0], imgui::Condition::FirstUseEver) // Stack below SimplyMove panel
                .build(|| {
                    ui.text(&format!("Simulation: {}", simulation.name()));

                    // Show GPU status
                    if simulation.is_gpu_ready() {
                        ui.text_colored([0.0, 1.0, 0.0, 1.0], "🔹 GPU Ready");
                    } else {
                        ui.text_colored([0.7, 0.7, 0.7, 1.0], "💻 CPU Only");
                    }

                    ui.separator();

                    // Play/Pause controls
                    if ui.button(if self.is_paused {
                        "▶ Play"
                    } else {
                        "⏸ Pause"
                    }) {
                        self.is_paused = !self.is_paused;
                        simulation.set_running(!self.is_paused);
                    }

                    ui.same_line();
                    if ui.button("⏹ Reset") {
                        simulation.reset(scene);
                    }

                    ui.separator();

                    // Time controls
                    ui.slider("Time Scale", 0.1, 3.0, &mut self.time_scale);

                    let mut use_fixed_timestep = self.fixed_timestep.is_some();
                    if ui.checkbox("Fixed Timestep", &mut use_fixed_timestep) {
                        if use_fixed_timestep && self.fixed_timestep.is_none() {
                            self.fixed_timestep = Some(1.0 / 60.0); // 60 FPS
                        } else if !use_fixed_timestep {
                            self.fixed_timestep = None;
                        }
                    }

                    if let Some(ref mut fixed_dt) = self.fixed_timestep {
                        ui.slider("Fixed DT", 1.0 / 120.0, 1.0 / 30.0, fixed_dt);
                    }
                });

            // Let simulation render its own UI (positioned at top of right side)
            simulation.render_ui(ui);
        } else {
            // No simulation loaded
            ui.window("Simulation Control")
                .size([panel_width, 100.0], imgui::Condition::FirstUseEver)
                .position([panel_x, 20.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("No simulation loaded");
                    ui.text("Use haggis.attach_simulation() to load one");
                });
        }
    }

    /// Get current simulation name
    ///
    /// # Returns
    /// Optional reference to the simulation name
    pub fn current_simulation_name(&self) -> Option<&str> {
        self.simulation.as_ref().map(|s| s.name())
    }

    /// Check if simulation is running
    ///
    /// # Returns
    /// `true` if simulation exists and is not paused
    pub fn is_running(&self) -> bool {
        !self.is_paused && self.simulation.is_some()
    }

    /// Check if simulation is paused
    ///
    /// # Returns
    /// `true` if simulation manager is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    /// Set pause state
    ///
    /// # Arguments
    /// * `paused` - Whether to pause the simulation
    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
        if let Some(simulation) = &mut self.simulation {
            simulation.set_running(!paused);
        }
    }

    /// Enable fixed timestep mode
    ///
    /// # Arguments
    /// * `timestep` - Fixed timestep in seconds, or None for variable timestep
    pub fn set_fixed_timestep(&mut self, timestep: Option<f32>) {
        self.fixed_timestep = timestep;
        self.accumulated_time = 0.0; // Reset accumulator
    }

    /// Get current time scale
    ///
    /// # Returns
    /// Current time scale multiplier
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Set time scale
    ///
    /// # Arguments
    /// * `scale` - Time scale multiplier (1.0 = normal speed)
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0); // Prevent negative time
    }

    /// Check if a simulation is currently attached
    ///
    /// # Returns
    /// `true` if a simulation is attached
    pub fn has_simulation(&self) -> bool {
        self.simulation.is_some()
    }


    /// Get visualization planes from the current simulation
    pub fn get_visualization_planes(&self) -> Vec<crate::gfx::rendering::VisualizationPlane> {
        if let Some(simulation) = &self.simulation {
            // Try to downcast to BaseSimulation to access visualization planes
            if let Some(base_sim) = simulation.as_any().downcast_ref::<BaseSimulation>() {
                return base_sim.get_visualization_planes();
            }
        }
        Vec::new()
    }
}
