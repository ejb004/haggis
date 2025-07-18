//! # Flocking Simulation Example
//!
//! This example demonstrates a boids/flocking behavior simulation.
//! It shows how to create complex emergent behaviors with simple rules.
//!
//! ## Features Demonstrated
//! - Boids flocking behavior (separation, alignment, cohesion)
//! - Emergent collective behavior from simple rules
//! - Boundary avoidance
//! - Multiple flocks with different behaviors
//! - Real-time parameter tuning
//!
//! ## Usage
//! ```bash
//! cargo run --example flocking_simulation
//! ```

use cgmath::{InnerSpace, Vector3};
use haggis::gfx::scene::Scene;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use imgui::Ui;

/// A single boid in the flocking simulation
#[derive(Clone)]
struct Boid {
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    flock_id: usize,
    active: bool,
}

/// Flocking simulation with multiple boid flocks
struct FlockingSimulation {
    boids: Vec<Boid>,
    time: f32,
    running: bool,

    // Flocking parameters
    separation_distance: f32,
    alignment_strength: f32,
    cohesion_strength: f32,
    max_speed: f32,
    perception_radius: f32,

    // Boundary parameters
    boundary_size: f32,
    boundary_avoidance: f32,

    // Flock configuration
    flock_count: usize,
    boids_per_flock: usize,

    // Environment forces
    wind_force: Vector3<f32>,
    gravity: Vector3<f32>,
}

impl FlockingSimulation {
    fn new() -> Self {
        Self {
            boids: Vec::new(),
            time: 0.0,
            running: true,
            separation_distance: 1.0,
            alignment_strength: 0.5,
            cohesion_strength: 0.3,
            max_speed: 8.0,
            perception_radius: 3.0,
            boundary_size: 8.0,
            boundary_avoidance: 2.0,
            flock_count: 3,
            boids_per_flock: 20,
            wind_force: Vector3::new(0.5, 0.0, 0.0),
            gravity: Vector3::new(0.0, 0.0, -0.5),
        }
    }

    fn spawn_boids(&mut self) {
        self.boids.clear();

        for flock_id in 0..self.flock_count {
            let center = Vector3::new(
                (flock_id as f32 - 1.0) * 4.0,
                (flock_id as f32 - 1.0) * 2.0,
                5.0 + flock_id as f32 * 2.0,
            );

            for i in 0..self.boids_per_flock {
                let angle = (i as f32 / self.boids_per_flock as f32) * 2.0 * std::f32::consts::PI;
                let radius = 1.0 + (i as f32 * 0.1) % 2.0;

                let position = center
                    + Vector3::new(
                        radius * angle.cos(),
                        radius * angle.sin(),
                        (i as f32 * 0.1) % 1.0,
                    );

                let velocity = Vector3::new(
                    (angle.cos() * 0.5) + (flock_id as f32 - 1.0) * 2.0,
                    (angle.sin() * 0.5) + (flock_id as f32 - 1.0) * 1.0,
                    (i as f32 * 0.05) % 1.0,
                );

                self.boids.push(Boid {
                    position,
                    velocity,
                    flock_id,
                    active: true,
                });
            }
        }
    }

    fn update_boids(&mut self, delta_time: f32) {
        let mut new_velocities = Vec::new();

        for (i, boid) in self.boids.iter().enumerate() {
            if !boid.active {
                new_velocities.push(boid.velocity);
                continue;
            }

            let mut separation = Vector3::new(0.0, 0.0, 0.0);
            let mut alignment = Vector3::new(0.0, 0.0, 0.0);
            let mut cohesion = Vector3::new(0.0, 0.0, 0.0);
            let mut neighbor_count = 0;

            // Calculate flocking forces
            for (j, other) in self.boids.iter().enumerate() {
                if i == j || !other.active {
                    continue;
                }

                let distance = (boid.position - other.position).magnitude();

                // Only consider boids within perception radius
                if distance > self.perception_radius {
                    continue;
                }

                // Separation - avoid crowding
                if distance < self.separation_distance && distance > 0.0 {
                    let diff = (boid.position - other.position).normalize();
                    separation += diff / distance; // Stronger separation for closer boids
                }

                // Alignment and cohesion for flockmates
                if boid.flock_id == other.flock_id {
                    alignment += other.velocity;
                    cohesion += other.position;
                    neighbor_count += 1;
                }
            }

            let mut new_velocity = boid.velocity;

            // Apply separation
            if separation.magnitude() > 0.0 {
                new_velocity += separation.normalize() * self.max_speed * 0.8;
            }

            // Apply alignment and cohesion
            if neighbor_count > 0 {
                // Alignment - steer towards average heading
                alignment = alignment / neighbor_count as f32;
                if alignment.magnitude() > 0.0 {
                    alignment = alignment.normalize() * self.max_speed;
                    new_velocity += (alignment - boid.velocity) * self.alignment_strength;
                }

                // Cohesion - steer towards average position
                cohesion = cohesion / neighbor_count as f32;
                let seek = cohesion - boid.position;
                if seek.magnitude() > 0.0 {
                    let cohesion_force = seek.normalize() * self.max_speed;
                    new_velocity += (cohesion_force - boid.velocity) * self.cohesion_strength;
                }
            }

            // Apply environmental forces
            new_velocity += self.wind_force;
            new_velocity += self.gravity;

            // Boundary avoidance
            let boundary_force = self.calculate_boundary_force(boid.position);
            new_velocity += boundary_force;

            // Limit speed
            if new_velocity.magnitude() > self.max_speed {
                new_velocity = new_velocity.normalize() * self.max_speed;
            }

            new_velocities.push(new_velocity);
        }

        // Update positions and velocities
        for (i, boid) in self.boids.iter_mut().enumerate() {
            if !boid.active {
                continue;
            }

            boid.velocity = new_velocities[i];
            boid.position += boid.velocity * delta_time;

            // Hard boundary enforcement
            if boid.position.x.abs() > self.boundary_size {
                boid.position.x = self.boundary_size * boid.position.x.signum();
                boid.velocity.x = -boid.velocity.x * 0.5;
            }
            if boid.position.y.abs() > self.boundary_size {
                boid.position.y = self.boundary_size * boid.position.y.signum();
                boid.velocity.y = -boid.velocity.y * 0.5;
            }
            if boid.position.z < 1.0 {
                boid.position.z = 1.0;
                boid.velocity.z = boid.velocity.z.abs();
            }
            if boid.position.z > 12.0 {
                boid.position.z = 12.0;
                boid.velocity.z = -boid.velocity.z.abs();
            }
        }
    }

    fn calculate_boundary_force(&self, position: Vector3<f32>) -> Vector3<f32> {
        let mut force = Vector3::new(0.0, 0.0, 0.0);

        // Boundary repulsion
        if position.x.abs() > self.boundary_size - self.boundary_avoidance {
            force.x = -position.x.signum() * self.boundary_avoidance * 2.0;
        }
        if position.y.abs() > self.boundary_size - self.boundary_avoidance {
            force.y = -position.y.signum() * self.boundary_avoidance * 2.0;
        }
        if position.z < 1.0 + self.boundary_avoidance {
            force.z = self.boundary_avoidance * 3.0;
        }
        if position.z > 12.0 - self.boundary_avoidance {
            force.z = -self.boundary_avoidance * 3.0;
        }

        force
    }

    fn sync_to_scene(&self, scene: &mut Scene) {
        for (i, boid) in self.boids.iter().enumerate() {
            if let Some(object) = scene.objects.get_mut(i) {
                if boid.active {
                    object.ui_transform.position =
                        [boid.position.x, boid.position.y, boid.position.z];

                    // Orient boid in direction of velocity
                    if boid.velocity.magnitude() > 0.1 {
                        let velocity_angle = boid.velocity.x.atan2(boid.velocity.y);
                        object.ui_transform.rotation[2] = velocity_angle.to_degrees();
                    }

                    // Different scales for different flocks
                    object.ui_transform.scale = match boid.flock_id {
                        0 => 0.06, // Red flock
                        1 => 0.05, // Blue flock
                        2 => 0.04, // Green flock
                        _ => 0.05,
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

impl Simulation for FlockingSimulation {
    fn initialize(&mut self, scene: &mut Scene) {
        println!("Initializing Flocking Simulation...");

        // Adjust flock sizes based on available objects
        let available_objects = scene.objects.len();
        self.boids_per_flock = (available_objects / self.flock_count).min(30);

        self.spawn_boids();

        println!(
            "Initialized {} boids in {} flocks",
            self.boids.len(),
            self.flock_count
        );
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if !self.running {
            return;
        }

        self.time += delta_time;

        // Update boid flocking behavior
        self.update_boids(delta_time);

        // Sync boid positions to scene objects
        self.sync_to_scene(scene);
    }

    fn render_ui(&mut self, ui: &Ui) {
        let display_size = ui.io().display_size;
        let panel_width = 400.0;
        let panel_height = 200.0;
        let bottom_margin = 10.0;

        ui.window("Flocking Simulation")
            .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
            .position(
                [10.0, display_size[1] - panel_height - bottom_margin],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text("Boids/Flocking Behavior");
                ui.separator();

                ui.text(&format!(
                    "Active Boids: {}",
                    self.boids.iter().filter(|b| b.active).count()
                ));
                ui.text(&format!("Flocks: {}", self.flock_count));
                ui.text(&format!("Boids per Flock: {}", self.boids_per_flock));
                ui.text(&format!("Time: {:.2}s", self.time));
                ui.spacing();

                ui.text("Flocking Parameters:");
                ui.slider(
                    "Separation Distance",
                    0.5,
                    3.0,
                    &mut self.separation_distance,
                );
                ui.slider("Alignment Strength", 0.0, 1.0, &mut self.alignment_strength);
                ui.slider("Cohesion Strength", 0.0, 1.0, &mut self.cohesion_strength);
                ui.slider("Max Speed", 1.0, 15.0, &mut self.max_speed);
                ui.slider("Perception Radius", 1.0, 5.0, &mut self.perception_radius);
                ui.spacing();

                ui.text("Environment:");
                ui.slider("Boundary Size", 3.0, 15.0, &mut self.boundary_size);
                ui.slider("Boundary Avoidance", 0.5, 5.0, &mut self.boundary_avoidance);
                ui.slider("Wind Force X", -2.0, 2.0, &mut self.wind_force.x);
                ui.slider("Gravity Z", -2.0, 2.0, &mut self.gravity.z);
                ui.spacing();

                if ui.button("Respawn Boids") {
                    self.spawn_boids();
                }
                ui.same_line();
                if ui.button("Scatter") {
                    for boid in &mut self.boids {
                        boid.velocity += Vector3::new(
                            (self.time.sin() * 137.0 + boid.position.x * 0.1) % 2.0 - 1.0,
                            (self.time.cos() * 113.0 + boid.position.y * 0.1) % 2.0 - 1.0,
                            (self.time.sin() * 89.0 + boid.position.z * 0.1) % 1.0 - 0.5,
                        ) * 10.0;
                    }
                }

                ui.separator();
                ui.text("Flocking Principles:");
                ui.text("✓ Separation - Avoid crowding");
                ui.text("✓ Alignment - Steer towards average heading");
                ui.text("✓ Cohesion - Steer towards average position");
                ui.text("✓ Boundary avoidance");
                ui.text("✓ Emergent behavior");
            });

        ui.window("Flock Statistics")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .position(
                [
                    panel_width + 20.0,
                    display_size[1] - panel_height - bottom_margin,
                ],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text("Flocking Behavior Analysis");
                ui.separator();

                for flock_id in 0..self.flock_count {
                    let flock_boids: Vec<_> = self
                        .boids
                        .iter()
                        .filter(|b| b.flock_id == flock_id && b.active)
                        .collect();
                    let count = flock_boids.len();

                    if count > 0 {
                        let avg_speed: f32 = flock_boids
                            .iter()
                            .map(|b| b.velocity.magnitude())
                            .sum::<f32>()
                            / count as f32;
                        let center = flock_boids
                            .iter()
                            .fold(Vector3::new(0.0, 0.0, 0.0), |acc, b| acc + b.position)
                            / count as f32;
                        let spread = flock_boids
                            .iter()
                            .map(|b| (b.position - center).magnitude())
                            .fold(0.0, f32::max);

                        let color = match flock_id {
                            0 => "🔴",
                            1 => "🔵",
                            2 => "🟢",
                            _ => "⚫",
                        };

                        ui.text(&format!(
                            "{} Flock {} ({} boids):",
                            color,
                            flock_id + 1,
                            count
                        ));
                        ui.text(&format!("  Avg Speed: {:.2}", avg_speed));
                        ui.text(&format!("  Spread: {:.2}", spread));
                        ui.spacing();
                    }
                }

                ui.separator();
                ui.text("Emergent behavior from simple rules!");
            });
    }

    fn name(&self) -> &str {
        "Flocking Simulation"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, scene: &mut Scene) {
        self.time = 0.0;
        self.spawn_boids();
        self.sync_to_scene(scene);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials for different flocks
    haggis
        .app_state
        .scene
        .add_material_rgb("flock_red", 1.0, 0.3, 0.3, 0.8, 0.4);

    haggis
        .app_state
        .scene
        .add_material_rgb("flock_blue", 0.3, 0.5, 1.0, 0.8, 0.4);

    haggis
        .app_state
        .scene
        .add_material_rgb("flock_green", 0.3, 1.0, 0.3, 0.8, 0.4);

    // Visual objects for boids
    // Red flock
    for i in 0..20 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("flock_red")
            .with_name(&format!("boid_red_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.06, 0.0);
    }

    // Blue flock
    for i in 0..20 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("flock_blue")
            .with_name(&format!("boid_blue_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.05, 0.0);
    }

    // Green flock
    for i in 0..20 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("flock_green")
            .with_name(&format!("boid_green_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.04, 0.0);
    }

    // Create and attach the flocking simulation
    let flocking_sim = FlockingSimulation::new();
    haggis.attach_simulation(flocking_sim);

    // Flocking-specific UI
    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Flocking guide
        ui.window("Flocking Guide")
            .size([350.0, 200.0], imgui::Condition::FirstUseEver)
            // .position(
            //     [
            //         panel_width + 330.0,
            //         display_size[1] - panel_height - bottom_margin,
            //     ],
            //     imgui::Condition::FirstUseEver,
            // )
            .build(|| {
                ui.text("Boids/Flocking Behavior Demo");
                ui.separator();

                ui.text("This simulation demonstrates:");
                ui.text("• Emergent flocking behavior");
                ui.text("• Three classic boids rules:");
                ui.text("  1. Separation - Avoid crowding");
                ui.text("  2. Alignment - Steer towards average heading");
                ui.text("  3. Cohesion - Steer towards average position");
                ui.text("• Boundary avoidance");
                ui.text("• Environmental forces");
                ui.spacing();

                ui.text("Watch how simple rules create complex,");
                ui.text("life-like group behavior!");
                ui.spacing();

                ui.text("Each color represents a different flock:");
                ui.text("🔴 Red flock   🔵 Blue flock   🟢 Green flock");
                ui.spacing();

                ui.text("Adjust parameters in the Flocking panel");
                ui.text("to see how they affect the behavior.");
            });
    });

    haggis.run();
    Ok(())
}
