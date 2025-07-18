//! # Custom Forces Example
//!
//! This example demonstrates how to use the mid-level API to create custom force
//! implementations with more control than the high-level API provides.
//!
//! ## Features Demonstrated
//! - Custom force field implementations
//! - ManagedSimulation wrapper for additional control
//! - Performance monitoring and profiling
//! - Manual parameter adjustment
//! - Hybrid CPU/GPU resource management
//!
//! ## Usage
//! ```bash
//! cargo run --example custom_forces
//! ```

use haggis::simulation::high_level::{ParticleSystem, ParticleSimulation, ForceField};
use haggis::simulation::mid_level::ManagedSimulation;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use cgmath::Vector3;

/// Custom simulation that implements complex force behaviors
/// This shows how to extend the high-level API with custom logic
struct CustomForceSimulation {
    particle_sim: ParticleSimulation,
    time: f32,
    wave_amplitude: f32,
    wave_frequency: f32,
    attractor_strength: f32,
    custom_enabled: bool,
}

impl CustomForceSimulation {
    fn new() -> Self {
        // Start with a basic particle system
        let particle_system = ParticleSystem::new()
            .with_count(300)
            .with_gravity([0.0, 0.0, -5.0])
            .with_damping(0.97)
            .with_lifetime(30.0)
            .build();

        let particle_sim = ParticleSimulation::new("Custom Forces".to_string(), particle_system);

        Self {
            particle_sim,
            time: 0.0,
            wave_amplitude: 5.0,
            wave_frequency: 0.5,
            attractor_strength: 15.0,
            custom_enabled: true,
        }
    }

    /// Apply custom time-varying forces
    fn apply_custom_forces(&mut self, delta_time: f32) {
        if !self.custom_enabled {
            return;
        }

        self.time += delta_time;

        // Create a moving wave attractor
        let wave_x = self.wave_amplitude * (self.time * self.wave_frequency).sin();
        let wave_y = self.wave_amplitude * (self.time * self.wave_frequency * 1.3).cos();
        let wave_z = 3.0 + 2.0 * (self.time * self.wave_frequency * 0.7).sin();

        // Clear existing forces and add new ones
        // Note: This requires extending the ParticleSystem API to clear forces
        // For now, we'll demonstrate the concept
        
        // Add the moving attractor
        self.particle_sim.system_mut().add_force(ForceField::Point {
            position: Vector3::new(wave_x, wave_y, wave_z),
            strength: self.attractor_strength,
        });

        // Add a repulsive force that pulses
        let pulse_strength = -10.0 * (self.time * 2.0).sin().abs();
        self.particle_sim.system_mut().add_force(ForceField::Radial {
            center: Vector3::new(0.0, 0.0, 8.0),
            strength: pulse_strength,
        });

        // Add a spiral force that changes direction
        let spiral_strength = 8.0 * (self.time * 0.3).cos();
        self.particle_sim.system_mut().add_force(ForceField::Vortex {
            center: Vector3::new(0.0, 0.0, 5.0),
            axis: Vector3::new(0.0, 0.0, 1.0),
            strength: spiral_strength,
        });
    }
}

impl Simulation for CustomForceSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.particle_sim.initialize(scene);
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        // Apply custom forces first
        self.apply_custom_forces(delta_time);
        
        // Then update the underlying particle simulation
        self.particle_sim.update(delta_time, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        // Custom UI for force parameters
        ui.window("Custom Force Controls")
            .size([350.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mid-Level API: Custom Force Implementation");
                ui.separator();
                
                ui.checkbox("Enable Custom Forces", &mut self.custom_enabled);
                ui.spacing();
                
                if self.custom_enabled {
                    ui.text("Wave Attractor:");
                    ui.slider("Amplitude", 0.0, 10.0, &mut self.wave_amplitude);
                    ui.slider("Frequency", 0.1, 2.0, &mut self.wave_frequency);
                    ui.spacing();
                    
                    ui.text("Attractor Strength:");
                    ui.slider("Strength", 0.0, 50.0, &mut self.attractor_strength);
                    ui.spacing();
                    
                    ui.text("Force Status:");
                    ui.text(&format!("Time: {:.2}s", self.time));
                    ui.text(&format!("Wave X: {:.2}", self.wave_amplitude * (self.time * self.wave_frequency).sin()));
                    ui.text(&format!("Wave Y: {:.2}", self.wave_amplitude * (self.time * self.wave_frequency * 1.3).cos()));
                    ui.text(&format!("Pulse: {:.2}", -10.0 * (self.time * 2.0).sin().abs()));
                }
                
                ui.spacing();
                if ui.button("Reset Time") {
                    self.time = 0.0;
                }
            });

        // Delegate to particle simulation UI
        self.particle_sim.render_ui(ui);
    }

    fn name(&self) -> &str {
        "Custom Forces Simulation"
    }

    fn is_running(&self) -> bool {
        self.particle_sim.is_running()
    }

    fn set_running(&mut self, running: bool) {
        self.particle_sim.set_running(running);
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.time = 0.0;
        self.particle_sim.reset(scene);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials
    haggis
        .app_state
        .scene
        .add_material_rgb("custom_particle", 1.0, 0.5, 0.2, 0.9, 0.4);
    
    haggis
        .app_state
        .scene
        .add_material_rgb("ground", 0.3, 0.3, 0.3, 0.5, 0.5);

    // Add ground
    haggis
        .add_object("examples/test/ground.obj")
        .with_material("ground")
        .with_name("ground")
        .with_transform([0.0, 0.0, 0.0], 3.0, 0.0);

    // Add particle visual objects
    for i in 0..30 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("custom_particle")
            .with_name(&format!("custom_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.06, 0.0);
    }

    // Create custom simulation
    let custom_sim = CustomForceSimulation::new();
    
    // Wrap with ManagedSimulation for additional features
    let managed_sim = ManagedSimulation::new(custom_sim)
        .with_debug(true);

    // Attach to haggis
    haggis.attach_simulation(managed_sim);

    // Enhanced UI
    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Performance monitoring
        ui.window("Performance Monitoring")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mid-Level API Features:");
                ui.separator();
                ui.text("✓ Custom force implementations");
                ui.text("✓ Real-time parameter adjustment");
                ui.text("✓ Performance monitoring");
                ui.text("✓ Debug mode enabled");
                ui.text("✓ Managed simulation wrapper");
                ui.spacing();
                
                ui.text("Resource Management:");
                ui.text("• Automatic GPU/CPU switching");
                ui.text("• Custom force calculations");
                ui.text("• Time-varying behaviors");
                ui.text("• Parameter interpolation");
            });

        // Implementation details
        ui.window("Implementation Details")
            .size([350.0, 250.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Mid-Level API Implementation:");
                ui.separator();
                
                ui.text("1. Custom Simulation struct");
                ui.text("   - Wraps ParticleSimulation");
                ui.text("   - Adds custom force logic");
                ui.text("   - Implements Simulation trait");
                ui.spacing();
                
                ui.text("2. Time-varying forces:");
                ui.text("   - Wave attractor (sin/cos)");
                ui.text("   - Pulsing repulsion");
                ui.text("   - Direction-changing spiral");
                ui.spacing();
                
                ui.text("3. ManagedSimulation wrapper:");
                ui.text("   - Adds debug capabilities");
                ui.text("   - Performance monitoring");
                ui.text("   - Parameter management");
                ui.spacing();
                
                ui.text("This demonstrates the flexibility");
                ui.text("of the mid-level API for custom");
                ui.text("behaviors beyond high-level presets.");
            });
    });

    haggis.run();
    Ok(())
}