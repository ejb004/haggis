//! # UI Styles Example
//!
//! This example demonstrates how to use different UI styles in Haggis:
//! - Light theme (default)
//! - Dark theme
//! - Custom theme with user-defined colors
//!
//! Press '1' for Light, '2' for Dark, '3' for Custom theme.

use haggis::{UiFont, UiStyle};
use std::sync::{Arc, Mutex};

fn main() {
    // Shared style state between UI and keyboard handler
    let current_style = Arc::new(Mutex::new(UiStyle::Light));
    let style_clone = Arc::clone(&current_style);

    let mut app = haggis::default();

    // Set initial light theme
    app.set_ui_style(UiStyle::Light);
    // app.set_ui_style(UiStyle::Dark);
    // app.set_ui_style(UiStyle::Matrix);

    // Try different font options:
    app.set_ui_font(UiFont::Default);
    // app.set_ui_font(UiFont::Monospace); // Monospace fallback
    // app.set_ui_font(UiFont::Custom {
    //     data: include_bytes!("fonts/inter.ttf"), // Add your own font file
    //     size: 32.0,
    // });

    // app.set_ui_style(UiStyle::Custom {
    //     background: [0.2, 0.3, 0.4, 1.0],     // Window background
    //     text: [1.0, 1.0, 1.0, 1.0],           // Text color
    //     button: [0.4, 0.6, 0.8, 1.0],         // Button color
    //     button_hovered: [0.5, 0.7, 0.9, 1.0], // Hovered button
    //     button_active: [0.3, 0.5, 0.7, 1.0],  // Active button
    // });

    // Add some objects to demonstrate the UI
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.5, 0.0)
        .with_name("Reference Cube at Origin");

    // Add another cube for comparison at same position as plane
    app.add_object("examples/test/monkey.obj")
        .with_transform([0.0, 2.0, 0.0], 0.3, 0.0)
        .with_name("Reference Cube at Plane Position");

    // Set up UI with style switching controls and sample graph
    app.set_ui(move |ui, _scene, _selected| {
        ui.window("UI Style Demo").build(|| {
            ui.text("UI STYLE & FONT DEMO:");
            ui.separator();
            ui.button("Sample Button");
            ui.separator();
            ui.text("Font Configuration:");
            ui.text("• Default: Standard ImGui font");
            ui.text("• Monospace: Fixed-width fallback");
            ui.text("• Custom: Load your own TTF fonts");
            ui.separator();
            ui.text("Matrix Theme Active");
        });

        // Sample graph window
        ui.window("Sample Graph").build(|| {
            let values = [
                0.6, 0.1, 1.0, 0.5, 0.92, 0.1, 0.2, 0.8, 0.4, 0.3, 0.7, 0.2, 0.9, 0.1, 0.6, 0.8,
                0.3, 0.5, 0.7, 0.9,
            ];

            ui.text("Data Visualization:");
            ui.plot_lines("Performance", &values)
                .graph_size([300.0, 100.0])
                .scale_min(0.0)
                .scale_max(1.0)
                .build();

            ui.separator();
            ui.text("Controls:");
            let mut value = 0.5;
            ui.slider("Threshold", 0.0, 1.0, &mut value);

            let mut checkbox_value = true;
            ui.checkbox("Enable filtering", &mut checkbox_value);
        });
    });

    // Note: In a real application, you would handle keyboard input in the event loop
    // For this demo, we're showing the API usage. The styles can be changed by
    // modifying the app.set_ui_style() call above.

    app.run();
}
