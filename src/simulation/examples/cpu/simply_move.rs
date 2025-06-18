// src/simulation/examples/cpu/simply_move.rs
//! Simple CPU simulation that moves all objects up and down
//!
//! A basic example simulation that demonstrates how to update object positions
//! over time. All objects in the scene will oscillate vertically in a sine wave pattern.

use crate::{gfx::scene::Scene, simulation::traits::Simulation};
use imgui::Ui;

/// Simple simulation that moves all objects up and down in a sine wave
pub struct SimplyMove {
    running: bool,
    time: f32,
    amplitude: f32,
    frequency: f32,
    initial_positions: Vec<f32>, // Store original Y positions
}

impl SimplyMove {
    /// Create a new SimplyMove simulation
    pub fn new() -> Self {
        Self {
            running: true, // Start running automatically
            time: 0.0,
            amplitude: 2.0, // How far up/down to move
            frequency: 0.1, // How fast to oscillate
            initial_positions: Vec::new(),
        }
    }
}

impl Simulation for SimplyMove {
    fn initialize(&mut self, scene: &mut Scene) {
        println!("Initializing SimplyMove simulation...");

        // Store initial Y positions of all objects
        self.initial_positions.clear();
        for object in &scene.objects {
            self.initial_positions.push(object.ui_transform.position[1]);
        }
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if !self.running {
            return;
        }

        // Update time
        self.time += delta_time;

        // Calculate vertical offset using sine wave
        let y_offset =
            (self.time * self.frequency * 2.0 * std::f32::consts::PI).sin() * self.amplitude;

        let angle = (self.time) * 45.0;

        // Apply offset to all objects (except those starting with _)
        for (i, object) in scene.objects.iter_mut().enumerate() {
            // Skip objects whose name starts with _
            if object.name.starts_with('_') {
                continue;
            }

            if let Some(initial_y) = self.initial_positions.get(i) {
                object.ui_transform.position[1] = initial_y + y_offset;
                object.ui_transform.rotation[1] = angle;
                object.apply_ui_transform();
            }
        }
    }

    fn render_ui(&mut self, ui: &Ui) {
        let display_size = ui.io().display_size;
        let panel_width = 300.0;
        let panel_x = display_size[0] - panel_width - 20.0; // Position on right side

        ui.window("SimplyMove Controls")
            .size([panel_width, 220.0], imgui::Condition::FirstUseEver)
            .position([panel_x, 20.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Move + Rotate + Scale");
                ui.separator();

                // Movement parameters
                ui.slider("Amplitude", 0.1, 5.0, &mut self.amplitude);
                ui.slider("Frequency", 0.1, 3.0, &mut self.frequency);

                ui.separator();

                // Status display
                ui.text(&format!("Time: {:.2}s", self.time));
                ui.text(&format!("Objects: {}", self.initial_positions.len()));

                let y_offset = (self.time * self.frequency * 2.0 * std::f32::consts::PI).sin()
                    * self.amplitude;
                let rotation_angle = self.time * self.frequency * 1.5 * 2.0 * std::f32::consts::PI;
                let scale_offset = 1.0
                    + (self.time * self.frequency * 0.8 * 2.0 * std::f32::consts::PI).sin() * 0.2;

                ui.text(&format!("Y Offset: {:.2}", y_offset));
                ui.text(&format!(
                    "Rotation: {:.1}°",
                    rotation_angle.to_degrees() % 360.0
                ));
                ui.text(&format!("Scale: {:.2}", scale_offset));

                ui.separator();
                ui.text("Objects starting with _ are ignored");

                ui.separator();

                // Reset button
                if ui.button("Reset Time") {
                    self.time = 0.0;
                }
            });
    }

    fn name(&self) -> &str {
        "SimplyMove"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, scene: &mut Scene) {
        self.time = 0.0;

        // Reset all objects to their initial positions
        for (i, object) in scene.objects.iter_mut().enumerate() {
            if let Some(initial_y) = self.initial_positions.get(i) {
                object.ui_transform.position[1] = *initial_y;
                object.apply_ui_transform();
            }
        }
    }

    fn cleanup(&mut self, scene: &mut Scene) {
        // Reset objects to initial positions when simulation is removed
        self.reset(scene);
    }
}
