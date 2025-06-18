use haggis::{simulation::examples::cpu::SimplyMove, ui::default_transform_panel};

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
        .add_material_rgb("grey", 0.6, 0.6, 0.6, 0.5, 0.0); // grey ground

    haggis
        .add_object("examples/test/monkey.obj")
        .with_material("gold") // Assign steel material
        .with_transform([0.0, 0.0, 0.0], 0.8, -30.0);

    haggis
        .add_object("examples/test/cube.obj")
        .with_material("grey") // Assign steel material
        .with_name("_ground")
        .with_transform([0.0, -20.0, 0.0], 10.0, 0.0);

    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);
    });

    haggis.attach_simulation(SimplyMove::new());

    haggis.run();
    Ok(())
}
