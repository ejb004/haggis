//! # Haggis Quickstart Example
//!
//! This is the "Hello World" example for the Haggis 3D graphics framework.
//! Perfect for beginners who want to understand the core concepts quickly.
//!
//! ## What this example shows:
//! - How to use the new Haggis builder API
//! - How to create materials and 3D objects
//! - How to implement a simple physics simulation
//! - How to add interactive UI controls
//!
//! ## Usage:
//! ```bash
//! cargo run --example quickstart
//! ```
//!
//! ## What you'll see:
//! - Three colorful cubes bouncing around with gravity
//! - Interactive UI controls to adjust physics parameters
//! - Real-time simulation statistics

use haggis::prelude::*;

/// A simple particle with basic physics properties
#[derive(Clone)]
struct Particle {
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    active: bool,
}

/// Simple physics simulation demonstrating the framework basics
struct QuickstartSimulation {
    particles: Vec<Particle>,
    gravity: f32,
    bounce_damping: f32,
    ground_level: f32,
    time: f32,
    running: bool,
}

impl QuickstartSimulation {
    fn new() -> Self {
        Self {
            particles: vec![
                Particle {
                    position: Vector3::new(-2.0, 0.0, 8.0),
                    velocity: Vector3::new(2.0, 1.0, 0.0),
                    active: true,
                },
                Particle {
                    position: Vector3::new(0.0, 0.0, 10.0),
                    velocity: Vector3::new(0.0, 1.5, 0.0),
                    active: true,
                },
                Particle {
                    position: Vector3::new(2.0, -1.0, 6.0),
                    velocity: Vector3::new(-1.0, 0.5, 2.0),
                    active: true,
                },
            ],
            gravity: -9.8,
            bounce_damping: 0.8,
            ground_level: 0.0,
            time: 0.0,
            running: true,
        }
    }

    fn update_physics(&mut self, delta_time: f32) {
        if !self.running {
            return;
        }

        self.time += delta_time;

        for particle in &mut self.particles {
            if !particle.active {
                continue;
            }

            // Apply gravity
            particle.velocity.z += self.gravity * delta_time;

            // Update position
            particle.position += particle.velocity * delta_time;

            // Ground collision (Z-up coordinate system)
            if particle.position.z <= self.ground_level {
                particle.position.z = self.ground_level;
                particle.velocity.z = -particle.velocity.z * self.bounce_damping;

                // Add some friction
                particle.velocity.x *= 0.9;
                particle.velocity.y *= 0.9;
            }

            // Boundary constraints
            let boundary = 10.0;
            if particle.position.x.abs() > boundary {
                particle.position.x = boundary * particle.position.x.signum();
                particle.velocity.x = -particle.velocity.x * self.bounce_damping;
            }
            if particle.position.y.abs() > boundary {
                particle.position.y = boundary * particle.position.y.signum();
                particle.velocity.y = -particle.velocity.y * self.bounce_damping;
            }
        }
    }

    fn sync_to_scene(&self, scene: &mut Scene) {
        for (i, particle) in self.particles.iter().enumerate() {
            if let Some(object) = scene.objects.get_mut(i) {
                if particle.active {
                    object.ui_transform.position = [
                        particle.position.x,
                        particle.position.y,
                        particle.position.z,
                    ];
                    object.ui_transform.rotation[2] = self.time * 45.0;
                    object.apply_ui_transform();
                    object.visible = true;
                } else {
                    object.visible = false;
                }
            }
        }
    }
}

impl Simulation for QuickstartSimulation {
    fn initialize(&mut self, _scene: &mut Scene) {
        // Simulation is already initialized in new()
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        self.update_physics(delta_time);
        self.sync_to_scene(scene);
    }

    fn render_ui(&mut self, ui: &Ui) {
        ui.window("Quickstart Controls")
            .size([300.0, 250.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🚀 Haggis Quickstart");
                ui.separator();

                ui.text(&format!("Time: {:.1}s", self.time));
                ui.text(&format!(
                    "Active Particles: {}",
                    self.particles.iter().filter(|p| p.active).count()
                ));
                ui.spacing();

                ui.text("Physics Settings:");
                ui.slider("Gravity", -20.0, 0.0, &mut self.gravity);
                ui.slider("Bounce Damping", 0.0, 1.0, &mut self.bounce_damping);
                ui.slider("Ground Level", -2.0, 3.0, &mut self.ground_level);
                ui.spacing();

                if ui.button(if self.running {
                    "⏸️ Pause"
                } else {
                    "▶️ Play"
                }) {
                    self.running = !self.running;
                }
                ui.same_line();
                if ui.button("🔄 Reset") {
                    *self = QuickstartSimulation::new();
                }
            });

        ui.window("Framework Info")
            .size([280.0, 180.0], imgui::Condition::FirstUseEver)
            .position([320.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("📚 What's Happening:");
                ui.separator();
                ui.text("• Physics simulation with gravity");
                ui.text("• Collision detection with ground");
                ui.text("• Visual objects follow particles");
                ui.text("• Interactive parameter control");
                ui.spacing();

                ui.text("🎮 Camera Controls:");
                ui.text("• Mouse: Look around");
                ui.text("• Scroll: Zoom in/out");
                ui.text("• Shift+Mouse: Pan view");
            });
    }

    fn name(&self) -> &str {
        "Quickstart Example"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, _scene: &mut Scene) {
        *self = QuickstartSimulation::new();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() -> HaggisResult<()> {
    println!("🎯 Haggis Quickstart Example");
    println!("============================");
    println!("Demonstrating a simple physics simulation with bouncing cubes.");
    println!();

    // Create the application using the standard API
    let mut app = haggis::default();

    // Create materials
    app.app_state
        .scene
        .add_material_rgb("red_metal", 0.9, 0.1, 0.1, 0.8, 0.2);
    app.app_state
        .scene
        .add_material_rgb("green_plastic", 0.1, 0.9, 0.1, 0.1, 0.8);
    app.app_state
        .scene
        .add_material_rgb("blue_ceramic", 0.1, 0.1, 0.9, 0.2, 0.3);
    app.app_state
        .scene
        .add_material_rgb("ground", 0.5, 0.5, 0.5, 0.0, 0.9);

    // Add three cubes with different materials and transforms
    app.add_cube()
        .with_material("red_metal")
        .with_transform([-2.0, 0.0, 8.0], 0.4, 0.0)
        .with_name("particle_1");

    app.add_cube()
        .with_material("green_plastic")
        .with_transform([0.0, 0.0, 10.0], 0.3, 0.0)
        .with_name("particle_2");

    app.add_cube()
        .with_material("blue_ceramic")
        .with_transform([2.0, -1.0, 6.0], 0.5, 0.0)
        .with_name("particle_3");

    // Add a ground plane
    app.add_plane(20.0, 20.0, 10, 10)
        .with_material("ground")
        .with_transform([0.0, 0.0, -1.0], 10.0, 180.0)
        .with_name("ground_plane");

    // Attach the simulation
    app.attach_simulation(QuickstartSimulation::new());

    // Set up UI to show the default transform panel
    app.set_ui(|ui, scene, selected| {
        default_transform_panel(ui, scene, selected);
    });

    app.run();
    Ok(())
}
