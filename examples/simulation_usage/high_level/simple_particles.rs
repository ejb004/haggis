//! # Simple Particles Example
//!
//! This example demonstrates a simple particle system simulation using the haggis framework.
//! Perfect for beginners who want to get started quickly with working particle physics.
//!
//! ## Features Demonstrated
//! - Simple particle physics with gravity and bouncing
//! - Visual particles represented by cubes
//! - Basic collision detection with ground and walls
//! - Real-time parameter adjustment
//! - Simple UI integration
//!
//! ## Usage
//! ```bash
//! cargo run --example simple_particles
//! ```

use cgmath::Vector3;
use haggis::gfx::scene::Scene;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use imgui::Ui;

/// Simple particle representation
#[derive(Clone)]
struct Particle {
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    initial_position: Vector3<f32>,
    active: bool,
}

/// High-level particle simulation
struct SimpleParticleSystem {
    particles: Vec<Particle>,
    gravity: Vector3<f32>,
    damping: f32,
    ground_level: f32,
    boundary_size: f32,
    running: bool,
    time: f32,
    spawn_rate: f32,
    last_spawn: f32,
}

impl SimpleParticleSystem {
    fn new() -> Self {
        Self {
            particles: Vec::new(),
            gravity: Vector3::new(0.0, -9.8, 0.0), // Y-up coordinate system
            damping: 0.95,
            ground_level: 0.0,
            boundary_size: 8.0,
            running: true,
            time: 0.0,
            spawn_rate: 2.0,
            last_spawn: 0.0,
        }
    }

    fn spawn_particle(&mut self, index: usize) {
        let angle = (index as f32 * 0.618) * 2.0 * std::f32::consts::PI; // Golden angle
        let radius = 2.0 + (index as f32 * 0.1) % 3.0;

        let position = Vector3::new(
            radius * angle.cos(),
            5.0 + (index as f32 * 0.2) % 3.0, // Y-up: start particles in the air
            radius * angle.sin(),
        );

        let velocity = Vector3::new(
            (angle.cos() * 0.5) + (index as f32 * 0.1) % 2.0 - 1.0,
            1.0 + (index as f32 * 0.05) % 2.0, // Y-up: initial upward velocity
            (angle.sin() * 0.5) + (index as f32 * 0.13) % 2.0 - 1.0,
        );

        if index < self.particles.len() {
            self.particles[index] = Particle {
                position,
                velocity,
                initial_position: position,
                active: true,
            };
        }
    }

    fn update_particles(&mut self, delta_time: f32) {
        for particle in &mut self.particles {
            if !particle.active {
                continue;
            }

            // Apply gravity
            particle.velocity += self.gravity * delta_time;

            // Update position
            particle.position += particle.velocity * delta_time;

            // Ground collision (Y-up: ground is at Y=0)
            if particle.position.y <= self.ground_level {
                particle.position.y = self.ground_level;
                particle.velocity.y = -particle.velocity.y * self.damping;
                particle.velocity.x *= 0.9; // Friction
                particle.velocity.z *= 0.9;
            }

            // Boundary collisions
            if particle.position.x.abs() > self.boundary_size {
                particle.position.x = self.boundary_size * particle.position.x.signum();
                particle.velocity.x = -particle.velocity.x * self.damping;
            }
            if particle.position.z.abs() > self.boundary_size {
                particle.position.z = self.boundary_size * particle.position.z.signum();
                particle.velocity.z = -particle.velocity.z * self.damping;
            }

            // Respawn if too low (Y-up: below ground)
            if particle.position.y < -5.0 {
                particle.active = false;
            }
        }
    }

    fn respawn_inactive_particles(&mut self) {
        let inactive_indices: Vec<usize> = self
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.active)
            .map(|(i, _)| i)
            .collect();

        for i in inactive_indices {
            self.spawn_particle(i);
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
                    object.ui_transform.rotation[1] = self.time * 90.0; // Rotation for visual effect
                    object.apply_ui_transform();
                    object.visible = true;
                } else {
                    object.visible = false;
                }
            }
        }
    }
}

impl Simulation for SimpleParticleSystem {
    fn initialize(&mut self, scene: &mut Scene) {
        println!("Initializing Simple Particle System...");

        // Initialize particles based on the number of objects in the scene
        let particle_count = scene.objects.len().min(50); // Limit to prevent performance issues
        self.particles.clear();

        for i in 0..particle_count {
            self.spawn_particle(i);
        }

        println!("Initialized {} particles", particle_count);
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if !self.running {
            return;
        }

        self.time += delta_time;
        self.last_spawn += delta_time;

        // Update particle physics
        self.update_particles(delta_time);

        // Respawn particles periodically
        if self.last_spawn > self.spawn_rate {
            self.respawn_inactive_particles();
            self.last_spawn = 0.0;
        }

        // Sync particle positions to scene objects
        self.sync_to_scene(scene);
    }

    fn render_ui(&mut self, ui: &Ui) {
        let display_size = ui.io().display_size;
        let panel_width = 400.0;
        let panel_height = 200.0;
        let bottom_margin = 10.0;

        ui.window("Simple Particles")
            .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
            .position(
                [10.0, display_size[1] - panel_height - bottom_margin],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text("High-Level Particle System");
                ui.separator();

                ui.text(&format!(
                    "Active Particles: {}",
                    self.particles.iter().filter(|p| p.active).count()
                ));
                ui.text(&format!("Time: {:.2}s", self.time));
                ui.spacing();

                // Physics controls
                ui.text("Physics Parameters:");
                ui.slider("Gravity Y", -20.0, 0.0, &mut self.gravity.y);
                ui.slider("Damping", 0.7, 1.0, &mut self.damping);
                ui.slider("Ground Level", -2.0, 2.0, &mut self.ground_level);
                ui.slider("Boundary Size", 3.0, 15.0, &mut self.boundary_size);
                ui.spacing();

                // Spawn controls
                ui.text("Spawn Settings:");
                ui.slider("Spawn Rate", 0.5, 5.0, &mut self.spawn_rate);
                ui.spacing();

                // Control buttons
                if ui.button("Respawn All") {
                    self.respawn_inactive_particles();
                }
                ui.same_line();
                if ui.button("Reset") {
                    self.time = 0.0;
                    for i in 0..self.particles.len() {
                        self.spawn_particle(i);
                    }
                }

                ui.separator();
                ui.text("Features:");
                ui.text("✓ Gravity simulation");
                ui.text("✓ Ground collision");
                ui.text("✓ Boundary constraints");
                ui.text("✓ Automatic respawning");
                ui.text("✓ Real-time parameters");
            });
    }

    fn name(&self) -> &str {
        "Simple Particles"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, scene: &mut Scene) {
        self.time = 0.0;
        self.last_spawn = 0.0;
        for i in 0..self.particles.len() {
            self.spawn_particle(i);
        }
        self.sync_to_scene(scene);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the haggis framework
    let mut haggis = haggis::default();

    // Create a simple material for particles
    haggis
        .app_state
        .scene
        .add_material_rgb("particle_blue", 0.2, 0.6, 1.0, 0.8, 0.3);

    // Add visual cubes to represent particles
    for i in 0..25 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("particle_blue")
            .with_name(&format!("particle_{}", i))
            .with_transform([0.0, 0.0, 5.0], 0.1, 0.0);
    }

    // Create and attach the particle simulation
    let particle_sim = SimpleParticleSystem::new();
    haggis.attach_simulation(particle_sim);

    // Set up UI
    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Add a guide panel
        ui.window("Simple Particles Guide")
            .size([350.0, 200.0], imgui::Condition::FirstUseEver)
            // .position(
            //     [
            //         panel_width + 20.0,
            //         display_size[1] - panel_height - bottom_margin,
            //     ],
            //     imgui::Condition::FirstUseEver,
            // )
            .build(|| {
                ui.text("High-Level Particle System");
                ui.separator();
                ui.text("This example demonstrates:");
                ui.text("• Simple particle physics");
                ui.text("• Gravity and collisions");
                ui.text("• Automatic respawning");
                ui.text("• Real-time parameter tuning");
                ui.spacing();
                ui.text("Watch the blue cubes fall and bounce!");
                ui.text("Adjust parameters in the Particles panel.");
            });
    });

    // Run the application
    haggis.run();
    Ok(())
}
