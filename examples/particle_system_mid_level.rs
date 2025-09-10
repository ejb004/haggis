//! # Mid-Level Particle System Example
//!
//! Demonstrates intermediate usage with some customization.
//! Mixes builder abstractions with moderate direct control.

use haggis;
use haggis::simulation::{ParticleSystemBuilder, ParticleSystem};
use haggis::simulation::high_level::{ForceField, Constraint};
use haggis::{Builder, ExecutionHint};

struct CustomParticleSimulation {
    particle_system: ParticleSystem,
    time: f32,
    force_strength: f32,
}

impl CustomParticleSimulation {
    fn new() -> Self {
        let particle_system = ParticleSystemBuilder::new()
            .with_name("Custom Particles")
            .with_count(1000)
            .with_execution_hint(ExecutionHint::PreferGpu) // Explicit GPU preference
            .with_gravity([0.0, 0.0, -9.8])
            .with_lifetime(5.0)
            .build();

        Self {
            particle_system,
            time: 0.0,
            force_strength: 2.0,
        }
    }

    fn update_forces(&mut self) {
        // Simulate runtime force modification
        // In a real implementation, we'd need public methods to modify forces
        // For now, we'll just update our internal state
        self.force_strength = 2.0 + (self.time * 0.5).sin() * 1.5;
    }
}

impl haggis::simulation::traits::Simulation for CustomParticleSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        // Initialize the particle system
        self.particle_system.initialize(scene);
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        self.time += delta_time;
        
        // Update force strength dynamically
        self.update_forces();
        
        // Update the underlying particle system
        self.particle_system.update(delta_time, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("Mid-Level Example").build(|| {
            ui.text("Intermediate control with customization");
            ui.separator();
            ui.text(format!("Time: {:.1}s", self.time));
            ui.text(format!("Force Strength: {:.2}", self.force_strength));
            ui.separator();
            ui.text("Features:");
            ui.bullet_text("Custom simulation wrapper");
            ui.bullet_text("Runtime parameter calculation");
            ui.bullet_text("Mixed builder + simulation API");
            ui.bullet_text("Execution hint control");
            
            // Allow runtime parameter adjustment
            if ui.slider("Base Force", 0.0, 5.0, &mut self.force_strength) {
                // Force strength changed via UI
            }
        });
        
        // Also render the particle system's native UI
        self.particle_system.render_ui(ui);
    }

    fn name(&self) -> &str { "Custom Particle Simulation" }
    fn is_running(&self) -> bool { self.particle_system.is_running() }
    fn set_running(&mut self, running: bool) { self.particle_system.set_running(running); }
    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.time = 0.0;
        self.force_strength = 2.0;
        self.particle_system.reset(scene);
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

fn main() {
    env_logger::init();

    let mut app = haggis::default();
    
    // Create custom simulation with moderate control
    let simulation = CustomParticleSimulation::new();
    app.attach_simulation(simulation);

    app.run();
}