use haggis::ui::default_transform_panel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials in the scene
    haggis
        .app_state
        .scene
        .add_material_rgb("gold", 1.0, 0.84, 0.0, 1.0, 0.1); // Shiny gold
    haggis
        .app_state
        .scene
        .add_material_rgb("plastic", 0.2, 0.6, 0.8, 0.0, 0.8); // Rough blue plastic
    haggis
        .app_state
        .scene
        .add_material_rgb("steel", 0.7, 0.7, 0.7, 0.9, 0.1); // Metallic steel

    // Add objects and assign materials
    haggis
        .add_object("examples/test/cube.obj")
        .with_material("gold") // Assign gold material
        .with_transform([2.0, 0.0, 0.0], 1.5, 45.0);

    haggis
        .add_object("examples/test/monkey.obj")
        .with_material("plastic") // Assign plastic material
        .with_transform([-2.0, 0.0, 0.0], 0.8, -30.0);

    haggis
        .add_object("examples/test/fractal.obj")
        .with_material("steel") // Assign steel material
        .with_transform([-2.0, 10.0, 0.0], 0.8, -30.0);

    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);
    });

    haggis.run();
    Ok(())
}
