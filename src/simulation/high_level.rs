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
//! use haggis::simulation::builders::ParticleSystemBuilder;
//!
//! let particles = ParticleSystemBuilder::new()
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
use crate::builder::CommonConfig;
use crate::compute::ComputeEngine;
use cgmath::{InnerSpace, Vector3};
use rand::Rng;
use bytemuck::{Pod, Zeroable};

/// High-level particle system with automatic resource management
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub forces: Vec<ForceField>,
    pub constraints: Vec<Constraint>,
    pub settings: ParticleSettings,
    pub compute_engine: Option<ComputeEngine>,
    pub common: CommonConfig,
}

/// Individual particle data (GPU-compatible)
#[repr(C)]
#[derive(Clone, Debug, Copy, Pod, Zeroable)]
pub struct Particle {
    pub position: [f32; 3],
    pub velocity: [f32; 3], 
    pub acceleration: [f32; 3],
    pub mass: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub active: u32, // GPU-compatible bool
    pub _padding: u32,
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
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            acceleration: [0.0, 0.0, 0.0],
            mass: 1.0,
            lifetime: 5.0,
            max_lifetime: 5.0,
            active: 1,
            _padding: 0,
        }
    }
}

// Helper functions for Vector3 <-> array conversion
fn vec3_to_array(v: Vector3<f32>) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn array_to_vec3(a: [f32; 3]) -> Vector3<f32> {
    Vector3::new(a[0], a[1], a[2])
}

impl ForceField {
    /// Create uniform force field
    pub fn uniform(force: [f32; 3]) -> Self {
        ForceField::Uniform {
            force: array_to_vec3(force),
        }
    }

    /// Create gravity force field
    pub fn gravity(acceleration: [f32; 3]) -> Self {
        ForceField::Gravity {
            acceleration: array_to_vec3(acceleration),
        }
    }

    /// Create point attractor/repulsor
    pub fn point(position: [f32; 3], strength: f32) -> Self {
        ForceField::Point {
            position: array_to_vec3(position),
            strength,
        }
    }
}

impl ParticleSystem {
    /// Creates a new particle system with default settings
    pub fn new() -> Self {
        let settings = ParticleSettings::default();
        let particles = (0..settings.count)
            .map(|_| Particle::default())
            .collect();

        Self {
            particles,
            forces: Vec::new(),
            constraints: Vec::new(),
            settings,
            compute_engine: None,
            common: CommonConfig::default(),
        }
    }

    /// Adds a force field to the system
    pub fn add_force(&mut self, force: ForceField) -> &mut Self {
        self.forces.push(force);
        self
    }

    /// Adds a constraint to the system
    pub fn add_constraint(&mut self, constraint: Constraint) -> &mut Self {
        self.constraints.push(constraint);
        self
    }

    /// Spawns a new particle at the given position
    pub fn spawn_particle(&mut self, position: [f32; 3], velocity: [f32; 3]) {
        if let Some(particle) = self.particles.iter_mut().find(|p| p.active == 0) {
            particle.position = position;
            particle.velocity = velocity;
            particle.acceleration = [0.0, 0.0, 0.0];
            particle.lifetime = self.settings.default_lifetime;
            particle.max_lifetime = self.settings.default_lifetime;
            particle.active = 1;
        }
    }

    /// Updates the particle system using CPU
    fn update_cpu(&mut self, delta_time: f32) {
        let scaled_dt = delta_time * self.settings.time_scale;
        let damping = self.settings.damping;
        let auto_respawn = self.settings.auto_respawn;
        let default_lifetime = self.settings.default_lifetime;

        // Clone forces and constraints to avoid borrowing issues
        let forces = self.forces.clone();
        let constraints = self.constraints.clone();

        for particle in self.particles.iter_mut() {
            if particle.active == 0 {
                continue;
            }

            // Reset acceleration
            particle.acceleration = [0.0, 0.0, 0.0];

            // Apply forces
            Self::apply_forces_to_particle(&forces, particle);

            // Apply constraints
            Self::apply_constraints_to_particle(&constraints, particle);

            // Integrate physics
            Self::integrate_particle(particle, scaled_dt, damping);

            // Update lifetime
            particle.lifetime -= scaled_dt;
            if particle.lifetime <= 0.0 {
                if auto_respawn {
                    Self::respawn_particle(particle, default_lifetime);
                } else {
                    particle.active = 0;
                }
            }
        }
    }

    fn apply_forces_to_particle(forces: &[ForceField], particle: &mut Particle) {
        for force in forces {
            let force_vector = match force {
                ForceField::Uniform { force } => *force,
                ForceField::Gravity { acceleration } => *acceleration * particle.mass,
                ForceField::Point { position, strength } => {
                    let pos_vec = array_to_vec3(particle.position);
                    let direction = *position - pos_vec;
                    let distance_sq = direction.magnitude2();
                    if distance_sq > 0.001 {
                        direction.normalize() * *strength / distance_sq
                    } else {
                        Vector3::new(0.0, 0.0, 0.0)
                    }
                }
                ForceField::Radial { center, strength } => {
                    let pos_vec = array_to_vec3(particle.position);
                    let direction = pos_vec - *center;
                    let distance = direction.magnitude();
                    if distance > 0.001 {
                        direction.normalize() * *strength / (distance * distance)
                    } else {
                        Vector3::new(0.0, 0.0, 0.0)
                    }
                }
                ForceField::Vortex { center, axis, strength } => {
                    let pos_vec = array_to_vec3(particle.position);
                    let offset = pos_vec - *center;
                    let tangent = axis.cross(offset);
                    tangent.normalize() * *strength
                }
            };

            let force_array = vec3_to_array(force_vector);
            particle.acceleration[0] += force_array[0] / particle.mass;
            particle.acceleration[1] += force_array[1] / particle.mass;
            particle.acceleration[2] += force_array[2] / particle.mass;
        }
    }

    fn apply_constraints_to_particle(constraints: &[Constraint], particle: &mut Particle) {
        for constraint in constraints {
            match constraint {
                Constraint::Box { min, max, bounce } => {
                    let pos_vec = array_to_vec3(particle.position);
                    let vel_vec = array_to_vec3(particle.velocity);
                    let mut new_pos = pos_vec;
                    let mut new_vel = vel_vec;

                    // Check boundaries and bounce
                    for i in 0..3 {
                        if new_pos[i] < min[i] {
                            new_pos[i] = min[i];
                            new_vel[i] = -new_vel[i] * bounce;
                        } else if new_pos[i] > max[i] {
                            new_pos[i] = max[i];
                            new_vel[i] = -new_vel[i] * bounce;
                        }
                    }

                    particle.position = vec3_to_array(new_pos);
                    particle.velocity = vec3_to_array(new_vel);
                }
                Constraint::Sphere { center, radius, bounce } => {
                    let pos_vec = array_to_vec3(particle.position);
                    let offset = pos_vec - *center;
                    let distance = offset.magnitude();

                    if distance > *radius {
                        let normal = offset / distance;
                        let new_pos = *center + normal * *radius;
                        particle.position = vec3_to_array(new_pos);

                        let vel_vec = array_to_vec3(particle.velocity);
                        let vel_along_normal = vel_vec.dot(normal);
                        if vel_along_normal > 0.0 {
                            let new_vel = vel_vec - normal * vel_along_normal * (1.0 + *bounce);
                            particle.velocity = vec3_to_array(new_vel);
                        }
                    }
                }
                Constraint::Ground { height, bounce } => {
                    if particle.position[2] < *height {
                        particle.position[2] = *height;
                        particle.velocity[2] = -particle.velocity[2] * bounce;
                    }
                }
                Constraint::MaxVelocity { max_speed } => {
                    let vel_vec = array_to_vec3(particle.velocity);
                    let speed = vel_vec.magnitude();
                    if speed > *max_speed {
                        let new_vel = vel_vec / speed * *max_speed;
                        particle.velocity = vec3_to_array(new_vel);
                    }
                }
            }
        }
    }

    fn integrate_particle(particle: &mut Particle, dt: f32, damping: f32) {
        // Velocity Verlet integration
        particle.velocity[0] += particle.acceleration[0] * dt;
        particle.velocity[1] += particle.acceleration[1] * dt;
        particle.velocity[2] += particle.acceleration[2] * dt;

        // Apply damping
        particle.velocity[0] *= damping;
        particle.velocity[1] *= damping;
        particle.velocity[2] *= damping;

        // Update position
        particle.position[0] += particle.velocity[0] * dt;
        particle.position[1] += particle.velocity[1] * dt;
        particle.position[2] += particle.velocity[2] * dt;
    }

    fn respawn_particle(particle: &mut Particle, default_lifetime: f32) {
        // Simple respawn at origin with random velocity
        let mut rng = rand::rng();
        particle.position = [0.0, 0.0, 0.0];
        particle.velocity = [
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
        ];
        particle.acceleration = [0.0, 0.0, 0.0];
        particle.lifetime = default_lifetime;
        particle.active = 1;
    }
}

impl Simulation for ParticleSystem {
    fn initialize(&mut self, _scene: &mut Scene) {
        // Initialize particles if needed
    }
    
    fn update(&mut self, delta_time: f32, _scene: &mut Scene) {
        self.update_cpu(delta_time);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("Particle System").build(|| {
            ui.text(format!("Particles: {}", self.particles.len()));
            ui.text(format!("Forces: {}", self.forces.len()));
            ui.text(format!("Constraints: {}", self.constraints.len()));

            let active_count = self.particles.iter().filter(|p| p.active == 1).count();
            ui.text(format!("Active: {}", active_count));

            ui.slider("Spawn Rate", 0.1, 100.0, &mut self.settings.spawn_rate);
            ui.slider("Damping", 0.0, 1.0, &mut self.settings.damping);
            ui.slider("Time Scale", 0.1, 5.0, &mut self.settings.time_scale);
        });
    }
    
    fn name(&self) -> &str {
        self.common.name.as_deref().unwrap_or("Particle System")
    }
    
    fn is_running(&self) -> bool {
        self.common.enabled
    }
    
    fn set_running(&mut self, running: bool) {
        self.common.enabled = running;
    }
    
    fn reset(&mut self, _scene: &mut Scene) {
        for particle in &mut self.particles {
            *particle = Particle::default();
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}