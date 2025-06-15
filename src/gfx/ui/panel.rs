use imgui::Ui;

use crate::gfx::{object::UiTransformState, scene::Scene};

pub fn objects() -> impl FnMut(&Ui, &mut Scene, &mut Option<usize>) {
    move |ui, scene, selected_object_index| {
        ui.window("Object Transform")
            .size([350.0, 500.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Scene Objects");
                ui.separator();

                let object_names = scene.get_object_names();

                if !object_names.is_empty() {
                    ui.text("Select Object:");
                    let mut current_selection = selected_object_index.unwrap_or(0);

                    if ui.combo(
                        "##object_select",
                        &mut current_selection,
                        &object_names,
                        |item| std::borrow::Cow::Borrowed(item),
                    ) {
                        *selected_object_index = Some(current_selection);
                    }

                    if let Some(selected_idx) = *selected_object_index {
                        if let Some(object) = scene.get_object_mut(selected_idx) {
                            ui.separator();
                            ui.text(format!("Transform: {}", object.name));

                            let mut changed = false;

                            if ui.collapsing_header("Position", imgui::TreeNodeFlags::DEFAULT_OPEN)
                            {
                                changed |= ui.slider(
                                    "X",
                                    -10.0,
                                    10.0,
                                    &mut object.ui_transform.position[0],
                                );
                                changed |= ui.slider(
                                    "Y",
                                    -10.0,
                                    10.0,
                                    &mut object.ui_transform.position[1],
                                );
                                changed |= ui.slider(
                                    "Z",
                                    -10.0,
                                    10.0,
                                    &mut object.ui_transform.position[2],
                                );
                            }

                            if ui.collapsing_header("Rotation", imgui::TreeNodeFlags::DEFAULT_OPEN)
                            {
                                changed |= ui.slider(
                                    "X Rot",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[0],
                                );
                                changed |= ui.slider(
                                    "Y Rot",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[1],
                                );
                                changed |= ui.slider(
                                    "Z Rot",
                                    -180.0,
                                    180.0,
                                    &mut object.ui_transform.rotation[2],
                                );
                            }

                            if ui.collapsing_header("Scale", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                                changed |= ui.slider(
                                    "Uniform Scale",
                                    0.1,
                                    5.0,
                                    &mut object.ui_transform.scale,
                                );
                            }

                            if changed {
                                object.apply_ui_transform();
                            }

                            ui.separator();
                            ui.text("Quick Actions:");
                            if ui.button("Reset Transform") {
                                object.ui_transform = UiTransformState::default();
                                object.apply_ui_transform();
                            }
                            ui.same_line();
                            if ui.button("Center Object") {
                                object.ui_transform.position = [0.0, 0.0, 0.0];
                                object.apply_ui_transform();
                            }

                            ui.separator();
                            ui.checkbox("Visible", &mut object.visible);

                            ui.separator();
                            ui.text("Object Info:");
                            // let total_triangles: u32 =
                            //     object.meshes.iter().map(|m| m.index_count / 3).sum();
                            // let total_vertices: usize =
                            //     object.meshes.iter().map(|m| m.vertices.len()).sum();
                            // ui.text(format!("Triangles: {}", total_triangles));
                            // ui.text(format!("Vertices: {}", total_vertices));
                        }
                    }
                } else {
                    ui.text("No objects in scene");
                    ui.text("Add objects with haggis.add_object()");
                }
            });
    }
}
