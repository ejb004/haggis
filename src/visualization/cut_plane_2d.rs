//! 2D Data Plane Visualization Component
//!
//! Generic 2D data visualizer that accepts 2D data arrays directly from the user.
//! No hardcoded 3D slicing logic - purely for displaying 2D data.

use super::rendering::VisualizationMaterial;
use super::traits::VisualizationComponent;
use super::ui::cut_plane_controls::{FilterMode, VisualizationMode};
use crate::gfx::{resources::texture_resource::TextureResource, scene::Scene};
use cgmath::Vector3;
use imgui::Ui;
use wgpu::{Device, Queue, Buffer};
use std::sync::Arc;

/// Data source for 2D visualization
#[derive(Clone)]
pub enum DataSource {
    /// CPU data - traditional Vec<f32> approach
    CpuData(Vec<f32>),
    /// GPU buffer - direct reference for compute shaders
    GpuBuffer {
        buffer: Arc<Buffer>,
        format: BufferFormat,
    },
}

/// Buffer data format specification
#[derive(Clone, Copy, Debug)]
pub struct BufferFormat {
    pub element_type: BufferElementType,
    pub width: u32,
    pub height: u32,
}

/// Supported buffer element types
#[derive(Clone, Copy, Debug)]
pub enum BufferElementType {
    U32,  // For Conway's Game of Life, etc.
    F32,  // For continuous data
    I32,  // For signed integer data
}

/// 2D data plane visualization component
///
/// Generic visualizer that supports both CPU data (Vec<f32>) and direct GPU buffer access
/// for maximum efficiency. GPU buffers avoid expensive GPU→CPU→GPU transfers.
pub struct CutPlane2D {
    // Configuration
    enabled: bool,
    mode: VisualizationMode,
    filter_mode: FilterMode,
    last_filter_mode: FilterMode, // Track changes

    // View controls
    zoom: f32,
    pan: [f32; 2],

    // Data source (CPU or GPU)
    data_source: Option<DataSource>,
    // CPU data dimensions (for proper size validation)
    cpu_data_dimensions: Option<(u32, u32)>,

    // Rendering - using separate visualization system
    material: Option<VisualizationMaterial>,

    // Display position in 3D space
    position: Vector3<f32>,
    size: f32,

    // Update flags
    needs_material_update: bool,
    needs_scene_object_update: bool,
    needs_filter_update: bool, // Track filter changes separately
}

impl CutPlane2D {
    /// Create a new 2D data plane visualization
    pub fn new() -> Self {
        Self {
            enabled: true,
            mode: VisualizationMode::Heatmap,
            filter_mode: FilterMode::Sharp, // Default to sharp for discrete data like Conway's Game of Life
            last_filter_mode: FilterMode::Sharp,
            zoom: 1.0,
            pan: [0.0, 0.0],
            data_source: None,
            cpu_data_dimensions: None,
            material: None,
            position: Vector3::new(0.0, 0.0, 0.0),
            size: 2.0,
            needs_material_update: true,
            needs_scene_object_update: true,
            needs_filter_update: false,
        }
    }

    /// Set CPU data for visualization (traditional approach)
    pub fn update_data(&mut self, data: Vec<f32>, width: u32, height: u32) {
        // Store data with dimensions for accurate size validation
        self.data_source = Some(DataSource::CpuData(data));
        // Store dimensions for CPU data in a compatible way
        self.cpu_data_dimensions = Some((width, height));
        self.needs_material_update = true;
        self.needs_scene_object_update = true;
    }

    /// Set GPU buffer for direct visualization (high-performance approach)
    /// This avoids expensive GPU→CPU→GPU transfers
    pub fn update_gpu_buffer(&mut self, buffer: Arc<Buffer>, format: BufferFormat) {
        self.data_source = Some(DataSource::GpuBuffer { buffer, format });
        self.needs_material_update = true;
        self.needs_scene_object_update = true;
    }

    /// Convenience method for Conway's Game of Life and similar u32 grid simulations
    pub fn update_u32_buffer(&mut self, buffer: Arc<Buffer>, width: u32, height: u32) {
        let format = BufferFormat {
            element_type: BufferElementType::U32,
            width,
            height,
        };
        self.update_gpu_buffer(buffer, format);
    }

    /// Get data dimensions
    pub fn get_dimensions(&self) -> (u32, u32) {
        match &self.data_source {
            Some(DataSource::CpuData(_data)) => {
                // Use stored dimensions for CPU data to prevent size mismatches
                self.cpu_data_dimensions.unwrap_or((64, 64)) // Fallback to default if not set
            }
            Some(DataSource::GpuBuffer { format, .. }) => (format.width, format.height),
            None => (0, 0),
        }
    }

    /// Set position of the visualization plane in 3D space
    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    /// Set size of the visualization plane
    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    /// Set texture filtering mode (Sharp vs Smooth)
    pub fn set_filter_mode(&mut self, filter_mode: FilterMode) {
        if self.filter_mode != filter_mode {
            self.filter_mode = filter_mode;
            self.needs_filter_update = true;
            // For CPU materials, we need to recreate with new filtering
            if matches!(self.data_source, Some(DataSource::CpuData(_))) {
                self.needs_material_update = true;
            }
        }
    }

    /// Get current filter mode
    pub fn get_filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    /// Get current position
    pub fn get_position(&self) -> Vector3<f32> {
        self.position
    }

    /// Get current size
    pub fn get_size(&self) -> f32 {
        self.size
    }

    /// Get visualization material for rendering
    pub fn get_material(&self) -> Option<&VisualizationMaterial> {
        self.material.as_ref()
    }

    /// Convert to VisualizationPlane for rendering
    pub fn to_visualization_plane(&self) -> Option<crate::gfx::rendering::VisualizationPlane> {
        if let Some(material) = &self.material {
            // Get buffer reference if using GPU data
            let data_buffer = match &self.data_source {
                Some(DataSource::GpuBuffer { buffer, .. }) => Some((**buffer).clone()),
                _ => None,
            };

            Some(crate::gfx::rendering::VisualizationPlane {
                position: self.position,
                size: cgmath::Vector3::new(self.size, self.size, self.size),
                material: material.clone(),
                data_buffer,  // Pass GPU buffer directly to renderer!
                texture: None,
            })
        } else {
            None
        }
    }

    /// Update material based on current data source
    fn update_material(&mut self, device: &Device, queue: &Queue) {
        let Some(data_source) = &self.data_source else {
            return;
        };

        match data_source {
            DataSource::CpuData(data) => {
                // Traditional CPU data path - process and create texture
                let (width, height) = self.get_dimensions();
                if width == 0 || height == 0 {
                    return;
                }

                let processed_data = match self.mode {
                    VisualizationMode::Heatmap => self.apply_heatmap_coloring(data),
                    VisualizationMode::Grid => self.apply_grid_pattern(data, width, height),
                    VisualizationMode::Points => self.apply_points_visualization(data),
                };

                let wgpu_filter = match self.filter_mode {
                    FilterMode::Sharp => wgpu::FilterMode::Nearest,
                    FilterMode::Smooth => wgpu::FilterMode::Linear,
                };

                self.material = Some(VisualizationMaterial::from_2d_data_with_filter(
                    device,
                    queue,
                    &processed_data,
                    width,
                    height,
                    "2D Data Plane Material",
                    wgpu_filter,
                ));
            }
            DataSource::GpuBuffer { buffer, format } => {
                // High-performance GPU buffer path - create material that references buffer directly
                self.material = Some(VisualizationMaterial::from_gpu_buffer(
                    device,
                    queue,
                    buffer.clone(),
                    *format,
                    self.mode,
                    "GPU Buffer Material",
                ));
            }
        }

        self.needs_material_update = false;
    }

    /// Apply heatmap coloring to 2D data
    fn apply_heatmap_coloring(&self, data: &[f32]) -> Vec<f32> {
        // Normalize data and return as-is for VisualizationMaterial to handle
        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let range = max_val - min_val;

        if range > 0.0 {
            data.iter()
                .map(|&value| (value - min_val) / range)
                .collect()
        } else {
            vec![0.5; data.len()] // All same value - use middle gray
        }
    }

    /// Apply grid pattern to 2D data
    fn apply_grid_pattern(&self, data: &[f32], width: u32, height: u32) -> Vec<f32> {
        let mut result = Vec::with_capacity(data.len());
        let checker_size = 8;

        // Normalize input data first
        let normalized_data = self.apply_heatmap_coloring(data);

        for (i, &value) in normalized_data.iter().enumerate() {
            let x = i as u32 % width;
            let y = i as u32 / width;

            let checker_x = (x / checker_size) % 2;
            let checker_y = (y / checker_size) % 2;
            let is_checker = (checker_x + checker_y) % 2 == 0;

            // Mix checkerboard pattern with data
            let base_intensity = if is_checker { 0.2 } else { 0.8 };
            let data_influence = 0.3;
            let final_value = base_intensity * (1.0 - data_influence) + value * data_influence;

            result.push(final_value);
        }

        result
    }

    /// Apply points visualization to 2D data
    fn apply_points_visualization(&self, data: &[f32]) -> Vec<f32> {
        // Normalize input data first
        let normalized_data = self.apply_heatmap_coloring(data);

        normalized_data
            .iter()
            .map(|&value| {
                if value > 0.8 {
                    1.0 // Bright points for high values
                } else {
                    value * 0.25 // Dim background
                }
            })
            .collect()
    }

    /// Render the visualization display
    fn render_visualization(&self, ui: &Ui) {
        if self.data_source.is_some() {
            ui.text("2D Data Visualization:");
            ui.separator();

            // Display data information
            let (width, height) = self.get_dimensions();
            ui.text(&format!("Data size: {}x{}", width, height));
            ui.text(&format!("Mode: {}", self.mode.as_str()));
            ui.text(&format!(
                "Position: ({:.2}, {:.2}, {:.2})",
                self.position.x, self.position.y, self.position.z
            ));
            ui.text(&format!("Size: {:.2}", self.size));

            ui.spacing();

            // Display placeholder for visualization
            ui.child_window("data_plane_display")
                .size([350.0, 350.0])
                .border(true)
                .build(|| {
                    let (width, height) = self.get_dimensions();
                    ui.text(&format!("Data: {}x{} values", width, height));
                    ui.text(&format!("Zoom: {:.1}x", self.zoom));
                    ui.text(&format!("Pan: ({:.2}, {:.2})", self.pan[0], self.pan[1]));

                    ui.spacing();
                    ui.text("[2D Data Plane Visualization]");
                    ui.text("(Using separate visualization renderer)");

                    // Show visualization mode info
                    match self.mode {
                        VisualizationMode::Heatmap => {
                            ui.spacing();
                            ui.text("Heatmap coloring applied");
                        }
                        VisualizationMode::Grid => {
                            ui.spacing();
                            ui.text("Grid pattern overlay applied");
                        }
                        VisualizationMode::Points => {
                            ui.spacing();
                            ui.text("Points visualization applied");
                        }
                    }
                });
        } else {
            ui.text("No data loaded");
            ui.text("Use update_data() to provide 2D data for visualization");
        }
    }
}

impl VisualizationComponent for CutPlane2D {
    fn initialize(&mut self, device: Option<&Device>, queue: Option<&Queue>) {
        // Generate some test data if none is provided
        if self.data_source.is_none() {
            self.generate_test_2d_data();
        }

        // Create initial material if device/queue available
        if let (Some(device), Some(queue)) = (device, queue) {
            self.update_material(device, queue);
        }
    }

    fn update(&mut self, _delta_time: f32, device: Option<&Device>, queue: Option<&Queue>) {
        // Update material if needed and resources available
        if self.needs_material_update {
            if let (Some(device), Some(queue)) = (device, queue) {
                self.update_material(device, queue);
            }
        }
        
        // Update filter mode for GPU materials (only when changed)
        if self.needs_filter_update && self.filter_mode != self.last_filter_mode {
            if let (Some(material), Some(queue)) = (&self.material, queue) {
                material.update_filter_mode(queue, self.filter_mode);
                self.last_filter_mode = self.filter_mode;
                self.needs_filter_update = false;
                println!("🎨 Filter mode updated to: {:?}", self.filter_mode); // Debug output
            }
        }
    }

    fn render_ui(&mut self, ui: &Ui) {
        // Simplified controls for 2D data visualization
        ui.checkbox("Enabled", &mut self.enabled);

        if !self.enabled {
            return;
        }

        ui.separator();

        // Visualization mode controls
        let mut mode_changed = false;
        if ui.radio_button_bool("Heatmap", self.mode == VisualizationMode::Heatmap) {
            self.mode = VisualizationMode::Heatmap;
            mode_changed = true;
        }
        if ui.radio_button_bool("Grid", self.mode == VisualizationMode::Grid) {
            self.mode = VisualizationMode::Grid;
            mode_changed = true;
        }
        if ui.radio_button_bool("Points", self.mode == VisualizationMode::Points) {
            self.mode = VisualizationMode::Points;
            mode_changed = true;
        }

        ui.separator();

        // View controls
        ui.slider("Zoom", 0.1, 5.0, &mut self.zoom);
        ui.slider_config("Pan X", -1.0, 1.0).build(&mut self.pan[0]);
        ui.slider_config("Pan Y", -1.0, 1.0).build(&mut self.pan[1]);

        ui.separator();

        // 3D positioning
        ui.slider_config("Position X", -5.0, 5.0)
            .build(&mut self.position.x);
        ui.slider_config("Position Y", -5.0, 5.0)
            .build(&mut self.position.y);
        ui.slider_config("Position Z", -5.0, 5.0)
            .build(&mut self.position.z);
        ui.slider_config("Size", 0.1, 10.0).build(&mut self.size);

        ui.separator();

        // Render the visualization display
        self.render_visualization(ui);

        // Update material if mode changed
        if mode_changed {
            self.needs_material_update = true;
        }
    }

    fn name(&self) -> &str {
        "2D Data Plane"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn set_data(&mut self, data: &[f32], dimensions: (u32, u32, u32)) {
        // Convert 3D data to 2D for compatibility - just take first slice
        if dimensions.0 > 0 && dimensions.1 > 0 && dimensions.2 > 0 {
            let slice_size = (dimensions.0 * dimensions.1) as usize;
            if data.len() >= slice_size {
                let slice_data = data[0..slice_size].to_vec();
                self.update_data(slice_data, dimensions.0, dimensions.1);
            }
        }
    }

    fn get_ui_position(&self) -> (f32, f32) {
        (20.0, 20.0)
    }

    fn get_ui_size(&self) -> (f32, f32) {
        (400.0, 600.0)
    }

    fn update_scene_objects(&mut self, _scene: &mut Scene) {
        // NOTE: We no longer create scene objects for visualization planes
        // The new VisualizationRenderer handles rendering directly through to_visualization_plane()
        // This prevents duplicate white planes from appearing in the scene

        if self.needs_scene_object_update {
            self.needs_scene_object_update = false;
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update_material_texture(&mut self, scene: &mut Scene, device: &Device, queue: &Queue) {
        if !self.enabled {
            return;
        }

        if !self.needs_material_update {
            return;
        }

        let Some(data_source) = &self.data_source else {
            return;
        };

        // Only handle CPU data in this legacy method
        let DataSource::CpuData(data) = data_source else {
            return; // GPU buffers don't need scene material updates
        };

        let material_name = "data_plane_material";

        if let Some(scene_material) = scene
            .get_material_manager_mut()
            .get_material_mut(&material_name.to_string())
        {
            let (width, height) = self.get_dimensions();
            
            // Process 2D data based on visualization mode
            let processed_data = match self.mode {
                VisualizationMode::Heatmap => self.apply_heatmap_coloring(data),
                VisualizationMode::Grid => self.apply_grid_pattern(data, width, height),
                VisualizationMode::Points => self.apply_points_visualization(data),
            };

            // Convert f32 data to RGBA8
            let rgba_data: Vec<u8> = processed_data
                .iter()
                .flat_map(|&value| {
                    let normalized = value.clamp(0.0, 1.0);
                    let color_val = (normalized * 255.0) as u8;
                    [color_val, color_val, color_val, 255u8] // Grayscale
                })
                .collect();

            // Create texture from RGBA data
            let texture = TextureResource::create_from_rgba_data(
                device,
                queue,
                &rgba_data,
                width,
                height,
                "2D Data Plane Texture",
            );

            // Set the texture on the scene material
            scene_material.set_texture(texture);
            scene_material.base_color = [1.0, 1.0, 1.0, 1.0]; // White base color to show texture

            self.needs_material_update = false;
        }
    }
}

impl CutPlane2D {
    /// Generate 2D test data for demonstration purposes
    fn generate_test_2d_data(&mut self) {
        let width = 64u32;
        let height = 64u32;

        let mut data = Vec::with_capacity((width * height) as usize);

        // Generate a 2D pattern: concentric circles
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let max_radius = (width.min(height) as f32) / 2.0;

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();

                // Create concentric circles pattern
                let normalized_distance = distance / max_radius;
                let value = ((normalized_distance * 8.0).sin() + 1.0) / 2.0;

                data.push(value);
            }
        }

        self.data_source = Some(DataSource::CpuData(data));
    }
}
