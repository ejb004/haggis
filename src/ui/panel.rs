// src/ui/panel.rs
//! Default UI panels for Haggis engine
//!
//! Provides pre-built UI panels for common engine functionality like object
//! transforms, material editing, and scene management.

use crate::gfx::scene::{object::UiTransformState, scene::Scene};

/// Default transform panel for object manipulation
///
/// Provides a comprehensive UI for selecting objects and editing their transforms,
/// visibility, and viewing object statistics. This is the main panel users will
/// interact with for basic scene editing.
///
/// # Arguments
/// * `ui` - ImGui UI context
/// * `scene` - Mutable scene reference for object manipulation
/// * `selected_index` - Currently selected object index
pub fn default_transform_panel(
    ui: &imgui::Ui,
    scene: &mut Scene,
    selected_index: &mut Option<usize>,
) {
    let display_size = ui.io().display_size;
    // Guard against invalid display size that could cause crashes
    if display_size[0] <= 0.0 || display_size[1] <= 0.0 {
        return;
    }
    let panel_width = (display_size[0] * 0.3).max(400.0).min(500.0); // Wider: 30% instead of 25%, min 400 instead of 350
    let panel_height = (display_size[1] * 0.85).max(600.0);

    ui.window("Transform Studio")
        .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
        .size_constraints([380.0, 500.0], [650.0, display_size[1]]) // Wider constraints
        .position([20.0, 20.0], imgui::Condition::FirstUseEver)
        .resizable(true)
        .collapsible(true)
        .build(|| {
            render_object_list(ui, scene, selected_index);
            ui.separator();
            render_transform_controls(ui, scene, selected_index);
        });
}

/// Renders the object selection list
fn render_object_list(ui: &imgui::Ui, scene: &mut Scene, selected_index: &mut Option<usize>) {
    ui.text("Scene Objects");
    ui.separator();

    let object_names = scene.get_object_names();

    if !object_names.is_empty() {
        ui.spacing();

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
    } else {
        render_empty_state(ui);
    }
}

/// Renders transform controls for the selected object
fn render_transform_controls(
    ui: &imgui::Ui,
    scene: &mut Scene,
    selected_index: &mut Option<usize>,
) {
    if let Some(selected_idx) = *selected_index {
        if let Some(object) = scene.get_object_mut(selected_idx) {
            ui.spacing();
            ui.text(&format!("Selected: {}", object.name));
            ui.spacing();
            ui.separator();

            render_position_controls(ui, &mut object.ui_transform);
            render_rotation_controls(ui, &mut object.ui_transform);
            render_scale_controls(ui, &mut object.ui_transform);
            render_action_buttons(ui, &mut object.ui_transform, &mut object.visible);
            render_object_info(ui, object);
        }
    }
}

/// Renders position control sliders with text input support
fn render_position_controls(ui: &imgui::Ui, transform: &mut UiTransformState) {
    if ui.collapsing_header("Position", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.columns(3, "pos_columns", false);

        // X Position
        ui.text("X");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##pos_x_slider", -10.0, 10.0, &mut transform.position[0]);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut x_text = format!("{:.3}", transform.position[0]);
        if ui.input_text("##pos_x_input", &mut x_text).build() {
            if let Ok(val) = x_text.parse::<f32>() {
                transform.position[0] = val;
            }
        }
        ui.next_column();

        // Y Position
        ui.text("Y");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##pos_y_slider", -10.0, 10.0, &mut transform.position[1]);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut y_text = format!("{:.3}", transform.position[1]);
        if ui.input_text("##pos_y_input", &mut y_text).build() {
            if let Ok(val) = y_text.parse::<f32>() {
                transform.position[1] = val;
            }
        }
        ui.next_column();

        // Z Position
        ui.text("Z");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##pos_z_slider", -10.0, 10.0, &mut transform.position[2]);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut z_text = format!("{:.3}", transform.position[2]);
        if ui.input_text("##pos_z_input", &mut z_text).build() {
            if let Ok(val) = z_text.parse::<f32>() {
                transform.position[2] = val;
            }
        }

        ui.columns(1, "", false);
    }
}

/// Renders rotation control sliders with text input support
fn render_rotation_controls(ui: &imgui::Ui, transform: &mut UiTransformState) {
    if ui.collapsing_header("Rotation", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.columns(3, "rot_columns", false);

        // X Rotation
        ui.text("X");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##rot_x_slider", -180.0, 180.0, &mut transform.rotation[0]);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut x_text = format!("{:.1}", transform.rotation[0]);
        if ui.input_text("##rot_x_input", &mut x_text).build() {
            if let Ok(val) = x_text.parse::<f32>() {
                transform.rotation[0] = val;
            }
        }
        ui.next_column();

        // Y Rotation
        ui.text("Y");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##rot_y_slider", -180.0, 180.0, &mut transform.rotation[1]);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut y_text = format!("{:.1}", transform.rotation[1]);
        if ui.input_text("##rot_y_input", &mut y_text).build() {
            if let Ok(val) = y_text.parse::<f32>() {
                transform.rotation[1] = val;
            }
        }
        ui.next_column();

        // Z Rotation
        ui.text("Z");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##rot_z_slider", -180.0, 180.0, &mut transform.rotation[2]);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut z_text = format!("{:.1}", transform.rotation[2]);
        if ui.input_text("##rot_z_input", &mut z_text).build() {
            if let Ok(val) = z_text.parse::<f32>() {
                transform.rotation[2] = val;
            }
        }

        ui.columns(1, "", false);

        // Clamp rotation values to reasonable range
        for rotation in &mut transform.rotation {
            *rotation = rotation.rem_euclid(360.0);
        }
    }
}

/// Renders scale control slider with text input support
fn render_scale_controls(ui: &imgui::Ui, transform: &mut UiTransformState) {
    if ui.collapsing_header("Scale", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.columns(3, "scale_columns", false);

        ui.text("Uniform");
        ui.next_column();
        ui.set_next_item_width(-30.0);
        ui.slider("##scale_slider", 0.1, 5.0, &mut transform.scale);
        ui.next_column();
        ui.set_next_item_width(-1.0);
        let mut scale_text = format!("{:.3}", transform.scale);
        if ui.input_text("##scale_input", &mut scale_text).build() {
            if let Ok(val) = scale_text.parse::<f32>() {
                transform.scale = val.max(0.01); // Prevent negative/zero scale
            }
        }

        ui.columns(1, "", false);
    }
}

/// Renders action buttons and visibility controls
fn render_action_buttons(ui: &imgui::Ui, transform: &mut UiTransformState, visible: &mut bool) {
    ui.spacing();
    ui.separator();
    ui.spacing();
    ui.text("Quick Actions");
    ui.spacing();

    if ui.button("Reset") {
        *transform = UiTransformState::default();
    }

    ui.same_line();

    if ui.button("Center") {
        transform.position = [0.0, 0.0, 0.0];
    }

    ui.spacing();
    ui.separator();
    ui.spacing();
    ui.checkbox("Visible in Scene", visible);
    ui.spacing();
}

/// Renders object statistics information
fn render_object_info(ui: &imgui::Ui, object: &crate::gfx::scene::object::Object) {
    ui.child_window("info_panel").border(true).build(|| {
        ui.text("Object Statistics");
        ui.separator();

        let total_triangles: u32 = object.meshes.iter().map(|m| m.index_count / 3).sum();
        let total_vertices: u32 = object.meshes.iter().map(|m| m.vertex_count).sum();

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

/// Renders empty state when no objects are in the scene
fn render_empty_state(ui: &imgui::Ui) {
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
