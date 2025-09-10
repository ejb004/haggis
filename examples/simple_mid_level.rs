//! # Simple Mid-Level Example  
//!
//! Demonstrates moderate control with builder patterns and direct API access.

use haggis;

fn main() {
    env_logger::init();

    let mut app = haggis::default();
    
    // Mid-level usage would combine builders with some direct control
    app.set_ui(|ui, _scene, _selected| {
        ui.window("Mid-Level Control").build(|| {
            ui.text("Demonstrates intermediate usage:");
            ui.separator();
            ui.bullet_text("Mix of builders and direct API");
            ui.bullet_text("Custom configuration handling");
            ui.bullet_text("Runtime parameter adjustment");
            ui.bullet_text("Selective abstraction usage");
            ui.separator();
            ui.text("This level provides balance between");
            ui.text("ease of use and fine-grained control.");
        });
    });

    app.run();
}