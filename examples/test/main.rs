use haggis::simulation::examples::cpu::SimplyMove;
use haggis::ui::default_transform_panel;
use haggis::visualization::CutPlane2D;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials in the scene
    haggis
        .app_state
        .scene
        .add_material_rgb("gold", 1.0, 0.84, 0.0, 1.0, 0.5); // Shiny gold

    haggis
        .app_state
        .scene
        .add_material_rgb("grey", 0.6, 0.6, 0.6, 0.5, 0.5); // grey ground

    haggis
        .app_state
        .scene
        .add_material_rgb("red", 0.9, 0.01, 0.01, 0.8, 0.4); // red plastic

    haggis
        .add_object("examples/test/f1.obj")
        .with_material("red") // Assign steel material
        .with_transform([0.0, -0.665, 0.0], 0.8, -30.0);
    haggis
        .add_object("examples/test/cube.obj")
        .with_material("gold") // Assign steel material
        .with_transform([0.0, -0.665, 0.0], 0.8, -30.0);

    haggis
        .add_object("examples/test/ground.obj")
        .with_material("grey") // Assign steel material
        .with_name("_ground")
        .with_transform([0.0, 0.0, 0.0], 1.0, 0.0);

    // Add cut plane visualization
    let cut_plane = CutPlane2D::new();
    haggis.add_visualization("cut_plane", cut_plane);

    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);
    });

    // haggis.attach_simulation(SimplyMove::new());

    // haggis.attach_simulation(
    //     haggis::simulation::examples::gpu::simply_move_gpu::GpuSimplyMove::new(),
    // );

    haggis.run();
    Ok(())
}
