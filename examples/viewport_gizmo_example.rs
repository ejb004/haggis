//! # Viewport Navigation Gizmo Example
//!
//! This example demonstrates the viewport navigation gizmo that provides camera orientation
//! feedback and allows clicking on faces to snap to orthographic views, similar to CAD software
//! like Fusion 360, Blender, or Maya.

fn main() {
    // Create the application
    let mut app = haggis::default();

    // Add some reference objects to the scene
    app.add_cube()
        .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
        
    app.add_cube()
        .with_transform([3.0, 0.0, 0.0], 0.5, 0.0);
        
    app.add_cube()
        .with_transform([0.0, 3.0, 0.0], 0.7, 0.0);
        
    app.add_cube()
        .with_transform([0.0, 0.0, 3.0], 0.3, 0.0);
    
    // Create and add the viewport navigation gizmo
    let viewport_gizmo = haggis::gfx::gizmos::ViewportGizmo::new();
    app.add_gizmo("viewport", viewport_gizmo);

    // Set up custom UI with viewport gizmo explanation
    app.set_ui(|ui, _scene, _selected| {
        ui.window("Viewport Gizmo Demo")
            .size([400.0, 300.0], imgui::Condition::FirstUseEver)
            .position([20.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🎯 Viewport Navigation Gizmo");
                ui.separator();
                ui.text("This gizmo shows camera orientation and provides");
                ui.text("quick navigation to standard orthographic views.");
                ui.spacing();
                
                ui.text("📱 Features:");
                ui.text("• Small colored cube in viewport corner");
                ui.text("• Shows current camera orientation");
                ui.text("• Click faces to snap to orthographic views");
                ui.text("• Smooth animated transitions");
                ui.text("• Color-coded faces (X=red, Y=green, Z=blue)");
                ui.spacing();
                
                ui.text("🎮 How to Use:");
                ui.text("1. Look for the small cube in the top-right");
                ui.text("2. Use quick view buttons in Viewport Gizmo panel");
                ui.text("3. Or click directly on cube faces (when implemented)");
                ui.text("4. Watch smooth camera transitions!");
                ui.spacing();
                
                ui.text("⚙️ Controls:");
                ui.text("• Mouse: Free-look camera");
                ui.text("• Scroll: Zoom in/out");  
                ui.text("• Viewport Gizmo panel: Quick view buttons");
                ui.text("• Adjust animation speed and camera distance");
            });
    });

    // Run the application
    app.run();
}