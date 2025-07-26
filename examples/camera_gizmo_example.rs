//! # Camera Gizmo Example
//!
//! This example demonstrates how to use the camera gizmo system to visualize
//! camera positions and movement history in 3D space.

fn main() {
    // Create the application
    let mut app = haggis::default();

    // Add a simple object to the scene for reference
    app.add_cube()
        .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
    
    // Create and configure the camera gizmo directly
    let camera_gizmo = haggis::gfx::gizmos::CameraGizmo::new();

    // Add the camera gizmo to the application
    app.add_gizmo("camera", camera_gizmo);

    // Set up custom UI with gizmo controls
    app.set_ui(|ui, scene, _selected| {
        ui.window("Camera Gizmo Demo")
            .size([350.0, 150.0], imgui::Condition::FirstUseEver)
            .position([20.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Camera Gizmo Example");
                ui.separator();
                ui.text("Move the camera around to see the gizmo in action!");
                ui.text("The red cube shows the current camera position.");
                ui.text("Enable history to see blue spheres for previous positions.");
                ui.separator();
                ui.text("Camera Controls:");
                ui.text("• Left Click + Drag: Orbit around target");
                ui.text("• Scroll: Zoom in/out");
                ui.text("• Shift + Left Click + Drag: Pan camera");
            });
    });

    // Run the application
    app.run();
}