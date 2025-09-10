//! # High-Level Particle System Example
//!
//! Demonstrates beginner-friendly usage with minimal boilerplate.
//! Uses builder pattern and automatic resource management exclusively.

use haggis;
use haggis::simulation::ParticleSystemBuilder;
use haggis::Builder;

fn main() {
    env_logger::init();

    let mut app = haggis::default();
    
    // Create particle system with fluent builder API - no manual resource management
    let particles = ParticleSystemBuilder::new()
        .with_name("Fountain Particles")
        .with_count(500)
        .with_gravity([0.0, 0.0, -9.8])
        .with_uniform_force([1.0, 0.0, 0.0])  // Wind effect
        .with_bounds([-5.0, -5.0, 0.0], [5.0, 5.0, 10.0])
        .with_lifetime(3.0)
        .with_spawn_rate(50.0)
        .with_damping(0.95)
        .build();

    // Attach simulation - framework handles everything
    app.attach_simulation(particles);

    // Simple UI setup
    app.set_ui(|ui, _scene, _selected| {
        ui.window("High-Level Example").build(|| {
            ui.text("Beginner-friendly particle system");
            ui.text("All complexity hidden behind builder pattern");
            ui.separator();
            ui.text("Features:");
            ui.bullet_text("Automatic GPU/CPU selection");
            ui.bullet_text("Memory management handled");
            ui.bullet_text("Sensible defaults provided");
            ui.bullet_text("Fluent builder API");
        });
    });

    app.run();
}