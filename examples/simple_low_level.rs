//! # Simple Low-Level Example
//!
//! Demonstrates direct GPU access and manual resource management concepts.

use haggis;

fn main() {
    env_logger::init();

    let mut app = haggis::default();
    
    app.set_ui(|ui, _scene, _selected| {
        ui.window("Low-Level GPU Access").build(|| {
            ui.text("Demonstrates expert-level control:");
            ui.separator();
            ui.bullet_text("Direct GPU buffer management");
            ui.bullet_text("Custom compute shaders");
            ui.bullet_text("Manual memory optimization");
            ui.bullet_text("Performance monitoring");
            ui.bullet_text("Explicit resource lifecycle");
            ui.separator();
            ui.text("GPU Compute Abstraction Layer:");
            ui.bullet_text("ComputeEngine for buffer management");
            ui.bullet_text("ComputeBuilder for pipeline setup");
            ui.bullet_text("Automatic workgroup calculation");
            ui.bullet_text("Pipeline caching and reuse");
            ui.separator();
            ui.text("This level provides maximum control");
            ui.text("for performance-critical applications.");
        });
    });

    app.run();
}