//! # Haggis Quickstart Example
//!
//! This is the "Hello World" example for the Haggis particle simulation framework.
//! Perfect for beginners who want to understand the core concepts quickly.
//!
//! ## What this example shows:
//! - How to create a basic Haggis application
//! - How to add 3D objects to the scene
//! - How to create a simple particle simulation
//! - How to add basic UI controls
//! - How to run the framework
//!
//! ## Usage:
//! ```bash
//! cargo run --example quickstart
//! ```
//!
//! ## What you'll see:
//! - A few colorful cubes bouncing around with gravity
//! - Simple UI controls to adjust physics
//! - Real-time particle count display
//!
//! This example demonstrates the essential patterns you'll use in more complex simulations.

use haggis::prelude::*;

/// A simple particle with basic properties
/// This represents one object in our simulation
#[derive(Clone)]
struct SimpleParticle {
    position: Vector3<f32>, // Where the particle is in 3D space
    velocity: Vector3<f32>, // How fast and in what direction it's moving
    active: bool,           // Whether this particle should be simulated
    height: f32,           // Height of the object for collision detection
}

/// Our basic particle simulation
/// This handles all the physics and updates for our particles
struct QuickstartSimulation {
    particles: Vec<SimpleParticle>, // All our particles
    gravity: f32,                   // How strong gravity is (negative = down)
    bounce_damping: f32,            // How much energy is lost when bouncing
    ground_level: f32,              // Z position of the ground
    time: f32,                      // How long the simulation has been running
    running: bool,                  // Whether the simulation is active
}

impl QuickstartSimulation {
    /// Create a new simulation with sensible defaults
    fn new() -> Self {
        Self {
            particles: Vec::new(),
            gravity: -9.8,       // Earth-like gravity
            bounce_damping: 0.8, // Lose 20% energy on bounce
            ground_level: 0.0,   // Ground at Z=0
            time: 0.0,
            running: true,
        }
    }

    /// Add a new particle at the given position with some initial velocity
    fn add_particle(&mut self, position: Vector3<f32>, velocity: Vector3<f32>) {
        self.add_particle_with_height(position, velocity, 0.4); // Default height for cubes
    }

    /// Add a new particle with specified height for collision detection
    fn add_particle_with_height(&mut self, position: Vector3<f32>, velocity: Vector3<f32>, height: f32) {
        self.particles.push(SimpleParticle {
            position,
            velocity,
            active: true,
            height,
        });
        println!(
            "Added particle at {:?} with velocity {:?} and height {:.2}",
            position, velocity, height
        );
    }

    /// Update all particles by one time step
    /// This is where the physics happens!
    fn update_particles(&mut self, delta_time: f32) {
        for particle in &mut self.particles {
            if !particle.active {
                continue; // Skip inactive particles
            }

            // STEP 1: Apply gravity to velocity (Z-up coordinate system)
            // Gravity affects velocity, not position directly
            particle.velocity.z += self.gravity * delta_time;

            // STEP 2: Apply velocity to position
            // This moves the particle based on its current velocity
            particle.position += particle.velocity * delta_time;

            // STEP 3: Handle ground collision (Z-up coordinate system)
            // Check collision using bottom face of object (position - height/2)
            let bottom_z = particle.position.z - particle.height / 2.0;
            if bottom_z <= self.ground_level {
                // Position the object so its bottom face is exactly on ground
                particle.position.z = self.ground_level + particle.height / 2.0;
                particle.velocity.z = -particle.velocity.z * self.bounce_damping; // Reverse and dampen

                // Also slow down horizontal movement a bit (friction)
                particle.velocity.x *= 0.9;
                particle.velocity.y *= 0.9;
            }

            // STEP 4: Simple boundary constraints
            // Keep particles in a reasonable area
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

    /// Synchronize particle positions with the visual objects in the scene
    /// This is how we make the 3D cubes move to match our simulation
    fn sync_particles_to_scene(&self, scene: &mut Scene) {
        for (i, particle) in self.particles.iter().enumerate() {
            // Get the corresponding 3D object from the scene
            // Skip the ground plane (last object) - it's not a particle
            if let Some(object) = scene.objects.get_mut(i) {
                // Don't update the ground plane
                if object.name == "ground_plane" {
                    continue;
                }

                if particle.active {
                    // Update the object's position to match the particle
                    object.ui_transform.position = [
                        particle.position.x,
                        particle.position.y,
                        particle.position.z,
                    ];

                    // Add a little rotation for visual flair
                    object.ui_transform.rotation[2] = self.time * 45.0; // Rotate around Z axis

                    // Apply the transform and make sure it's visible
                    object.apply_ui_transform();
                    object.visible = true;
                } else {
                    // Hide inactive particles
                    object.visible = false;
                }
            }
        }

        // Ensure ground plane stays in place and visible
        if let Some(ground) = scene
            .objects
            .iter_mut()
            .find(|obj| obj.name == "ground_plane")
        {
            ground.ui_transform.position = [0.0, 0.0, 0.0];
            ground.apply_ui_transform();
            ground.visible = true;
        }
    }

    /// Reset all particles to their starting positions
    fn reset_simulation(&mut self) {
        self.time = 0.0;
        self.particles.clear();

        // Add some particles in interesting starting positions (Z-up coordinate system)
        self.add_particle(Vector3::new(-2.0, 0.0, 8.0), Vector3::new(2.0, 1.0, 0.0));
        self.add_particle(Vector3::new(0.0, 0.0, 10.0), Vector3::new(0.0, 1.5, 0.0));
        self.add_particle(Vector3::new(2.0, -1.0, 6.0), Vector3::new(-1.0, 0.5, 2.0));
    }
}

/// Implement the Simulation trait so Haggis knows how to run our simulation
impl Simulation for QuickstartSimulation {
    /// Called once when the simulation starts
    /// This is where we set up our initial state
    fn initialize(&mut self, _scene: &mut Scene) {
        println!("🚀 Starting Haggis Quickstart Simulation!");
        println!("   - Initializing particles...");

        self.reset_simulation();

        println!("   - Created {} particles", self.particles.len());
        println!("   - Ready to simulate!");
    }

    /// Called every frame to update the simulation
    /// This is the main simulation loop
    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        if !self.running {
            return; // Skip update if paused
        }

        // Update our internal timer
        self.time += delta_time;

        // Run the physics simulation
        self.update_particles(delta_time);

        // Make the visual objects match our simulation
        self.sync_particles_to_scene(scene);
    }

    /// Called every frame to draw the user interface
    /// This is where we create controls and display information
    fn render_ui(&mut self, ui: &Ui) {
        // Get screen size for positioning
        let display_size = ui.io().display_size;

        // Create a control panel window
        ui.window("Quickstart Controls")
            .size([350.0, 300.0], imgui::Condition::FirstUseEver)
            .position(
                [10.0, display_size[1] - 320.0],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text("🚀 Haggis Quickstart Example");
                ui.separator();

                // Display simulation info
                ui.text(&format!("Simulation Time: {:.1}s", self.time));
                ui.text(&format!(
                    "Active Particles: {}",
                    self.particles.iter().filter(|p| p.active).count()
                ));
                ui.spacing();

                // Physics controls
                ui.text("Physics Settings:");
                ui.slider("Gravity", -20.0, 0.0, &mut self.gravity);
                ui.slider("Bounce Damping", 0.0, 1.0, &mut self.bounce_damping);
                ui.slider("Ground Level", -2.0, 3.0, &mut self.ground_level);
                ui.spacing();

                // Control buttons
                if ui.button("⏸️ Pause / ▶️ Play") {
                    self.running = !self.running;
                }
                ui.same_line();
                if ui.button("🔄 Reset") {
                    self.reset_simulation();
                }

                ui.separator();
                ui.text("💡 Tips:");
                ui.text("• Adjust gravity to see different effects");
                ui.text("• Try different damping values");
                ui.text("• Watch the cubes bounce and interact");
                ui.text("• Use camera controls to look around");
            });

        // Create an information panel
        ui.window("Framework Info")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .position(
                [display_size[0] - 310.0, 10.0],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text("📚 What's Happening:");
                ui.separator();
                ui.text("1. Particles start with position & velocity");
                ui.text("2. Gravity affects velocity each frame");
                ui.text("3. Velocity affects position each frame");
                ui.text("4. Collisions cause bouncing");
                ui.text("5. Visual objects follow particles");
                ui.spacing();

                ui.text("🎮 Camera Controls:");
                ui.text("• Mouse: Look around");
                ui.text("• Scroll: Zoom in/out");
                ui.text("• Shift+Mouse: Pan view");
                ui.spacing();

                ui.text("⚡ Next Steps:");
                ui.text("Explore high_level/ and low_level/");
                ui.text("examples for advanced features!");
            });
    }

    /// Return the name of this simulation
    fn name(&self) -> &str {
        "Quickstart Example"
    }

    /// Check if the simulation is currently running
    fn is_running(&self) -> bool {
        self.running
    }

    /// Start or stop the simulation
    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// Reset the simulation to initial state
    fn reset(&mut self, _scene: &mut Scene) {
        self.reset_simulation();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Main function - this is where everything starts
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Haggis Framework Quickstart");
    println!("==============================");
    println!("This example demonstrates the basics of the Haggis particle simulation framework.");
    println!("You'll see some cubes bouncing around with realistic physics!");
    println!();

    // STEP 1: Create the Haggis application
    // This sets up the window, graphics, and basic systems
    let mut haggis = haggis::default();
    println!("✅ Created Haggis application");

    // STEP 2: Create materials for our objects
    // Materials define how objects look (color, shininess, etc.)
    haggis
        .app_state
        .scene
        .add_material_rgb("red_cube", 1.0, 0.3, 0.3, 0.8, 0.4); // Red, somewhat metallic

    haggis
        .app_state
        .scene
        .add_material_rgb("green_cube", 0.3, 1.0, 0.3, 0.2, 0.6); // Green, less metallic

    haggis
        .app_state
        .scene
        .add_material_rgb("blue_cube", 0.3, 0.3, 1.0, 0.5, 0.3); // Blue, medium metallic

    // Add ground plane material
    haggis
        .app_state
        .scene
        .add_material_rgb("ground", 0.7, 0.7, 0.7, 0.1, 0.8); // Gray, non-metallic ground

    println!("✅ Created materials (red, green, blue, ground)");

    // STEP 3: Add 3D objects to the scene (using procedural geometry!)
    // These are the visual representations of our particles
    haggis
        .add_cube() // Create a procedural cube
        .with_material("red_cube") // Make it red
        .with_name("particle_1") // Give it a name
        .with_transform([0.0, 0.0, 0.0], 0.2, 0.0); // Position, scale, rotation

    haggis
        .add_cube()
        .with_material("green_cube")
        .with_name("particle_2")
        .with_transform([0.0, 0.0, 0.0], 0.15, 0.0);

    haggis
        .add_cube()
        .with_material("blue_cube")
        .with_name("particle_3")
        .with_transform([0.0, 0.0, 0.0], 0.25, 0.0);

    // Add ground plane (static, not affected by physics)
    haggis
        .add_plane(10.0, 10.0, 1, 1) // Create a 10x10 plane
        .with_material("ground")
        .with_name("ground_plane")
        .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);

    println!("✅ Added 3 cube objects and ground plane to the scene");

    // STEP 4: Create our simulation
    // This contains all the physics and particle logic
    let simulation = QuickstartSimulation::new();
    haggis.attach_simulation(simulation);
    println!("✅ Created and attached particle simulation");

    // STEP 5: Set up the user interface and enable performance monitoring
    // This defines what controls and panels are shown
    haggis.show_performance_panel(true); // Enable performance metrics
    haggis.set_ui(|ui, scene, selected_index| {
        // Show the default object inspector panel
        default_transform_panel(ui, scene, selected_index);
    });
    println!("✅ Set up user interface with performance monitoring");

    // STEP 6: Run the application!
    // This starts the main loop: update physics, render graphics, handle input
    println!();
    println!("🚀 Starting application...");
    println!("   💡 Look for the bouncing cubes!");
    println!("   🎮 Use mouse to look around, scroll to zoom");
    println!("   ⚙️  Adjust physics in the control panel");
    println!("   ❌ Close the window to exit");
    println!();

    haggis.run();

    println!("👋 Thanks for trying the Haggis Quickstart!");
    Ok(())
}

/// Additional helper functions and examples you might find useful:

#[allow(dead_code)]
fn example_adding_more_particles(simulation: &mut QuickstartSimulation) {
    // You can add particles anywhere in 3D space
    simulation.add_particle(
        Vector3::new(0.0, 0.0, 5.0), // Position: 5 units up
        Vector3::new(1.0, 0.0, 0.0), // Velocity: moving right
    );

    // Particles can start with any velocity
    simulation.add_particle(
        Vector3::new(-3.0, 2.0, 8.0),  // Position: up and to the left
        Vector3::new(0.5, -0.5, -1.0), // Velocity: moving down and diagonal
    );
}

#[allow(dead_code)]
fn example_physics_variations() {
    // Different physics settings create different effects:

    // Low gravity (moon-like)
    let _moon_gravity = -1.6;

    // High gravity (heavy planet)
    let _heavy_gravity = -20.0;

    // No gravity (space)
    let _zero_gravity = 0.0;

    // Super bouncy (rubber balls)
    let _rubber_damping = 0.95;

    // No bounce (sticky collisions)
    let _sticky_damping = 0.0;
}
