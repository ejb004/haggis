//! Debug example to test if the viewport gizmo actually renders

fn main() {
    // Create a simple scene with just the viewport gizmo
    let mut app = haggis::default();
    
    // Add a reference cube to compare with
    app.add_cube()
        .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);
    
    // Add viewport gizmo
    let viewport_gizmo = haggis::gfx::gizmos::ViewportGizmo::new();
    app.add_gizmo("viewport", viewport_gizmo);

    // Simple UI to show status
    app.set_ui(|ui, scene, _selected| {
        ui.window("Debug Viewport Gizmo")
            .size([350.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🔍 Viewport Gizmo Debug");
                ui.separator();
                
                let camera = &scene.camera_manager.camera;
                ui.text(format!("Camera position: ({:.2}, {:.2}, {:.2})", 
                    camera.eye.x, camera.eye.y, camera.eye.z));
                ui.text(format!("Camera target: ({:.2}, {:.2}, {:.2})", 
                    camera.target.x, camera.target.y, camera.target.z));
                
                ui.text(format!("Total objects in scene: {}", scene.objects.len()));
                
                ui.separator();
                ui.text("❓ Can you see:");
                ui.text("• A central gray cube?");
                ui.text("• Small colored cubes in top-right?");
                ui.text("• Gizmo Manager UI panel?");
                
                ui.separator();
                ui.text("🎮 Try moving the camera around!");
                ui.text("The corner gizmo should stay in place.");
            });
    });

    app.run();
}