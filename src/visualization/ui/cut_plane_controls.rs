//! UI controls for 2D cut plane visualization

use imgui::Ui;

/// Plane orientation options for the cut plane
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaneOrientation {
    XY, // Looking down Z axis
    XZ, // Looking down Y axis
    YZ, // Looking down X axis
}

impl PlaneOrientation {
    /// Get all available orientations
    pub fn all() -> [PlaneOrientation; 3] {
        [
            PlaneOrientation::XY,
            PlaneOrientation::XZ,
            PlaneOrientation::YZ,
        ]
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaneOrientation::XY => "XY",
            PlaneOrientation::XZ => "XZ",
            PlaneOrientation::YZ => "YZ",
        }
    }

    /// Get the normal axis for this orientation
    pub fn normal_axis(&self) -> usize {
        match self {
            PlaneOrientation::XY => 2, // Z axis
            PlaneOrientation::XZ => 1, // Y axis
            PlaneOrientation::YZ => 0, // X axis
        }
    }
}

/// Visualization mode for the cut plane
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualizationMode {
    Grid,
    Points,
    Heatmap,
}

/// Texture filtering mode for the cut plane
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    Sharp,  // Nearest filtering - pixelated, sharp boundaries
    Smooth, // Linear filtering - interpolated, smooth transitions
}

impl VisualizationMode {
    /// Get all available modes
    pub fn all() -> [VisualizationMode; 3] {
        [
            VisualizationMode::Grid,
            VisualizationMode::Points,
            VisualizationMode::Heatmap,
        ]
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            VisualizationMode::Grid => "Grid",
            VisualizationMode::Points => "Points",
            VisualizationMode::Heatmap => "Heatmap",
        }
    }
}

impl FilterMode {
    /// Get all available filter modes
    pub fn all() -> [FilterMode; 2] {
        [
            FilterMode::Sharp,
            FilterMode::Smooth,
        ]
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterMode::Sharp => "Sharp (Pixelated)",
            FilterMode::Smooth => "Smooth (Interpolated)",
        }
    }

}

/// Renders the cut plane control UI
///
/// # Arguments
///
/// * `ui` - ImGui UI context
/// * `orientation` - Current plane orientation
/// * `position` - Current plane position (0.0 to 1.0)
/// * `mode` - Current visualization mode
/// * `enabled` - Whether the cut plane is enabled
/// * `zoom` - Current zoom level
/// * `pan` - Current pan offset (x, y)
///
/// # Returns
///
/// `true` if any controls were modified
pub fn render_cut_plane_controls(
    ui: &Ui,
    orientation: &mut PlaneOrientation,
    position: &mut f32,
    mode: &mut VisualizationMode,
    enabled: &mut bool,
    zoom: &mut f32,
    pan: &mut [f32; 2],
) -> bool {
    let mut modified = false;

    ui.checkbox("Enable Cut Plane", enabled);
    if !*enabled {
        return modified;
    }

    ui.separator();

    // Plane orientation selection
    ui.text("Plane Orientation:");
    let orientations = PlaneOrientation::all();
    for &orient in &orientations {
        if ui.radio_button(orient.as_str(), orientation, orient) {
            modified = true;
        }
        if orient != orientations[orientations.len() - 1] {
            ui.same_line();
        }
    }

    ui.spacing();

    // Plane position slider
    ui.text("Plane Position:");
    if ui.slider("##plane_position", 0.0, 1.0, position) {
        modified = true;
    }

    ui.spacing();
    ui.separator();

    // Visualization mode selection
    ui.text("Visualization Mode:");
    let modes = VisualizationMode::all();
    for &vis_mode in &modes {
        if ui.radio_button(vis_mode.as_str(), mode, vis_mode) {
            modified = true;
        }
        if vis_mode != modes[modes.len() - 1] {
            ui.same_line();
        }
    }

    ui.spacing();
    ui.separator();

    // View controls
    ui.text("View Controls:");

    // Zoom control
    ui.text("Zoom:");
    if ui.slider("##zoom", 0.1, 5.0, zoom) {
        modified = true;
    }

    // Pan controls
    ui.text("Pan:");
    ui.columns(2, "pan_columns", false);

    ui.text("X:");
    ui.next_column();
    if ui.slider("##pan_x", -2.0, 2.0, &mut pan[0]) {
        modified = true;
    }
    ui.next_column();

    ui.text("Y:");
    ui.next_column();
    if ui.slider("##pan_y", -2.0, 2.0, &mut pan[1]) {
        modified = true;
    }

    ui.columns(1, "", false);

    ui.spacing();
    ui.separator();

    // Action buttons
    if ui.button("Reset View") {
        *zoom = 1.0;
        pan[0] = 0.0;
        pan[1] = 0.0;
        modified = true;
    }

    ui.same_line();

    if ui.button("Center Plane") {
        *position = 0.5;
        modified = true;
    }

    modified
}
