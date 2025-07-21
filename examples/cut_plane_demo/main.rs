//! # 2D Data Plane Visualization Demo
//!
//! This example demonstrates the 2D data plane visualization system in Haggis.
//! It shows how to provide 2D data arrays directly to the visualization component
//! for rendering as textured planes in 3D space.
//!
//! ## Features Demonstrated
//!
//! - Creating 2D data arrays directly (no 3D slicing)
//! - Adding a data plane visualization component
//! - Using the visualization UI controls:
//!   - Visualization modes (Heatmap, Grid, Points)
//!   - 3D positioning and size controls
//!   - Zoom and pan controls
//! - New API design with direct 2D data input
//!
//! ## Usage
//!
//! Run with: `cargo run --example cut_plane_demo`
//!
//! Use the right-side panel to control the data plane visualization:
//! - Try different visualization modes to see various representations
//! - Adjust position and size to move the plane in 3D space
//! - Use zoom and pan to examine details in the 2D view

use haggis::{simulation::BaseSimulation, CutPlane2D};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting 2D Data Plane Visualization Demo");
    println!("========================================");
    println!();
    println!("This demo shows how to visualize 2D data arrays directly");
    println!("using the new data plane visualization system.");
    println!();
    println!("Controls (in right-side panel):");
    println!("- Visualization Mode: Heatmap, Grid, or Points");
    println!("- Position: Move the plane in 3D space");
    println!("- Size: Adjust the plane size");
    println!("- View Controls: Zoom and pan the 2D view");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create a simulation with data plane visualization
    let mut simulation = BaseSimulation::new("2D Data Plane Analysis");

    // Create and configure the data plane visualization
    let mut data_plane = CutPlane2D::new();

    // Generate 2D data directly (new API!)
    let (data_2d, width, height) = generate_2d_test_data();
    data_plane.update_data(data_2d, width, height);

    // Position the plane in 3D space - place it obviously in front of camera
    // In Z-up coordinate system: X=right, Y=forward, Z=up
    // Camera looks at origin from behind, so place plane at Y=2 (forward from origin, towards camera view)
    data_plane.set_position(cgmath::Vector3::new(0.0, 2.0, 0.0)); // In front of camera in Z-up system
    data_plane.set_size(2.0); // Make it large to be very visible

    // Add the visualization to the simulation
    simulation.add_visualization("data_plane", data_plane);

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add some basic 3D objects for context (optional)
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.5, 0.0)
        .with_name("Reference Cube at Origin");

    // Add another cube for comparison at same position as plane
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 2.0, 0.0], 0.3, 0.0)
        .with_name("Reference Cube at Plane Position");

    // The simulation now handles its own UI through the BaseSimulation
    // No need for manual UI registration - the simulation manages its visualizations

    // Run the application
    app.run();

    Ok(())
}

/// Generate 2D test data with concentric circles pattern
fn generate_2d_test_data() -> (Vec<f32>, u32, u32) {
    let width = 80u32;
    let height = 80u32;

    let mut data = Vec::with_capacity((width * height) as usize);

    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_radius = (width.min(height) as f32) / 2.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            // Create concentric circles pattern
            let normalized_distance = distance / max_radius;
            let circles = ((normalized_distance * 8.0).sin() + 1.0) / 2.0;

            // Add some noise and variation
            let noise_x = (x as f32 * 0.15).sin();
            let noise_y = (y as f32 * 0.15).cos();
            let noise = (noise_x + noise_y) * 0.1;

            // Combine effects
            let value = (circles + noise).clamp(0.0, 1.0);

            // Add gradient effect
            let gradient = (x as f32 / width as f32) * 0.2;
            let final_value = (value + gradient).clamp(0.0, 1.0);

            data.push(final_value);
        }
    }

    (data, width, height)
}

/// Generate 2D checkerboard pattern
fn generate_checkerboard_2d_data() -> (Vec<f32>, u32, u32) {
    let width = 64u32;
    let height = 64u32;

    let mut data = Vec::with_capacity((width * height) as usize);
    let square_size = 8u32;

    for y in 0..height {
        for x in 0..width {
            let square_x = x / square_size;
            let square_y = y / square_size;

            let value = if (square_x + square_y) % 2 == 0 {
                1.0
            } else {
                0.2
            };

            data.push(value);
        }
    }

    (data, width, height)
}

/// Generate 2D sine wave pattern
#[allow(dead_code)]
fn generate_sine_wave_2d_data() -> (Vec<f32>, u32, u32) {
    let width = 64u32;
    let height = 64u32;

    let mut data = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        for x in 0..width {
            let fx = (x as f32 / width as f32) * 4.0 * std::f32::consts::PI;
            let fy = (y as f32 / height as f32) * 4.0 * std::f32::consts::PI;

            let value = (fx.sin() * fy.cos() + 1.0) / 2.0;
            data.push(value);
        }
    }

    (data, width, height)
}
