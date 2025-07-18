//! # Physics Demo Example
//!
//! This example demonstrates a more complex physics simulation with multiple force types.
//! It shows bouncing balls with gravity, collisions, and different particle behaviors.
//!
//! ## Features Demonstrated
//! - Multiple force types (gravity, wind, point attractors)
//! - Boundary constraints (box, spherical, ground)
//! - Real-time parameter adjustment through UI
//! - Multiple particle groups with different behaviors
//!
//! ## Usage
//! ```bash
//! cargo run --example physics_demo
//! ```

use cgmath::{InnerSpace, Vector3};
use haggis::gfx::scene::Scene;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use imgui::Ui;

/// Physics particle representation
#[derive(Clone)]
struct PhysicsParticle {
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    acceleration: Vector3<f32>,
    mass: f32,
    particle_type: ParticleType,
    active: bool,
    lifetime: f32,
    max_lifetime: f32,
}

#[derive(Clone, Copy)]
enum ParticleType {
    Bouncy,
    Floaty,
    Heavy,
}

/// Multi-physics simulation system
struct PhysicsDemo {
    particles: Vec<PhysicsParticle>,
    time: f32,
    running: bool,

    // Force parameters
    gravity: Vector3<f32>,
    wind_force: Vector3<f32>,
    attractor_position: Vector3<f32>,
    attractor_strength: f32,

    // Boundary parameters
    boundary_size: f32,
    ground_level: f32,
    bounce_damping: f32,

    // Simulation parameters
    spawn_rate: f32,
    last_spawn: f32,
    bouncy_count: usize,
    floaty_count: usize,
    heavy_count: usize,
}

impl PhysicsDemo {
    fn new() -> Self {
        Self {
            particles: Vec::new(),
            time: 0.0,
            running: true,
            gravity: Vector3::new(0.0, -9.8, 0.0), // Y-up coordinate system
            wind_force: Vector3::new(1.0, 0.0, 0.0), // Reduced wind force
            attractor_position: Vector3::new(0.0, 6.0, 0.0), // Y-up positioning
            attractor_strength: 8.0,               // Reduced attractor strength
            boundary_size: 7.0,
            ground_level: 0.0,
            bounce_damping: 0.85,
            spawn_rate: 1.5, // Slower spawn rate
            last_spawn: 0.0,
            bouncy_count: 12, // Reduced particle counts
            floaty_count: 8,
            heavy_count: 6,
        }
    }

    fn spawn_particle(&mut self, index: usize, particle_type: ParticleType) {
        let (pos, vel, mass, lifetime) = match particle_type {
            ParticleType::Bouncy => {
                // Spawn bouncy particles in a circle pattern
                let angle = (index as f32 * 0.618) * 2.0 * std::f32::consts::PI;
                let pos = Vector3::new(
                    2.0 * angle.cos(),
                    8.0 + (index as f32 * 0.1) % 3.0, // Y-up: start in air
                    2.0 * angle.sin(),
                );
                let vel = Vector3::new(
                    (index as f32 * 0.1) % 2.0 - 1.0, // Reduced velocity range
                    1.0,
                    (index as f32 * 0.13) % 2.0 - 1.0,
                );
                (pos, vel, 1.0, 15.0)
            }
            ParticleType::Floaty => {
                // Spawn floaty particles higher up
                let angle = (index as f32 * 0.414) * 2.0 * std::f32::consts::PI;
                let pos = Vector3::new(
                    3.0 * angle.cos(),
                    10.0 + (index as f32 * 0.2) % 2.0, // Y-up: start higher
                    3.0 * angle.sin(),
                );
                let vel = Vector3::new(
                    (index as f32 * 0.07) % 1.0 - 0.5, // Reduced velocity
                    0.5,
                    (index as f32 * 0.11) % 1.0 - 0.5,
                );
                (pos, vel, 0.3, 25.0)
            }
            ParticleType::Heavy => {
                // Spawn heavy particles in a line
                let pos = Vector3::new(
                    (index as f32 - 3.0) * 1.0, // Reduced spread
                    12.0,
                    -3.0,
                );
                let vel = Vector3::new(
                    (index as f32 * 0.05) % 0.5 - 0.25, // Reduced velocity
                    0.0,
                    1.0,
                );
                (pos, vel, 2.0, 20.0)
            }
        };

        if index < self.particles.len() {
            self.particles[index] = PhysicsParticle {
                position: pos,
                velocity: vel,
                acceleration: Vector3::new(0.0, 0.0, 0.0),
                mass,
                particle_type,
                active: true,
                lifetime,
                max_lifetime: lifetime,
            };
        }
    }

    fn update_particles(&mut self, delta_time: f32) {
        for particle in &mut self.particles {
            if !particle.active {
                continue;
            }

            // Reset acceleration
            particle.acceleration = Vector3::new(0.0, 0.0, 0.0);

            // Apply gravity (scaled by mass)
            particle.acceleration += self.gravity;

            // Apply type-specific forces
            match particle.particle_type {
                ParticleType::Bouncy => {
                    // Bouncy particles get extra downward force (Y-up)
                    particle.acceleration += Vector3::new(0.0, -3.0, 0.0); // Reduced force
                }
                ParticleType::Floaty => {
                    // Floaty particles affected by wind
                    particle.acceleration += self.wind_force / particle.mass;

                    // Point attractor force (gentler)
                    let to_attractor = self.attractor_position - particle.position;
                    let distance = to_attractor.magnitude();
                    if distance > 0.1 {
                        let force = to_attractor.normalize()
                            * (self.attractor_strength / (distance * distance + 4.0)); // Gentler force
                        particle.acceleration += force / particle.mass;
                    }
                }
                ParticleType::Heavy => {
                    // Heavy particles sink faster (Y-up)
                    particle.acceleration += Vector3::new(0.0, -8.0, 0.0); // Reduced force
                }
            }

            // Integrate physics
            particle.velocity += particle.acceleration * delta_time;
            particle.position += particle.velocity * delta_time;

            // Boundary constraints
            let damping = match particle.particle_type {
                ParticleType::Bouncy => 0.9,
                ParticleType::Floaty => 0.7,
                ParticleType::Heavy => 0.5,
            };

            // Ground collision (Y-up: ground is at Y=0)
            if particle.position.y <= self.ground_level {
                particle.position.y = self.ground_level;
                particle.velocity.y = -particle.velocity.y * damping;
                particle.velocity.x *= 0.8;
                particle.velocity.z *= 0.8;
            }

            // Box boundaries
            if particle.position.x.abs() > self.boundary_size {
                particle.position.x = self.boundary_size * particle.position.x.signum();
                particle.velocity.x = -particle.velocity.x * damping;
            }
            if particle.position.z.abs() > self.boundary_size {
                particle.position.z = self.boundary_size * particle.position.z.signum();
                particle.velocity.z = -particle.velocity.z * damping;
            }

            // Upper boundary (Y-up)
            if particle.position.y > 15.0 {
                particle.position.y = 15.0;
                particle.velocity.y = -particle.velocity.y * damping;
            }

            // Update lifetime
            particle.lifetime -= delta_time;
            if particle.lifetime <= 0.0 {
                particle.active = false;
            }
        }
    }

    fn respawn_particles(&mut self) {
        let mut bouncy_spawned = 0;
        let mut floaty_spawned = 0;
        let mut heavy_spawned = 0;

        let inactive_indices: Vec<(usize, ParticleType)> = self
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.active)
            .map(|(i, _)| {
                let particle_type = if bouncy_spawned < self.bouncy_count {
                    bouncy_spawned += 1;
                    ParticleType::Bouncy
                } else if floaty_spawned < self.floaty_count {
                    floaty_spawned += 1;
                    ParticleType::Floaty
                } else if heavy_spawned < self.heavy_count {
                    heavy_spawned += 1;
                    ParticleType::Heavy
                } else {
                    return (i, ParticleType::Bouncy); // fallback
                };
                (i, particle_type)
            })
            .collect();

        for (i, particle_type) in inactive_indices {
            self.spawn_particle(i, particle_type);
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

                    // Add some rotation based on velocity
                    let velocity_magnitude = particle.velocity.magnitude();
                    object.ui_transform.rotation[1] = self.time * velocity_magnitude * 50.0;

                    // Adjust scale based on particle type
                    object.ui_transform.scale = match particle.particle_type {
                        ParticleType::Bouncy => 0.08,
                        ParticleType::Floaty => 0.05,
                        ParticleType::Heavy => 0.12,
                    };

                    object.apply_ui_transform();
                    object.visible = true;
                } else {
                    object.visible = false;
                }
            }
        }
    }
}

impl Simulation for PhysicsDemo {
    fn initialize(&mut self, scene: &mut Scene) {
        println!("Initializing Physics Demo...");

        let total_particles = self.bouncy_count + self.floaty_count + self.heavy_count;
        let available_objects = scene.objects.len().min(total_particles);

        self.particles.clear();
        self.particles.resize(
            available_objects,
            PhysicsParticle {
                position: Vector3::new(0.0, 0.0, 0.0),
                velocity: Vector3::new(0.0, 0.0, 0.0),
                acceleration: Vector3::new(0.0, 0.0, 0.0),
                mass: 1.0,
                particle_type: ParticleType::Bouncy,
                active: false,
                lifetime: 0.0,
                max_lifetime: 0.0,
            },
        );

        self.respawn_particles();

        println!("Initialized {} particles", available_objects);
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
            self.respawn_particles();
            self.last_spawn = 0.0;
        }

        // Update attractor position (make it move gently) - Y-up
        self.attractor_position = Vector3::new(
            2.0 * (self.time * 0.2).sin(),       // Reduced movement
            6.0 + 1.0 * (self.time * 0.3).sin(), // Y-up positioning
            2.0 * (self.time * 0.15).cos(),
        );

        // Sync particle positions to scene objects
        self.sync_to_scene(scene);
    }

    fn render_ui(&mut self, ui: &Ui) {
        let display_size = ui.io().display_size;
        let panel_width = 400.0;
        let panel_height = 200.0;
        let bottom_margin = 10.0;

        ui.window("Physics Demo")
            .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
            .position(
                [10.0, display_size[1] - panel_height - bottom_margin],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text("Multi-Physics Simulation");
                ui.separator();

                let active_count = self.particles.iter().filter(|p| p.active).count();
                ui.text(&format!("Active Particles: {}", active_count));
                ui.text(&format!("Time: {:.2}s", self.time));
                ui.spacing();

                ui.text("Particle Types:");
                ui.text(&format!(
                    "🔴 Bouncy: {}",
                    self.particles
                        .iter()
                        .filter(|p| p.active && matches!(p.particle_type, ParticleType::Bouncy))
                        .count()
                ));
                ui.text(&format!(
                    "🔵 Floaty: {}",
                    self.particles
                        .iter()
                        .filter(|p| p.active && matches!(p.particle_type, ParticleType::Floaty))
                        .count()
                ));
                ui.text(&format!(
                    "⚫ Heavy: {}",
                    self.particles
                        .iter()
                        .filter(|p| p.active && matches!(p.particle_type, ParticleType::Heavy))
                        .count()
                ));
                ui.spacing();

                ui.text("Force Parameters:");
                ui.slider("Gravity Y", -20.0, 0.0, &mut self.gravity.y);
                ui.slider("Wind Force X", -5.0, 5.0, &mut self.wind_force.x);
                ui.slider(
                    "Attractor Strength",
                    0.0,
                    50.0,
                    &mut self.attractor_strength,
                );
                ui.spacing();

                ui.text("Boundary Settings:");
                ui.slider("Boundary Size", 3.0, 15.0, &mut self.boundary_size);
                ui.slider("Ground Level", -2.0, 2.0, &mut self.ground_level);
                ui.slider("Bounce Damping", 0.1, 1.0, &mut self.bounce_damping);
                ui.spacing();

                ui.text("Spawn Control:");
                ui.slider("Spawn Rate", 0.5, 3.0, &mut self.spawn_rate);
                ui.spacing();

                if ui.button("Respawn All") {
                    self.respawn_particles();
                }
                ui.same_line();
                if ui.button("Reset") {
                    self.time = 0.0;
                    self.respawn_particles();
                }

                ui.separator();
                ui.text("Features:");
                ui.text("✓ Multiple particle types");
                ui.text("✓ Complex force interactions");
                ui.text("✓ Boundary constraints");
                ui.text("✓ Moving point attractor");
                ui.text("✓ Real-time parameters");
            });
    }

    fn name(&self) -> &str {
        "Physics Demo"
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
        self.respawn_particles();
        self.sync_to_scene(scene);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials for different types of particles
    haggis
        .app_state
        .scene
        .add_material_rgb("bouncy_red", 1.0, 0.2, 0.2, 0.9, 0.4);

    haggis
        .app_state
        .scene
        .add_material_rgb("floaty_blue", 0.2, 0.4, 1.0, 0.7, 0.3);

    haggis
        .app_state
        .scene
        .add_material_rgb("heavy_grey", 0.5, 0.5, 0.5, 0.8, 0.6);

    // Add ground plane
    haggis
        .add_object("examples/test/ground.obj")
        .with_material("heavy_grey")
        .with_name("_ground") // Prefix with _ to ignore in simulation
        .with_transform([0.0, 0.0, 0.0], 5.0, 0.0);

    // Create visual objects for particles
    // Bouncy particles (red)
    for i in 0..15 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("bouncy_red")
            .with_name(&format!("bouncy_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.08, 0.0);
    }

    // Floaty particles (blue)
    for i in 0..10 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("floaty_blue")
            .with_name(&format!("floaty_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.05, 0.0);
    }

    // Heavy particles (grey)
    for i in 0..8 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("heavy_grey")
            .with_name(&format!("heavy_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.12, 0.0);
    }

    // Create and attach the physics simulation
    let physics_demo = PhysicsDemo::new();
    haggis.attach_simulation(physics_demo);

    // Enhanced UI with physics controls
    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Physics guide
        ui.window("Physics Demo Guide")
            .size([350.0, 200.0], imgui::Condition::FirstUseEver)
            // .position(
            //     [
            //         panel_width + 20.0,
            //         display_size[1] - panel_height - bottom_margin,
            //     ],
            //     imgui::Condition::FirstUseEver,
            // )
            .build(|| {
                ui.text("Multi-Physics Simulation Demo");
                ui.separator();

                ui.text("🔴 Bouncy Particles:");
                ui.text("  • High bounce, strong gravity");
                ui.text("  • Larger size, energetic motion");
                ui.spacing();

                ui.text("🔵 Floaty Particles:");
                ui.text("  • Affected by wind and attractor");
                ui.text("  • Light mass, gentle movement");
                ui.spacing();

                ui.text("⚫ Heavy Particles:");
                ui.text("  • High mass, falls quickly");
                ui.text("  • Less bounce, steady motion");
                ui.spacing();

                ui.text("Features:");
                ui.text("✓ Multiple force types");
                ui.text("✓ Boundary constraints");
                ui.text("✓ Moving point attractor");
                ui.text("✓ Real-time parameters");
            });
    });

    haggis.run();
    Ok(())
}
