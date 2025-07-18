//! # Basic CPU Particle System Example
//!
//! This example demonstrates the simplest possible particle system using
//! the high-level API. Perfect for beginners who want to get started quickly.

use crate::simulation::high_level::{ParticleSystem, ParticleSimulation};

/// Creates a simple CPU particle system with gravity
pub fn create_basic_particles() -> ParticleSimulation {
    let particles = ParticleSystem::new()
        .with_count(500)
        .with_gravity([0.0, 0.0, -9.8])
        .with_ground(0.0)
        .with_damping(0.95)
        .build();

    ParticleSimulation::new("Basic Particles".to_string(), particles)
}

/// Creates a particle fountain effect
pub fn create_fountain() -> ParticleSimulation {
    let particles = ParticleSystem::new()
        .with_count(1000)
        .with_gravity([0.0, 0.0, -15.0])
        .with_force([0.0, 0.0, 8.0])  // Upward force
        .with_ground(-1.0)
        .with_damping(0.98)
        .build();

    ParticleSimulation::new("Fountain".to_string(), particles)
}

/// Creates particles affected by wind
pub fn create_wind_particles() -> ParticleSimulation {
    let particles = ParticleSystem::new()
        .with_count(300)
        .with_force([5.0, 0.0, 0.0])  // Wind force
        .with_gravity([0.0, 0.0, -5.0])
        .with_bounds([-10.0, 10.0], [-10.0, 10.0], [0.0, 15.0])
        .with_damping(0.99)
        .build();

    ParticleSimulation::new("Wind Particles".to_string(), particles)
}

/// Creates an explosion effect
pub fn create_explosion() -> ParticleSimulation {
    let particles = ParticleSystem::explosion(
        800,
        [0.0, 0.0, 5.0],  // Center position
        50.0,             // Explosion strength
    );

    ParticleSimulation::new("Explosion".to_string(), particles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::traits::Simulation;
    use crate::gfx::scene::Scene;
    use crate::gfx::camera::{camera_utils::CameraManager, orbit_camera::OrbitCamera, camera_controller::CameraController};
    use cgmath::Vector3;

    #[test]
    fn test_basic_particles_creation() {
        let simulation = create_basic_particles();
        assert_eq!(simulation.name(), "Basic Particles");
        assert!(simulation.is_running());
        assert_eq!(simulation.system().particles().len(), 500);
    }

    #[test]
    fn test_fountain_creation() {
        let simulation = create_fountain();
        assert_eq!(simulation.name(), "Fountain");
        assert_eq!(simulation.system().particles().len(), 1000);
    }

    #[test]
    fn test_simulation_update() {
        let mut simulation = create_basic_particles();
        let camera = OrbitCamera::new(8.0, 0.4, 0.2, Vector3::new(0.0, 0.0, 0.0), 1.0);
        let controller = CameraController::new(0.005, 0.1);
        let camera_manager = CameraManager::new(camera, controller);
        let mut scene = Scene::new(camera_manager);

        // Initialize and update
        simulation.initialize(&mut scene);
        simulation.update(0.016, &mut scene);

        // Check that particles are still active
        assert!(simulation.system().active_count() > 0);
    }
}