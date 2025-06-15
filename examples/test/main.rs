// Try this simple UI test to see if the freeze happens with basic UI

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    haggis.add_object("examples/test/torus.obj");
    haggis.add_object("examples/test/monkey.obj");

    // Very simple UI to test
    haggis.set_ui(|ui| {
        ui.window("Debug Test")
            .size([200.0, 100.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Simple test");

                // Try a simple button
                if ui.button("Test Button") {
                    println!("Button clicked - no freeze?");
                }

                // Show mouse position
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!("Mouse: {:.1}, {:.1}", mouse_pos[0], mouse_pos[1]));
            });
    });

    haggis.run();

    Ok(())
}
