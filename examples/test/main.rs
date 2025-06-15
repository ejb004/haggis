// Try this simple UI test to see if the freeze happens with basic UI

use haggis::gfx::{object::UiTransformState, ui::panel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    haggis
        .add_object("examples/test/fractal.obj")
        .with_transform([2.0, 0.0, 0.0], 1.5, 45.0);

    haggis
        .add_object("examples/test/monkey.obj")
        .with_transform([-2.0, 0.0, 0.0], 0.8, -30.0);

    // UI with full scene access
    haggis.set_ui(|ui, scene, selected_index| {
        ui.window("Object Transform")
            .size([300.0, 400.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Scene Objects");
                ui.separator();

                let object_names = scene.get_object_names();

                if !object_names.is_empty() {
                    // Object selection
                    let mut current_selection = selected_index.unwrap_or(0);

                    if ui.combo(
                        "##object_select",
                        &mut current_selection,
                        &object_names,
                        |item| std::borrow::Cow::Borrowed(item),
                    ) {
                        *selected_index = Some(current_selection);
                    }

                    // Transform controls for selected object
                    if let Some(selected_idx) = *selected_index {
                        if let Some(object) = scene.get_object_mut(selected_idx) {
                            ui.separator();
                            ui.text(format!("Transform: {}", object.name));

                            // Position controls
                            if ui.collapsing_header("Position", imgui::TreeNodeFlags::DEFAULT_OPEN)
                            {
                                ui.slider("X", -10.0, 10.0, &mut object.ui_transform.position[0]);
                                ui.slider("Y", -10.0, 10.0, &mut object.ui_transform.position[1]);
                                ui.slider("Z", -10.0, 10.0, &mut object.ui_transform.position[2]);
                            }

                            // Rotation controls
                            if ui.collapsing_header("Rotation", imgui::TreeNodeFlags::DEFAULT_OPEN)
                            {
                                ui.slider(
                                    "X Rot",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[0],
                                );
                                ui.slider(
                                    "Y Rot",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[1],
                                );
                                ui.slider(
                                    "Z Rot",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[2],
                                );
                            }

                            // Scale controls
                            if ui.collapsing_header("Scale", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                                ui.slider(
                                    "Uniform Scale",
                                    0.1,
                                    5.0,
                                    &mut object.ui_transform.scale,
                                );
                            }

                            ui.separator();

                            // Quick actions
                            if ui.button("Reset Transform") {
                                object.ui_transform = UiTransformState::default();
                            }
                            ui.same_line();
                            if ui.button("Center Object") {
                                object.ui_transform.position = [0.0, 0.0, 0.0];
                            }

                            ui.separator();

                            // Visibility
                            ui.checkbox("Visible", &mut object.visible);
                        }
                    }
                } else {
                    ui.text("No objects in scene");
                }
            });
    });

    haggis.run();

    Ok(())
}
