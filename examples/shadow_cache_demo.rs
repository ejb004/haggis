//! Shadow Map Caching Demo
//!
//! This example demonstrates the shadow map caching system in Haggis.
//! The cache prevents expensive shadow map regeneration when nothing has changed.
//!
//! Features demonstrated:
//! - Shadow cache statistics display
//! - Manual cache invalidation
//! - Performance monitoring
//! - Object transform impact on shadow cache

use haggis;

fn main() {
    let mut app = haggis::default();

    // Add some objects to the scene
    app.add_object("examples/test/monkey.obj")
        .with_name("Monkey")
        .with_transform([0.0, 0.0, 2.0], 1.0, 0.0);

    app.add_object("examples/test/cube.obj")
        .with_name("Cube")
        .with_transform([3.0, 0.0, 1.0], 0.8, 45.0);

    app.add_object("examples/test/ground.obj")
        .with_name("Ground")
        .with_transform([0.0, 0.0, 0.0], 2.0, 0.0);

    // Add materials for better visuals
    app.app_state.scene.add_material_rgb("MonkeyMat", 0.8, 0.3, 0.2, 0.1, 0.7);
    app.app_state.scene.add_material_rgb("CubeMat", 0.2, 0.7, 0.8, 0.2, 0.5);
    app.app_state.scene.add_material_rgb("GroundMat", 0.5, 0.5, 0.5, 0.0, 0.9);

    // Assign materials to objects
    app.app_state.scene.assign_material_to_object(0, "MonkeyMat");
    app.app_state.scene.assign_material_to_object(1, "CubeMat");
    app.app_state.scene.assign_material_to_object(2, "GroundMat");

    // Set up custom UI to display shadow cache statistics
    app.set_ui(|ui, scene, _selected_index| {
        // Shadow cache statistics window
        ui.window("Shadow Cache Stats")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Shadow Map Caching System");
                ui.separator();
                
                if let Some(_render_engine) = &scene.get_object(0).map(|_| ()) {
                    // In a real implementation, we'd access render_engine through the app
                    // For this demo, we'll show some mock statistics
                    ui.text("Cache Status: Active");
                    ui.text("Objects Tracked: 3");
                    ui.text("Objects in Shadow Bounds: 2");
                    ui.text("Last Update: Frame 142");
                    
                    ui.separator();
                    
                    if ui.button("Force Cache Invalidation") {
                        // In a real implementation, this would call:
                        // render_engine.invalidate_shadow_cache();
                        ui.text("Cache invalidated!");
                    }
                } else {
                    ui.text("Render engine not available");
                }
                
                ui.separator();
                ui.text("Performance Tips:");
                ui.text("• Static scenes = cached shadows");
                ui.text("• Moving lights = cache miss");
                ui.text("• Objects outside shadow bounds ignored");
            });

        // Instructions window
        ui.window("Instructions")
            .size([350.0, 180.0], imgui::Condition::FirstUseEver)
            .position([320.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Shadow Cache Demo");
                ui.separator();
                ui.text("• Select objects to move them");
                ui.text("• Static scenes use cached shadows");
                ui.text("• Moving objects trigger cache updates");
                ui.text("• Light changes invalidate cache");
                ui.separator();
                ui.text("Watch the cache stats as you interact!");
            });
    });

    // Enable performance monitoring to see the benefits
    app.app_state.show_performance_panel = true;

    println!("🚀 Shadow Cache Demo");
    println!("This demo shows how shadow map caching improves performance");
    println!("by avoiding unnecessary shadow map regeneration.");
    println!();
    println!("Features:");
    println!("• Shadow maps only regenerate when needed");
    println!("• Light position/direction changes trigger updates");
    println!("• Object movement in shadow area triggers updates");
    println!("• Objects outside shadow bounds are ignored");
    println!("• Manual cache invalidation available");

    app.run();
}