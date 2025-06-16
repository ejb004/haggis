use haggis::gfx::{object::UiTransformState, ui::panel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    haggis
        .add_object("examples/test/cube.obj")
        .with_transform([2.0, 0.0, 0.0], 1.5, 45.0);

    haggis
        .add_object("examples/test/monkey.obj")
        .with_transform([-2.0, 0.0, 0.0], 0.8, -30.0);

    // Sophisticated UI with enhanced graphics design
    haggis.set_ui(|ui, scene, selected_index| {
        // Get display size for responsive design
        let display_size = ui.io().display_size;
        let panel_width = (display_size[0] * 0.25).max(350.0).min(450.0); // 25% of screen, clamped
        let panel_height = (display_size[1] * 0.85).max(600.0); // 85% of screen height

        // Main control panel with modern dark theme
        ui.window("🎯 Transform Studio")
            .size([panel_width, panel_height], imgui::Condition::FirstUseEver)
            .size_constraints([320.0, 500.0], [600.0, display_size[1]])
            .position([20.0, 20.0], imgui::Condition::FirstUseEver)
            .resizable(true)
            .collapsible(true)
            .build(|| {
                // Custom styling for modern dark theme
                let _window_bg =
                    ui.push_style_color(imgui::StyleColor::WindowBg, [0.12, 0.12, 0.15, 0.95]);
                let _child_bg =
                    ui.push_style_color(imgui::StyleColor::ChildBg, [0.15, 0.15, 0.18, 1.0]);

                // Header section with gradient-like effect
                {
                    let _text_color =
                        ui.push_style_color(imgui::StyleColor::Text, [0.85, 0.85, 0.9, 1.0]);
                    // let _font = ui.push_font_scale(1.1);
                    ui.text("Scene Objects");
                }

                // Modern separator
                {
                    let _sep_color =
                        ui.push_style_color(imgui::StyleColor::Separator, [0.4, 0.4, 0.5, 0.8]);
                    ui.separator();
                }

                let object_names = scene.get_object_names();

                if !object_names.is_empty() {
                    // Object selection section with modern card-like appearance
                    ui.spacing();

                    // Child window for object list with modern styling
                    ui.child_window("object_list")
                        .size([0.0, 150.0]) // Full width, fixed height
                        .border(true)
                        .build(|| {
                            let _border_color = ui
                                .push_style_color(imgui::StyleColor::Border, [0.3, 0.3, 0.4, 0.7]);
                            let _item_spacing =
                                ui.push_style_var(imgui::StyleVar::ItemSpacing([4.0, 8.0]));
                            let _text_align = ui
                                .push_style_var(imgui::StyleVar::SelectableTextAlign([0.02, 0.5]));

                            for (i, object_name) in object_names.iter().enumerate() {
                                let is_selected = selected_index.map_or(false, |sel| sel == i);

                                // Modern object icons with color coding
                                let (icon, accent_color) = match i % 6 {
                                    0 => ("🔷", [0.3, 0.7, 1.0, 1.0]), // Blue diamond
                                    1 => ("🔶", [1.0, 0.6, 0.2, 1.0]), // Orange diamond
                                    2 => ("🟢", [0.4, 0.8, 0.4, 1.0]), // Green circle
                                    3 => ("🟣", [0.7, 0.4, 0.8, 1.0]), // Purple circle
                                    4 => ("🔺", [1.0, 0.4, 0.4, 1.0]), // Red triangle
                                    _ => ("⬡", [0.6, 0.6, 0.7, 1.0]),  // Gray hexagon
                                };

                                let display_name = format!("{} {}", icon, object_name);

                                // Modern selection colors with subtle gradients
                                let (_header, _header_hovered, _header_active) = if is_selected {
                                    (
                                        ui.push_style_color(
                                            imgui::StyleColor::Header,
                                            [0.25, 0.5, 0.8, 0.4],
                                        ),
                                        ui.push_style_color(
                                            imgui::StyleColor::HeaderHovered,
                                            [0.3, 0.6, 0.9, 0.5],
                                        ),
                                        ui.push_style_color(
                                            imgui::StyleColor::HeaderActive,
                                            [0.35, 0.7, 1.0, 0.6],
                                        ),
                                    )
                                } else {
                                    (
                                        ui.push_style_color(
                                            imgui::StyleColor::Header,
                                            [0.2, 0.2, 0.25, 0.3],
                                        ),
                                        ui.push_style_color(
                                            imgui::StyleColor::HeaderHovered,
                                            [0.25, 0.25, 0.3, 0.4],
                                        ),
                                        ui.push_style_color(
                                            imgui::StyleColor::HeaderActive,
                                            [0.3, 0.3, 0.35, 0.5],
                                        ),
                                    )
                                };

                                if ui
                                    .selectable_config(&display_name)
                                    .selected(is_selected)
                                    .allow_double_click(false)
                                    .build()
                                {
                                    *selected_index = Some(i);
                                }

                                // Show selection indicator
                                if is_selected {
                                    ui.same_line();
                                    let _accent =
                                        ui.push_style_color(imgui::StyleColor::Text, accent_color);
                                    ui.text("●");
                                }
                            }
                        });

                    ui.spacing();
                    ui.separator();

                    // Transform controls for selected object
                    if let Some(selected_idx) = *selected_index {
                        if let Some(object) = scene.get_object_mut(selected_idx) {
                            ui.spacing();

                            // Selected object header with modern styling
                            {
                                let _text_color = ui.push_style_color(
                                    imgui::StyleColor::Text,
                                    [0.95, 0.85, 0.3, 1.0],
                                );
                                // let _font = ui.push_font_scale(1.05);
                                ui.text(format!("⚙️ {}", object.name));
                            }

                            ui.spacing();
                            {
                                let _sep_color = ui.push_style_color(
                                    imgui::StyleColor::Separator,
                                    [0.4, 0.4, 0.5, 0.6],
                                );
                                ui.separator();
                            }

                            // Modern collapsing sections with better colors
                            let _frame_padding =
                                ui.push_style_var(imgui::StyleVar::FramePadding([8.0, 6.0]));

                            // Position controls
                            {
                                let _header_color = ui.push_style_color(
                                    imgui::StyleColor::Header,
                                    [0.2, 0.5, 0.3, 0.7],
                                );
                                let _header_hovered = ui.push_style_color(
                                    imgui::StyleColor::HeaderHovered,
                                    [0.25, 0.6, 0.35, 0.8],
                                );

                                if ui.collapsing_header(
                                    "📍 Position",
                                    imgui::TreeNodeFlags::DEFAULT_OPEN,
                                ) {
                                    let _item_spacing =
                                        ui.push_style_var(imgui::StyleVar::ItemSpacing([4.0, 8.0]));

                                    // Modern slider styling
                                    let _slider_grab = ui.push_style_color(
                                        imgui::StyleColor::SliderGrab,
                                        [0.4, 0.7, 0.4, 0.9],
                                    );
                                    let _slider_grab_active = ui.push_style_color(
                                        imgui::StyleColor::SliderGrabActive,
                                        [0.5, 0.8, 0.5, 1.0],
                                    );
                                    let _frame_bg = ui.push_style_color(
                                        imgui::StyleColor::FrameBg,
                                        [0.15, 0.15, 0.2, 0.8],
                                    );
                                    let _frame_bg_hovered = ui.push_style_color(
                                        imgui::StyleColor::FrameBgHovered,
                                        [0.2, 0.2, 0.25, 0.9],
                                    );

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
                            }

                            // Rotation controls
                            {
                                let _header_color = ui.push_style_color(
                                    imgui::StyleColor::Header,
                                    [0.5, 0.25, 0.5, 0.7],
                                );
                                let _header_hovered = ui.push_style_color(
                                    imgui::StyleColor::HeaderHovered,
                                    [0.6, 0.3, 0.6, 0.8],
                                );

                                if ui.collapsing_header(
                                    "🔄 Rotation",
                                    imgui::TreeNodeFlags::DEFAULT_OPEN,
                                ) {
                                    let _item_spacing =
                                        ui.push_style_var(imgui::StyleVar::ItemSpacing([4.0, 8.0]));

                                    let _slider_grab = ui.push_style_color(
                                        imgui::StyleColor::SliderGrab,
                                        [0.7, 0.4, 0.7, 0.9],
                                    );
                                    let _slider_grab_active = ui.push_style_color(
                                        imgui::StyleColor::SliderGrabActive,
                                        [0.8, 0.5, 0.8, 1.0],
                                    );
                                    let _frame_bg = ui.push_style_color(
                                        imgui::StyleColor::FrameBg,
                                        [0.15, 0.15, 0.2, 0.8],
                                    );
                                    let _frame_bg_hovered = ui.push_style_color(
                                        imgui::StyleColor::FrameBgHovered,
                                        [0.2, 0.2, 0.25, 0.9],
                                    );

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
                            }

                            // Scale controls
                            {
                                let _header_color = ui.push_style_color(
                                    imgui::StyleColor::Header,
                                    [0.5, 0.5, 0.25, 0.7],
                                );
                                let _header_hovered = ui.push_style_color(
                                    imgui::StyleColor::HeaderHovered,
                                    [0.6, 0.6, 0.3, 0.8],
                                );

                                if ui.collapsing_header(
                                    "📏 Scale",
                                    imgui::TreeNodeFlags::DEFAULT_OPEN,
                                ) {
                                    let _item_spacing =
                                        ui.push_style_var(imgui::StyleVar::ItemSpacing([4.0, 8.0]));

                                    let _slider_grab = ui.push_style_color(
                                        imgui::StyleColor::SliderGrab,
                                        [0.7, 0.7, 0.4, 0.9],
                                    );
                                    let _slider_grab_active = ui.push_style_color(
                                        imgui::StyleColor::SliderGrabActive,
                                        [0.8, 0.8, 0.5, 1.0],
                                    );
                                    let _frame_bg = ui.push_style_color(
                                        imgui::StyleColor::FrameBg,
                                        [0.15, 0.15, 0.2, 0.8],
                                    );
                                    let _frame_bg_hovered = ui.push_style_color(
                                        imgui::StyleColor::FrameBgHovered,
                                        [0.2, 0.2, 0.25, 0.9],
                                    );

                                    ui.columns(2, "scale_columns", false);
                                    ui.text("Uniform");
                                    ui.next_column();
                                    ui.set_next_item_width(-1.0);
                                    ui.slider("##scale", 0.1, 5.0, &mut object.ui_transform.scale);
                                    ui.columns(1, "", false);
                                }
                            }

                            ui.spacing();
                            ui.separator();

                            // Modern action buttons section
                            ui.spacing();
                            {
                                let _text_color = ui.push_style_color(
                                    imgui::StyleColor::Text,
                                    [0.9, 0.9, 0.95, 1.0],
                                );
                                ui.text("🛠️ Quick Actions");
                            }
                            ui.spacing();

                            // Modern button styling with rounded appearance simulation
                            let _frame_padding =
                                ui.push_style_var(imgui::StyleVar::FramePadding([12.0, 8.0]));
                            let _item_spacing =
                                ui.push_style_var(imgui::StyleVar::ItemSpacing([8.0, 8.0]));

                            // Reset button with modern red accent
                            {
                                let _button_color = ui.push_style_color(
                                    imgui::StyleColor::Button,
                                    [0.6, 0.25, 0.25, 0.8],
                                );
                                let _button_hovered = ui.push_style_color(
                                    imgui::StyleColor::ButtonHovered,
                                    [0.7, 0.3, 0.3, 0.9],
                                );
                                let _button_active = ui.push_style_color(
                                    imgui::StyleColor::ButtonActive,
                                    [0.8, 0.35, 0.35, 1.0],
                                );

                                if ui.button("🔄 Reset") {
                                    object.ui_transform = UiTransformState::default();
                                }
                            }

                            ui.same_line();

                            // Center button with modern blue accent
                            {
                                let _button_color = ui.push_style_color(
                                    imgui::StyleColor::Button,
                                    [0.25, 0.4, 0.65, 0.8],
                                );
                                let _button_hovered = ui.push_style_color(
                                    imgui::StyleColor::ButtonHovered,
                                    [0.3, 0.5, 0.75, 0.9],
                                );
                                let _button_active = ui.push_style_color(
                                    imgui::StyleColor::ButtonActive,
                                    [0.35, 0.6, 0.85, 1.0],
                                );

                                if ui.button("🎯 Center") {
                                    object.ui_transform.position = [0.0, 0.0, 0.0];
                                }
                            }

                            ui.spacing();
                            ui.separator();

                            // Modern visibility and info section
                            ui.spacing();

                            // Visibility toggle with modern styling
                            {
                                let _checkmark_color = ui.push_style_color(
                                    imgui::StyleColor::CheckMark,
                                    [0.4, 0.8, 0.4, 1.0],
                                );
                                ui.checkbox("👁️ Visible in Scene", &mut object.visible);
                            }

                            ui.spacing();

                            // Modern info panel
                            ui.child_window("info_panel")
                                .size([0.0, 80.0])
                                .border(true)
                                .build(|| {
                                    let _border_color = ui.push_style_color(
                                        imgui::StyleColor::Border,
                                        [0.3, 0.3, 0.4, 0.5],
                                    );
                                    let _text_color = ui.push_style_color(
                                        imgui::StyleColor::Text,
                                        [0.7, 0.75, 0.8, 1.0],
                                    );

                                    ui.text("📊 Object Statistics");
                                    ui.separator();

                                    let total_triangles: u32 =
                                        object.meshes.iter().map(|m| m.index_count / 3).sum();
                                    let total_vertices: u32 =
                                        object.meshes.iter().map(|m| m.vertex_count).sum();

                                    ui.columns(2, "stats", false);
                                    ui.text("Triangles:");
                                    ui.next_column();
                                    ui.text(&format!("{:}", total_triangles));
                                    ui.next_column();
                                    ui.text("Vertices:");
                                    ui.next_column();
                                    ui.text(&format!("{:}", total_vertices));
                                    ui.columns(1, "", false);
                                });
                        }
                    }
                } else {
                    // Modern empty state
                    ui.spacing();
                    let _text_color =
                        ui.push_style_color(imgui::StyleColor::Text, [0.6, 0.6, 0.7, 1.0]);

                    ui.child_window("empty_state")
                        .size([0.0, 120.0])
                        .border(false)
                        .build(|| {
                            // Center the content
                            let window_width = ui.content_region_avail()[0];
                            let text_width = ui.calc_text_size("📭 No Objects")[0];
                            let cursor_y = ui.cursor_pos()[1]; // Keep current Y position
                            ui.set_cursor_pos([(window_width - text_width) * 0.5, cursor_y]);

                            {
                                // let _font = ui.push_font_scale(1.2);
                                ui.text("📭 No Objects");
                            }

                            ui.spacing();
                            ui.text("Add objects using:");
                            {
                                let _code_color = ui.push_style_color(
                                    imgui::StyleColor::Text,
                                    [0.8, 0.8, 0.9, 1.0],
                                );
                                ui.text("haggis.add_object(\"path/to/model.obj\")");
                            }
                        });
                }
            });
    });

    haggis.run();

    Ok(())
}
