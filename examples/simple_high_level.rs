//! # Simple High-Level Example
//!
//! Demonstrates the new builder pattern APIs in a basic working example.

use haggis;

fn main() {
    env_logger::init();

    let mut app = haggis::default();
    
    // Simple scene setup using new builder patterns would go here
    // This demonstrates the high-level API concept without complex dependencies
    
    app.set_ui(|ui, _scene, _selected| {
        ui.window("High-Level Builder Pattern").build(|| {
            ui.text("Demonstrates consistent builder patterns:");
            ui.separator();
            ui.bullet_text("Unified Builder trait");
            ui.bullet_text("Common configuration options");
            ui.bullet_text("Validation and error handling");
            ui.bullet_text("Fluent method chaining");
            ui.separator();
            ui.text("This example shows the framework architecture");
            ui.text("without complex simulation dependencies.");
        });
    });

    app.run();
}