use haggis::gfx::{object::UiTransformState, ui::panel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    haggis
        .add_object("examples/test/cube.obj")
        .with_transform([2.0, 0.0, 0.0], 1.5, 45.0);

    haggis
        .add_object("examples/test/monkey.obj")
        .with_transform([-2.0, 0.0, 0.0], 0.8, -30.0);

    haggis
        .add_object("examples/test/fractal.obj")
        .with_transform([-2.0, 10.0, 0.0], 0.8, -30.0);

    // Simple UI with default styling
    haggis.set_ui(|ui, scene, selected_index| {
        // Get display size for responsive design
        let display_size = ui.io().display_size;
        let panel_width = (display_size[0] * 0.25).max(350.0).min(450.0);
        let panel_height = (display_size[1] * 0.85).max(600.0);

        // Main control panel
        ui.window("Transform Studio")
            .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
            .size_constraints([320.0, 500.0], [600.0, display_size[1]])
            .position([20.0, 20.0], imgui::Condition::FirstUseEver)
            .resizable(true)
            .collapsible(true)
            .build(|| {
                ui.text("Scene Objects");
                ui.separator();

                let object_names = scene.get_object_names();

                if !object_names.is_empty() {
                    ui.spacing();

                    // Object selection list
                    ui.child_window("object_list")
                        .size([0.0, 150.0])
                        .border(true)
                        .build(|| {
                            for (i, object_name) in object_names.iter().enumerate() {
                                let is_selected = selected_index.map_or(false, |sel| sel == i);

                                if ui
                                    .selectable_config(object_name)
                                    .selected(is_selected)
                                    .allow_double_click(false)
                                    .build()
                                {
                                    *selected_index = Some(i);
                                }
                            }
                        });

                    ui.spacing();
                    ui.separator();

                    // Transform controls for selected object
                    if let Some(selected_idx) = *selected_index {
                        if let Some(object) = scene.get_object_mut(selected_idx) {
                            ui.spacing();
                            ui.text(&format!("Selected: {}", object.name));
                            ui.spacing();
                            ui.separator();

                            // Position controls
                            if ui.collapsing_header("Position", imgui::TreeNodeFlags::DEFAULT_OPEN)
                            {
                                ui.columns(2, "pos_columns", false);
                                ui.text("X");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider(
                                    "##pos_x",
                                    -10.0,
                                    10.0,
                                    &mut object.ui_transform.position[0],
                                );
                                ui.next_column();

                                ui.text("Y");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider(
                                    "##pos_y",
                                    -10.0,
                                    10.0,
                                    &mut object.ui_transform.position[1],
                                );
                                ui.next_column();

                                ui.text("Z");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider(
                                    "##pos_z",
                                    -10.0,
                                    10.0,
                                    &mut object.ui_transform.position[2],
                                );
                                ui.columns(1, "", false);
                            }

                            // Rotation controls
                            if ui.collapsing_header("Rotation", imgui::TreeNodeFlags::DEFAULT_OPEN)
                            {
                                ui.columns(2, "rot_columns", false);
                                ui.text("X");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider(
                                    "##rot_x",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[0],
                                );
                                ui.next_column();

                                ui.text("Y");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider(
                                    "##rot_y",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[1],
                                );
                                ui.next_column();

                                ui.text("Z");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider(
                                    "##rot_z",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[2],
                                );
                                ui.columns(1, "", false);
                            }

                            // Scale controls
                            if ui.collapsing_header("Scale", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                                ui.columns(2, "scale_columns", false);
                                ui.text("Uniform");
                                ui.next_column();
                                ui.set_next_item_width(-1.0);
                                ui.slider("##scale", 0.1, 5.0, &mut object.ui_transform.scale);
                                ui.columns(1, "", false);
                            }

                            ui.spacing();
                            ui.separator();

                            // Action buttons
                            ui.spacing();
                            ui.text("Quick Actions");
                            ui.spacing();

                            if ui.button("Reset") {
                                object.ui_transform = UiTransformState::default();
                            }

                            ui.same_line();

                            if ui.button("Center") {
                                object.ui_transform.position = [0.0, 0.0, 0.0];
                            }

                            ui.spacing();
                            ui.separator();

                            // Visibility and info
                            ui.spacing();
                            ui.checkbox("Visible in Scene", &mut object.visible);
                            ui.spacing();

                            // Object statistics
                            ui.child_window("info_panel")
                                // .size([0.0, 80.0])
                                .border(true)
                                .build(|| {
                                    ui.text("Object Statistics");
                                    ui.separator();

                                    let total_triangles: u32 =
                                        object.meshes.iter().map(|m| m.index_count / 3).sum();
                                    let total_vertices: u32 =
                                        object.meshes.iter().map(|m| m.vertex_count).sum();

                                    ui.columns(2, "stats", false);
                                    ui.text("Triangles:");
                                    ui.next_column();
                                    ui.text(&format!("{}", total_triangles));
                                    ui.next_column();
                                    ui.text("Vertices:");
                                    ui.next_column();
                                    ui.text(&format!("{}", total_vertices));
                                    ui.columns(1, "", false);
                                });
                        }
                    }
                } else {
                    // Empty state
                    ui.spacing();
                    ui.child_window("empty_state")
                        .size([0.0, 120.0])
                        .border(false)
                        .build(|| {
                            ui.text("No Objects");
                            ui.spacing();
                            ui.text("Add objects using:");
                            ui.text("haggis.add_object(\"path/to/model.obj\")");
                        });
                }
            });
    });

    haggis.run();

    Ok(())
}
