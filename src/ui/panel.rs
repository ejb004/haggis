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
    let panel_width = (display_size[0] * 0.25).max(350.0).min(450.0);
    let panel_height = (display_size[1] * 0.85).max(600.0);

    ui.window("Transform Studio")
        .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
        .size_constraints([320.0, 500.0], [600.0, display_size[1]])
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

/// Renders position control sliders
fn render_position_controls(ui: &imgui::Ui, transform: &mut UiTransformState) {
    if ui.collapsing_header("Position", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.columns(2, "pos_columns", false);

        ui.text("X");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##pos_x", -10.0, 10.0, &mut transform.position[0]);
        ui.next_column();

        ui.text("Y");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##pos_y", -10.0, 10.0, &mut transform.position[1]);
        ui.next_column();

        ui.text("Z");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##pos_z", -10.0, 10.0, &mut transform.position[2]);

        ui.columns(1, "", false);
    }
}

/// Renders rotation control sliders
fn render_rotation_controls(ui: &imgui::Ui, transform: &mut UiTransformState) {
    if ui.collapsing_header("Rotation", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.columns(2, "rot_columns", false);

        ui.text("X");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##rot_x", -180.0, 180.0, &mut transform.rotation[0]);
        ui.next_column();

        ui.text("Y");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##rot_y", -180.0, 180.0, &mut transform.rotation[1]);
        ui.next_column();

        ui.text("Z");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##rot_z", -180.0, 180.0, &mut transform.rotation[2]);

        ui.columns(1, "", false);
    }
}

/// Renders scale control slider
fn render_scale_controls(ui: &imgui::Ui, transform: &mut UiTransformState) {
    if ui.collapsing_header("Scale", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.columns(2, "scale_columns", false);

        ui.text("Uniform");
        ui.next_column();
        ui.set_next_item_width(-1.0);
        ui.slider("##scale", 0.1, 5.0, &mut transform.scale);

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
