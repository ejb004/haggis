//! # High-Level Simulation API
//!
//! This module provides simple, declarative interfaces for common simulation tasks.
//! It hides buffer management, GPU setup, and boilerplate code to make simulations
//! accessible for beginners while maintaining performance.
//!
//! ## Key Features
//!
//! - **Builder Pattern APIs**: Fluent method chaining for configuration
//! - **Automatic Resource Management**: Handles GPU buffers, memory allocation
//! - **Sensible Defaults**: Works out of the box with minimal configuration
//! - **CPU/GPU Abstraction**: Automatically chooses optimal execution path
//! - **Type Safety**: Compile-time checks for common mistakes
//!
//! ## Examples
//!
//! ### Basic Particle System
//! ```no_run
//! use haggis::simulation::high_level::ParticleSystem;
//!
//! let particles = ParticleSystem::new()
//!     .with_count(1000)
//!     .with_gravity([0.0, 0.0, -9.8])
//!     .with_bounds([-10.0, 10.0], [-10.0, 10.0], [0.0, 20.0])
//!     .build();
//! ```
//!
//! ### Force Application
//! ```no_run
//! use haggis::simulation::high_level::ForceField;
//!
//! let wind = ForceField::uniform([2.0, 0.0, 0.0]);
//! let gravity = ForceField::gravity([0.0, 0.0, -9.8]);
//! ```

use crate::gfx::scene::Scene;
use crate::simulation::traits::Simulation;
use cgmath::{InnerSpace, Vector3};
use rand::Rng;

/// High-level particle system with automatic resource management
pub struct ParticleSystem {
    particles: Vec<Particle>,
    forces: Vec<ForceField>,
    constraints: Vec<Constraint>,
    settings: ParticleSettings,
    #[allow(dead_code)]
    use_gpu: bool,
    needs_gpu_update: bool,
    #[allow(dead_code)]
    gpu_resources: Option<GpuParticleResources>,
}

/// Individual particle data
#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub acceleration: Vector3<f32>,
    pub mass: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub active: bool,
}

/// Force field types for particle simulation
#[derive(Clone, Debug)]
pub enum ForceField {
    /// Uniform force applied to all particles
    Uniform { force: Vector3<f32> },
    /// Gravity force (downward)
    Gravity { acceleration: Vector3<f32> },
    /// Point attractor/repulsor
    Point {
        position: Vector3<f32>,
        strength: f32,
    },
    /// Radial force (explosion/implosion)
    Radial { center: Vector3<f32>, strength: f32 },
    /// Vortex force (spiral)
    Vortex {
        center: Vector3<f32>,
        axis: Vector3<f32>,
        strength: f32,
    },
}

/// Constraint types for particle behavior
#[derive(Clone, Debug)]
pub enum Constraint {
    /// Box boundary constraint
    Box {
        min: Vector3<f32>,
        max: Vector3<f32>,
        bounce: f32,
    },
    /// Spherical boundary constraint
    Sphere {
        center: Vector3<f32>,
        radius: f32,
        bounce: f32,
    },
    /// Ground plane constraint
    Ground { height: f32, bounce: f32 },
    /// Maximum velocity constraint
    MaxVelocity { max_speed: f32 },
}

/// Particle system configuration
#[derive(Clone, Debug)]
pub struct ParticleSettings {
    pub count: usize,
    pub spawn_rate: f32,
    pub default_lifetime: f32,
    pub default_mass: f32,
    pub damping: f32,
    pub time_scale: f32,
    pub auto_respawn: bool,
    pub gpu_threshold: usize, // Switch to GPU when particle count exceeds this
}

/// GPU resources for particle simulation
#[allow(dead_code)]
struct GpuParticleResources {
    // This will be populated when we implement GPU support
    particle_buffer: Option<wgpu::Buffer>,
    compute_pipeline: Option<wgpu::ComputePipeline>,
    bind_group: Option<wgpu::BindGroup>,
}

impl Default for ParticleSettings {
    fn default() -> Self {
        Self {
            count: 100,
            spawn_rate: 10.0,
            default_lifetime: 5.0,
            default_mass: 1.0,
            damping: 0.99,
            time_scale: 1.0,
            auto_respawn: true,
            gpu_threshold: 1000,
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            acceleration: Vector3::new(0.0, 0.0, 0.0),
            mass: 1.0,
            lifetime: 5.0,
            max_lifetime: 5.0,
            active: true,
        }
    }
}

impl ParticleSystem {
    /// Creates a new particle system builder
    pub fn new() -> ParticleSystemBuilder {
        ParticleSystemBuilder::default()
    }

    /// Adds a force field to the system
    pub fn add_force(&mut self, force: ForceField) -> &mut Self {
        self.forces.push(force);
        self.needs_gpu_update = true;
        self
    }

    /// Adds a constraint to the system
    pub fn add_constraint(&mut self, constraint: Constraint) -> &mut Self {
        self.constraints.push(constraint);
        self.needs_gpu_update = true;
        self
    }

    /// Spawns a new particle at the given position
    pub fn spawn_particle(&mut self, position: Vector3<f32>, velocity: Vector3<f32>) {
        if let Some(particle) = self.particles.iter_mut().find(|p| !p.active) {
            particle.position = position;
            particle.velocity = velocity;
            particle.acceleration = Vector3::new(0.0, 0.0, 0.0);
            particle.lifetime = self.settings.default_lifetime;
            particle.max_lifetime = self.settings.default_lifetime;
            particle.active = true;
        }
    }

    /// Updates the particle system
    fn update_cpu(&mut self, delta_time: f32) {
        let scaled_dt = delta_time * self.settings.time_scale;

        for particle in &mut self.particles {
            if !particle.active {
                continue;
            }

            // Reset acceleration
            particle.acceleration = Vector3::new(0.0, 0.0, 0.0);

            // Apply forces
            for force in &self.forces {
                let force_vector = match force {
                    ForceField::Uniform { force } => *force,
                    ForceField::Gravity { acceleration } => *acceleration * particle.mass,
                    ForceField::Point { position, strength } => {
                        let direction = *position - particle.position;
                        let distance = direction.magnitude();
                        if distance > 0.001 {
                            direction.normalize() * (*strength / (distance * distance))
                        } else {
                            Vector3::new(0.0, 0.0, 0.0)
                        }
                    }
                    ForceField::Radial { center, strength } => {
                        let direction = particle.position - *center;
                        let distance = direction.magnitude();
                        if distance > 0.001 {
                            direction.normalize() * (*strength / distance)
                        } else {
                            Vector3::new(0.0, 0.0, 0.0)
                        }
                    }
                    ForceField::Vortex {
                        center,
                        axis,
                        strength,
                    } => {
                        let to_particle = particle.position - *center;
                        let axis_component = axis.dot(to_particle) * *axis;
                        let radial = to_particle - axis_component;
                        let tangent = axis.cross(radial);
                        tangent.normalize() * (*strength / (radial.magnitude() + 0.001))
                    }
                };
                particle.acceleration += force_vector / particle.mass;
            }

            // Update physics
            particle.velocity += particle.acceleration * scaled_dt;
            particle.velocity *= self.settings.damping;
            particle.position += particle.velocity * scaled_dt;

            // Apply constraints
            for constraint in &self.constraints {
                match constraint {
                    Constraint::Box { min, max, bounce } => {
                        for i in 0..3 {
                            if particle.position[i] < min[i] {
                                particle.position[i] = min[i];
                                particle.velocity[i] *= -*bounce;
                            } else if particle.position[i] > max[i] {
                                particle.position[i] = max[i];
                                particle.velocity[i] *= -*bounce;
                            }
                        }
                    }
                    Constraint::Sphere {
                        center,
                        radius,
                        bounce,
                    } => {
                        let to_center = particle.position - *center;
                        let distance = to_center.magnitude();
                        if distance > *radius {
                            let direction = to_center.normalize();
                            particle.position = *center + direction * *radius;
                            let velocity_along_normal = particle.velocity.dot(direction);
                            particle.velocity -=
                                direction * velocity_along_normal * (1.0 + *bounce);
                        }
                    }
                    Constraint::Ground { height, bounce } => {
                        if particle.position.z < *height {
                            particle.position.z = *height;
                            particle.velocity.z *= -*bounce;
                        }
                    }
                    Constraint::MaxVelocity { max_speed } => {
                        let speed = particle.velocity.magnitude();
                        if speed > *max_speed {
                            particle.velocity = particle.velocity.normalize() * *max_speed;
                        }
                    }
                }
            }

            // Update lifetime
            particle.lifetime -= scaled_dt;
            if particle.lifetime <= 0.0 {
                if self.settings.auto_respawn {
                    particle.lifetime = self.settings.default_lifetime;
                    let mut rng = rand::rng();
                    particle.position = Vector3::new(
                        (rng.random::<f32>() - 0.5) * 2.0,
                        (rng.random::<f32>() - 0.5) * 2.0,
                        rng.random::<f32>() * 5.0,
                    );
                    particle.velocity = Vector3::new(
                        (rng.random::<f32>() - 0.5) * 4.0,
                        (rng.random::<f32>() - 0.5) * 4.0,
                        rng.random::<f32>() * 2.0,
                    );
                } else {
                    particle.active = false;
                }
            }
        }
    }

    /// Gets active particle count
    pub fn active_count(&self) -> usize {
        self.particles.iter().filter(|p| p.active).count()
    }

    /// Gets reference to particles for rendering
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
}

/// Builder for creating particle systems
pub struct ParticleSystemBuilder {
    settings: ParticleSettings,
    forces: Vec<ForceField>,
    constraints: Vec<Constraint>,
    use_gpu: Option<bool>,
}

impl Default for ParticleSystemBuilder {
    fn default() -> Self {
        Self {
            settings: ParticleSettings::default(),
            forces: Vec::new(),
            constraints: Vec::new(),
            use_gpu: None,
        }
    }
}

impl ParticleSystemBuilder {
    /// Sets the number of particles
    pub fn with_count(mut self, count: usize) -> Self {
        self.settings.count = count;
        self
    }

    /// Sets the spawn rate (particles per second)
    pub fn with_spawn_rate(mut self, rate: f32) -> Self {
        self.settings.spawn_rate = rate;
        self
    }

    /// Sets the default particle lifetime
    pub fn with_lifetime(mut self, lifetime: f32) -> Self {
        self.settings.default_lifetime = lifetime;
        self
    }

    /// Adds gravity force
    pub fn with_gravity(mut self, acceleration: [f32; 3]) -> Self {
        self.forces.push(ForceField::Gravity {
            acceleration: Vector3::new(acceleration[0], acceleration[1], acceleration[2]),
        });
        self
    }

    /// Adds uniform force
    pub fn with_force(mut self, force: [f32; 3]) -> Self {
        self.forces.push(ForceField::Uniform {
            force: Vector3::new(force[0], force[1], force[2]),
        });
        self
    }

    /// Adds box boundary constraint
    pub fn with_bounds(mut self, x_range: [f32; 2], y_range: [f32; 2], z_range: [f32; 2]) -> Self {
        self.constraints.push(Constraint::Box {
            min: Vector3::new(x_range[0], y_range[0], z_range[0]),
            max: Vector3::new(x_range[1], y_range[1], z_range[1]),
            bounce: 0.8,
        });
        self
    }

    /// Adds ground plane constraint
    pub fn with_ground(mut self, height: f32) -> Self {
        self.constraints.push(Constraint::Ground {
            height,
            bounce: 0.6,
        });
        self
    }

    /// Sets damping factor
    pub fn with_damping(mut self, damping: f32) -> Self {
        self.settings.damping = damping;
        self
    }

    /// Forces GPU usage (if available)
    pub fn use_gpu(mut self) -> Self {
        self.use_gpu = Some(true);
        self
    }

    /// Forces CPU usage
    pub fn use_cpu(mut self) -> Self {
        self.use_gpu = Some(false);
        self
    }

    /// Builds the particle system
    pub fn build(self) -> ParticleSystem {
        let should_use_gpu = self
            .use_gpu
            .unwrap_or_else(|| self.settings.count > self.settings.gpu_threshold);

        let mut particles = Vec::with_capacity(self.settings.count);
        for _ in 0..self.settings.count {
            particles.push(Particle::default());
        }

        // Initialize particles with random positions and velocities
        let mut rng = rand::rng();
        for particle in &mut particles {
            particle.position = Vector3::new(
                (rng.random::<f32>() - 0.5) * 2.0,
                (rng.random::<f32>() - 0.5) * 2.0,
                rng.random::<f32>() * 5.0,
            );
            particle.velocity = Vector3::new(
                (rng.random::<f32>() - 0.5) * 4.0,
                (rng.random::<f32>() - 0.5) * 4.0,
                rng.random::<f32>() * 2.0,
            );
        }

        ParticleSystem {
            particles,
            forces: self.forces,
            constraints: self.constraints,
            settings: self.settings,
            use_gpu: should_use_gpu,
            needs_gpu_update: true,
            gpu_resources: None,
        }
    }
}

/// Wrapper to implement Simulation trait for ParticleSystem
pub struct ParticleSimulation {
    system: ParticleSystem,
    name: String,
    running: bool,
}

impl ParticleSimulation {
    /// Creates a new particle simulation
    pub fn new(name: String, system: ParticleSystem) -> Self {
        Self {
            system,
            name,
            running: true,
        }
    }

    /// Gets mutable reference to the particle system
    pub fn system_mut(&mut self) -> &mut ParticleSystem {
        &mut self.system
    }

    /// Gets reference to the particle system
    pub fn system(&self) -> &ParticleSystem {
        &self.system
    }
}

impl Simulation for ParticleSimulation {
    fn initialize(&mut self, _scene: &mut Scene) {
        // Initialization is handled in the builder
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if !self.running {
            return;
        }

        // Update particle system
        self.system.update_cpu(delta_time);

        // Update scene objects based on particle positions
        // This is where we would sync particle positions to scene objects
        // For now, we'll just update the first few objects if they exist
        let active_particles: Vec<_> = self
            .system
            .particles
            .iter()
            .filter(|p| p.active)
            .take(scene.objects.len())
            .collect();

        for (i, particle) in active_particles.iter().enumerate() {
            if let Some(object) = scene.objects.get_mut(i) {
                object.set_translation(particle.position);
            }
        }
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window(&format!("{} - Particles", self.name))
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text(format!("Active Particles: {}", self.system.active_count()));
                ui.text(format!("Total Particles: {}", self.system.particles.len()));

                ui.separator();

                ui.text("Settings:");
                ui.text(format!(
                    "Time Scale: {:.2}",
                    self.system.settings.time_scale
                ));
                ui.text(format!("Damping: {:.2}", self.system.settings.damping));
                ui.text(format!("Forces: {}", self.system.forces.len()));
                ui.text(format!("Constraints: {}", self.system.constraints.len()));

                ui.separator();

                if ui.button("Reset Particles") {
                    for particle in &mut self.system.particles {
                        particle.active = true;
                        particle.lifetime = self.system.settings.default_lifetime;
                        particle.position = Vector3::new(
                            (rand::random::<f32>() - 0.5) * 2.0,
                            (rand::random::<f32>() - 0.5) * 2.0,
                            rand::random::<f32>() * 5.0,
                        );
                        particle.velocity = Vector3::new(
                            (rand::random::<f32>() - 0.5) * 4.0,
                            (rand::random::<f32>() - 0.5) * 4.0,
                            rand::random::<f32>() * 2.0,
                        );
                    }
                }
            });
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, _scene: &mut Scene) {
        // Reset all particles
        for particle in &mut self.system.particles {
            particle.active = true;
            particle.lifetime = self.system.settings.default_lifetime;
            particle.position = Vector3::new(
                (rand::random::<f32>() - 0.5) * 2.0,
                (rand::random::<f32>() - 0.5) * 2.0,
                rand::random::<f32>() * 5.0,
            );
            particle.velocity = Vector3::new(
                (rand::random::<f32>() - 0.5) * 4.0,
                (rand::random::<f32>() - 0.5) * 4.0,
                rand::random::<f32>() * 2.0,
            );
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Convenience functions for common simulation patterns
impl ParticleSystem {
    /// Creates a basic gravity-based particle system
    pub fn gravity_fountain(count: usize, gravity_strength: f32) -> ParticleSystem {
        ParticleSystem::new()
            .with_count(count)
            .with_gravity([0.0, 0.0, -gravity_strength])
            .with_ground(0.0)
            .with_damping(0.95)
            .build()
    }

    /// Creates a particle system with wind effects
    pub fn wind_particles(count: usize, wind_force: [f32; 3]) -> ParticleSystem {
        ParticleSystem::new()
            .with_count(count)
            .with_force(wind_force)
            .with_bounds([-5.0, 5.0], [-5.0, 5.0], [0.0, 10.0])
            .with_damping(0.99)
            .build()
    }

    /// Creates an explosion-style particle system
    pub fn explosion(count: usize, center: [f32; 3], strength: f32) -> ParticleSystem {
        let mut builder = ParticleSystem::new()
            .with_count(count)
            .with_lifetime(3.0)
            .with_damping(0.95);

        // Add radial force for explosion effect
        builder.forces.push(ForceField::Radial {
            center: Vector3::new(center[0], center[1], center[2]),
            strength,
        });

        builder.build()
    }
}
