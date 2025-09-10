//! # Simulation Builders
//!
//! Consistent builder patterns for simulation components

use crate::builder::{Builder, CommonConfig, ConfigurableBuilder};
use crate::simulation::high_level::{ParticleSystem, ForceField, Constraint, ParticleSettings, Particle};
use cgmath::Vector3;

/// Builder for particle systems with fluent API
pub struct ParticleSystemBuilder {
    pub(crate) common: CommonConfig,
    pub(crate) settings: ParticleSettings,
    pub(crate) forces: Vec<ForceField>,
    pub(crate) constraints: Vec<Constraint>,
    pub(crate) initial_particles: Vec<Particle>,
}

impl Default for ParticleSystemBuilder {
    fn default() -> Self {
        Self {
            common: CommonConfig::default(),
            settings: ParticleSettings::default(),
            forces: Vec::new(),
            constraints: Vec::new(),
            initial_particles: Vec::new(),
        }
    }
}

impl ParticleSystemBuilder {
    /// Create new particle system builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set particle count
    pub fn with_count(mut self, count: usize) -> Self {
        self.settings.count = count;
        self
    }

    /// Add gravity force
    pub fn with_gravity(mut self, acceleration: [f32; 3]) -> Self {
        self.forces.push(ForceField::Gravity {
            acceleration: Vector3::new(acceleration[0], acceleration[1], acceleration[2]),
        });
        self
    }

    /// Add uniform force (like wind)
    pub fn with_uniform_force(mut self, force: [f32; 3]) -> Self {
        self.forces.push(ForceField::Uniform {
            force: Vector3::new(force[0], force[1], force[2]),
        });
        self
    }

    /// Add box boundary constraint
    pub fn with_bounds(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.constraints.push(Constraint::Box {
            min: Vector3::new(min[0], min[1], min[2]),
            max: Vector3::new(max[0], max[1], max[2]),
            bounce: 0.8,
        });
        self
    }

    /// Set particle lifetime
    pub fn with_lifetime(mut self, lifetime: f32) -> Self {
        self.settings.default_lifetime = lifetime;
        self
    }

    /// Set spawn rate (particles per second)
    pub fn with_spawn_rate(mut self, rate: f32) -> Self {
        self.settings.spawn_rate = rate;
        self
    }

    /// Set particle mass
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.settings.default_mass = mass;
        self
    }

    /// Set damping factor (0.0 = no damping, 1.0 = full damping)
    pub fn with_damping(mut self, damping: f32) -> Self {
        self.settings.damping = damping.clamp(0.0, 1.0);
        self
    }

    /// Add custom force field
    pub fn with_force(mut self, force: ForceField) -> Self {
        self.forces.push(force);
        self
    }

    /// Add custom constraint
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }
}

impl Builder<ParticleSystem> for ParticleSystemBuilder {
    fn build(self) -> ParticleSystem {
        let particles = (0..self.settings.count)
            .map(|_| Particle::default())
            .collect();

        ParticleSystem {
            particles,
            forces: self.forces,
            constraints: self.constraints,
            settings: self.settings,
            compute_engine: None,
            common: self.common,
        }
    }
}

impl ConfigurableBuilder<ParticleSystem> for ParticleSystemBuilder {
    fn merge(mut self, other: Self) -> Self {
        self.forces.extend(other.forces);
        self.constraints.extend(other.constraints);
        self.initial_particles.extend(other.initial_particles);
        
        // Merge settings (other takes precedence if values differ)
        if other.settings.count != ParticleSettings::default().count {
            self.settings.count = other.settings.count;
        }
        if other.settings.spawn_rate != ParticleSettings::default().spawn_rate {
            self.settings.spawn_rate = other.settings.spawn_rate;
        }
        
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.settings.count == 0 {
            return Err("Particle count must be greater than 0".to_string());
        }
        if self.settings.spawn_rate < 0.0 {
            return Err("Spawn rate must be non-negative".to_string());
        }
        if self.settings.default_lifetime <= 0.0 {
            return Err("Particle lifetime must be positive".to_string());
        }
        Ok(())
    }
}

// Implement common builder methods using macro
crate::impl_common_builder_methods!(ParticleSystemBuilder);