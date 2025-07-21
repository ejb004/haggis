//! 2D Data Plane Visualization Component
//!
//! Generic 2D data visualizer that accepts 2D data arrays directly from the user.
//! No hardcoded 3D slicing logic - purely for displaying 2D data.

use super::traits::VisualizationComponent;
use super::ui::cut_plane_controls::VisualizationMode;
use super::rendering::VisualizationMaterial;
use crate::gfx::{scene::Scene, resources::texture_resource::TextureResource};
use imgui::Ui;
use wgpu::{Device, Queue};
use cgmath::Vector3;

/// 2D data plane visualization component
///
/// Generic visualizer for 2D data arrays. User provides 2D data directly,
/// and this component renders it as a textured plane using the dedicated
/// visualization rendering system.
pub struct CutPlane2D {
    // Configuration
    enabled: bool,
    mode: VisualizationMode,
    
    // View controls
    zoom: f32,
    pan: [f32; 2],
    
    // Data - simplified to 2D only
    data_2d: Option<Vec<f32>>,
    data_width: u32,
    data_height: u32,
    
    // Rendering - using separate visualization system
    material: Option<VisualizationMaterial>,
    
    // Display position in 3D space
    position: Vector3<f32>,
    size: f32,
    
    // Update flags
    needs_material_update: bool,
    needs_scene_object_update: bool,
}

impl CutPlane2D {
    /// Create a new 2D data plane visualization
    pub fn new() -> Self {
        Self {
            enabled: true,
            mode: VisualizationMode::Heatmap,
            zoom: 1.0,
            pan: [0.0, 0.0],
            data_2d: None,
            data_width: 0,
            data_height: 0,
            material: None,
            position: Vector3::new(0.0, 0.0, 0.0),
            size: 2.0,
            needs_material_update: true,
            needs_scene_object_update: true,
        }
    }

    /// Set 2D data for visualization
    pub fn update_data(&mut self, data: Vec<f32>, width: u32, height: u32) {
        self.data_2d = Some(data);
        self.data_width = width;
        self.data_height = height;
        self.needs_material_update = true;
        self.needs_scene_object_update = true;
    }

    /// Set position of the visualization plane in 3D space
    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    /// Set size of the visualization plane
    pub fn set_size(&mut self, size: f32) {
        self.size = size;
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
            Some(crate::gfx::rendering::VisualizationPlane {
                position: self.position,
                // Pass size directly like regular objects - will be handled by uniform scale
                size: cgmath::Vector3::new(self.size, self.size, self.size), 
                material: material.clone(),
                data_buffer: None,
                texture: None,
            })
        } else {
            None
        }
    }

    /// Update material from 2D data
    fn update_material(&mut self, device: &Device, queue: &Queue) {
        let Some(data) = &self.data_2d else { 
            return;
        };
        
        if self.data_width == 0 || self.data_height == 0 {
            return;
        }
        
        // Process 2D data based on visualization mode
        let processed_data = match self.mode {
            VisualizationMode::Heatmap => self.apply_heatmap_coloring(data),
            VisualizationMode::Grid => self.apply_grid_pattern(data),
            VisualizationMode::Points => self.apply_points_visualization(data),
        };

        // Create material from processed 2D data
        self.material = Some(VisualizationMaterial::from_2d_data(
            device,
            queue,
            &processed_data,
            self.data_width,
            self.data_height,
            "2D Data Plane Material",
        ));
        
        self.needs_material_update = false;
    }

    /// Apply heatmap coloring to 2D data
    fn apply_heatmap_coloring(&self, data: &[f32]) -> Vec<f32> {
        // Normalize data and return as-is for VisualizationMaterial to handle
        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let range = max_val - min_val;
        
        if range > 0.0 {
            data.iter().map(|&value| (value - min_val) / range).collect()
        } else {
            vec![0.5; data.len()] // All same value - use middle gray
        }
    }

    /// Apply grid pattern to 2D data
    fn apply_grid_pattern(&self, data: &[f32]) -> Vec<f32> {
        let mut result = Vec::with_capacity(data.len());
        let checker_size = 8;
        
        // Normalize input data first
        let normalized_data = self.apply_heatmap_coloring(data);
        
        for (i, &value) in normalized_data.iter().enumerate() {
            let x = i as u32 % self.data_width;
            let y = i as u32 / self.data_width;
            
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
        
        normalized_data.iter().map(|&value| {
            if value > 0.8 {
                1.0 // Bright points for high values
            } else {
                value * 0.25 // Dim background
            }
        }).collect()
    }
    
    /// Render the visualization display
    fn render_visualization(&self, ui: &Ui) {
        if self.data_2d.is_some() {
            ui.text("2D Data Visualization:");
            ui.separator();
            
            // Display data information
            ui.text(&format!(
                "Data size: {}x{}", 
                self.data_width, 
                self.data_height
            ));
            ui.text(&format!("Mode: {}", self.mode.as_str()));
            ui.text(&format!("Position: ({:.2}, {:.2}, {:.2})", 
                    self.position.x, self.position.y, self.position.z));
            ui.text(&format!("Size: {:.2}", self.size));
            
            ui.spacing();
            
            // Display placeholder for visualization
            ui.child_window("data_plane_display")
                .size([350.0, 350.0])
                .border(true)
                .build(|| {
                    ui.text(&format!("Data: {}x{} values", self.data_width, self.data_height));
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
        if self.data_2d.is_none() {
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
        ui.slider_config("Position X", -5.0, 5.0).build(&mut self.position.x);
        ui.slider_config("Position Y", -5.0, 5.0).build(&mut self.position.y);
        ui.slider_config("Position Z", -5.0, 5.0).build(&mut self.position.z);
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

    fn update_material_texture(&mut self, scene: &mut Scene, device: &Device, queue: &Queue) {
        if !self.enabled {
            return;
        }
        
        if !self.needs_material_update {
            return;
        }
        
        if self.data_2d.is_none() {
            return;
        }

        let material_name = "data_plane_material";
        
        if let Some(scene_material) = scene.get_material_manager_mut().get_material_mut(&material_name.to_string()) {
            if let Some(data) = &self.data_2d {
                
                // Process 2D data based on visualization mode
                let processed_data = match self.mode {
                    VisualizationMode::Heatmap => self.apply_heatmap_coloring(data),
                    VisualizationMode::Grid => self.apply_grid_pattern(data),
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
                    self.data_width,
                    self.data_height,
                    "2D Data Plane Texture",
                );
                
                // Set the texture on the scene material
                scene_material.set_texture(texture);
                scene_material.base_color = [1.0, 1.0, 1.0, 1.0]; // White base color to show texture
                
                self.needs_material_update = false;
            }
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
        
        self.data_2d = Some(data);
        self.data_width = width;
        self.data_height = height;
    }
}