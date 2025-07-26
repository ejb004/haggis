//! # Viewport Navigation Gizmo
//!
//! This module provides a 3D viewport navigation gizmo similar to those found in CAD software
//! like Fusion 360, Blender, or Maya. It displays a small cube in the corner of the viewport
//! that shows the current camera orientation and allows clicking on faces to snap to
//! orthographic views.

use crate::gfx::gizmos::traits::Gizmo;
use crate::gfx::scene::Scene;
use crate::gfx::geometry::primitives::generate_cube;
use cgmath::{Vector3, Vector4, Matrix4, Point3, InnerSpace, EuclideanSpace, SquareMatrix};
use imgui::Ui;
use std::any::Any;
use wgpu::{Device, Queue};

/// Size of the viewport gizmo cube in screen space
const GIZMO_SIZE: f32 = 0.08;

/// Distance from screen edges (in normalized coordinates)
const MARGIN: f32 = 0.05;

/// Standard orthographic view directions
#[derive(Debug, Clone, Copy)]
pub enum ViewDirection {
    Front,   // +Y
    Back,    // -Y
    Right,   // +X
    Left,    // -X
    Top,     // +Z
    Bottom,  // -Z
}

impl ViewDirection {
    /// Get the camera position for this view direction
    pub fn get_camera_position(&self, distance: f32) -> Vector3<f32> {
        match self {
            ViewDirection::Front => Vector3::new(0.0, distance, 0.0),
            ViewDirection::Back => Vector3::new(0.0, -distance, 0.0),
            ViewDirection::Right => Vector3::new(distance, 0.0, 0.0),
            ViewDirection::Left => Vector3::new(-distance, 0.0, 0.0),
            ViewDirection::Top => Vector3::new(0.0, 0.0, distance),
            ViewDirection::Bottom => Vector3::new(0.0, 0.0, -distance),
        }
    }

    /// Get the up vector for this view direction
    pub fn get_up_vector(&self) -> Vector3<f32> {
        match self {
            ViewDirection::Front | ViewDirection::Back | 
            ViewDirection::Right | ViewDirection::Left => Vector3::new(0.0, 0.0, 1.0), // Z-up
            ViewDirection::Top => Vector3::new(0.0, 1.0, 0.0), // Y forward when looking down
            ViewDirection::Bottom => Vector3::new(0.0, -1.0, 0.0), // Y backward when looking up
        }
    }

    /// Get the face color for this direction
    pub fn get_face_color(&self) -> [f32; 3] {
        match self {
            ViewDirection::Right => [1.0, 0.2, 0.2],  // Red for +X
            ViewDirection::Left => [0.7, 0.1, 0.1],   // Dark red for -X
            ViewDirection::Front => [0.2, 1.0, 0.2],  // Green for +Y  
            ViewDirection::Back => [0.1, 0.7, 0.1],   // Dark green for -Y
            ViewDirection::Top => [0.2, 0.2, 1.0],    // Blue for +Z
            ViewDirection::Bottom => [0.1, 0.1, 0.7], // Dark blue for -Z
        }
    }

    /// Get the label for this direction
    pub fn get_label(&self) -> &'static str {
        match self {
            ViewDirection::Front => "F",
            ViewDirection::Back => "B", 
            ViewDirection::Right => "R",
            ViewDirection::Left => "L",
            ViewDirection::Top => "T",
            ViewDirection::Bottom => "Bot",
        }
    }
}

/// Viewport navigation gizmo that provides camera orientation feedback and view switching
pub struct ViewportGizmo {
    /// Whether the gizmo is enabled
    enabled: bool,
    
    /// Object indices for the gizmo cube faces
    face_object_indices: Vec<usize>,
    
    /// Current camera distance for view switching
    camera_distance: f32,
    
    /// Position of the gizmo in viewport space (0.0 to 1.0)
    viewport_position: (f32, f32),
    
    /// Size of the gizmo in viewport space
    viewport_size: f32,
    
    /// Whether to show face labels
    show_labels: bool,
    
    /// Counter for generating unique object IDs
    id_counter: usize,
    
    /// Target position when animating view changes
    target_view: Option<ViewDirection>,
    
    /// Animation progress (0.0 to 1.0)
    animation_progress: f32,
    
    /// Animation speed
    animation_speed: f32,
}

impl ViewportGizmo {
    /// Create a new viewport gizmo
    pub fn new() -> Self {
        Self {
            enabled: true,
            face_object_indices: Vec::new(),
            camera_distance: 8.0,
            viewport_position: (0.9, 0.9), // Top-right corner (90% across, 90% down)
            viewport_size: 0.15, // Larger for better visibility
            show_labels: true,
            id_counter: 0,
            target_view: None,
            animation_progress: 0.0,
            animation_speed: 3.0, // 3 seconds for view transition
        }
    }
    
    /// Generate a unique object ID
    fn generate_id(&mut self) -> String {
        self.id_counter += 1;
        format!("viewport_gizmo_{}", self.id_counter)
    }
    
    /// Create the gizmo cube faces
    fn create_gizmo_faces(&mut self, scene: &mut Scene) {
        // Clear existing faces
        self.cleanup_faces(scene);
        
        let views = [
            ViewDirection::Front,
            ViewDirection::Back,
            ViewDirection::Right,
            ViewDirection::Left,
            ViewDirection::Top,
            ViewDirection::Bottom,
        ];
        
        for view in &views {
            let face_geometry = generate_cube();
            let object_name = self.generate_id();
            scene.add_procedural_object(face_geometry, &object_name);
            let object_index = scene.get_object_count() - 1;
            
            // Create material for this face
            let color = view.get_face_color();
            let material_name = format!("{}_material", object_name);
            scene.add_material_rgb(
                &material_name,
                color[0], color[1], color[2],
                0.1, // Slightly metallic
                0.4, // Medium roughness
            );
            
            // Assign material and set initial properties
            scene.assign_material_to_object(object_index, &material_name);
            
            if let Some(object) = scene.get_object_mut(object_index) {
                // Start with a small scale - will be updated in render loop
                object.set_scale(0.02);
                object.visible = true;
            }
            
            self.face_object_indices.push(object_index);
        }
    }
    
    /// Clean up existing face objects
    fn cleanup_faces(&mut self, scene: &mut Scene) {
        // Remove in reverse order to maintain indices
        for &index in self.face_object_indices.iter().rev() {
            if index < scene.objects.len() {
                scene.objects.remove(index);
            }
        }
        self.face_object_indices.clear();
    }
    
    /// Update gizmo position and orientation based on current camera
    fn update_gizmo_transform(&self, scene: &mut Scene) {
        if !self.enabled || self.face_object_indices.is_empty() {
            // Hide all faces when disabled
            for &index in &self.face_object_indices {
                if let Some(object) = scene.get_object_mut(index) {
                    object.visible = false;
                }
            }
            return;
        }
        
        let camera = &scene.camera_manager.camera;
        
        // TODO: Get actual screen size from render engine
        // For now, use a reasonable default
        let screen_size = (1920.0, 1080.0);
        
        // Calculate gizmo position in world space that corresponds to screen position
        let gizmo_world_pos = self.calculate_gizmo_world_position(camera, screen_size);
        
        // Calculate scale based on distance to maintain consistent screen size
        let distance_to_camera = (gizmo_world_pos - camera.eye).magnitude();
        let screen_scale = distance_to_camera * self.viewport_size * 0.02; // Smaller multiplier for better visibility
        
        // For now, position all faces at the same location to create a single cube
        // In a full implementation, you'd separate them to show individual faces
        let face_positions = vec![gizmo_world_pos; 6]; // All faces at same position for now
        
        // Update each face
        for (i, &object_index) in self.face_object_indices.iter().enumerate() {
            if let Some(object) = scene.get_object_mut(object_index) {
                if i < face_positions.len() {
                    object.set_translation(face_positions[i]);
                } else {
                    object.set_translation(gizmo_world_pos);
                }
                
                object.set_scale(screen_scale);
                object.visible = true;
            }
        }
    }
    
    /// Calculate positions for each face of the cube
    fn calculate_face_positions(&self, center: Vector3<f32>, scale: f32) -> Vec<Vector3<f32>> {
        let half_scale = scale * 0.5;
        vec![
            center + Vector3::new(0.0, half_scale, 0.0),  // Front (+Y)
            center + Vector3::new(0.0, -half_scale, 0.0), // Back (-Y)
            center + Vector3::new(half_scale, 0.0, 0.0),  // Right (+X)
            center + Vector3::new(-half_scale, 0.0, 0.0), // Left (-X)
            center + Vector3::new(0.0, 0.0, half_scale),  // Top (+Z)
            center + Vector3::new(0.0, 0.0, -half_scale), // Bottom (-Z)
        ]
    }
    
    /// Calculate world position for the gizmo based on viewport position
    fn calculate_gizmo_world_position(&self, camera: &crate::gfx::camera::orbit_camera::OrbitCamera, screen_size: (f32, f32)) -> Vector3<f32> {
        // Convert viewport position (0.0 to 1.0) to screen coordinates
        let screen_x = self.viewport_position.0 * screen_size.0;
        let screen_y = (1.0 - self.viewport_position.1) * screen_size.1; // Flip Y for screen coords
        
        // Convert to normalized device coordinates (-1 to 1)
        let ndc_x = (2.0 * screen_x) / screen_size.0 - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y) / screen_size.1;
        
        // Use a fixed depth in NDC space (close to camera but not clipped)
        let ndc_depth = -0.2; // Between -1 (near) and 1 (far)
        
        // Get camera matrices
        let eye = Point3::from_vec(camera.eye);
        let target = Point3::from_vec(camera.target);
        let view_matrix = Matrix4::look_at_rh(eye, target, camera.up);
        let proj_matrix = cgmath::perspective(camera.fovy, camera.aspect, camera.znear, camera.zfar);
        
        // Calculate inverse view-projection matrix
        let view_proj_matrix = proj_matrix * view_matrix;
        
        if let Some(inv_view_proj) = view_proj_matrix.invert() {
            // Convert NDC to world space
            let ndc_point = Vector4::new(ndc_x, ndc_y, ndc_depth, 1.0);
            let world_point = inv_view_proj * ndc_point;
            
            // Convert from homogeneous coordinates
            Vector3::new(
                world_point.x / world_point.w,
                world_point.y / world_point.w,
                world_point.z / world_point.w,
            )
        } else {
            // Fallback if matrix inversion fails
            camera.eye + Vector3::new(2.0, 0.0, 2.0)
        }
    }
    
    /// Handle view switching when a face is clicked
    pub fn switch_to_view(&mut self, view: ViewDirection) {
        self.target_view = Some(view);
        self.animation_progress = 0.0;
    }
    
    /// Update view animation
    fn update_view_animation(&mut self, delta_time: f32, scene: &mut Scene) {
        if let Some(target_view) = self.target_view {
            self.animation_progress += delta_time * self.animation_speed;
            
            if self.animation_progress >= 1.0 {
                // Animation complete - snap to final position
                self.animation_progress = 1.0;
                self.snap_camera_to_view(scene, target_view);
                self.target_view = None;
            } else {
                // Interpolate camera position
                self.interpolate_camera_to_view(scene, target_view, self.animation_progress);
            }
        }
    }
    
    /// Snap camera to a specific view direction
    fn snap_camera_to_view(&self, scene: &mut Scene, view: ViewDirection) {
        let camera = &mut scene.camera_manager.camera;
        let target = camera.target; // Keep current target
        
        let new_position = target + view.get_camera_position(self.camera_distance);
        
        // Update camera with new position
        camera.eye = new_position;
        camera.distance = self.camera_distance;
        
        // Calculate new pitch and yaw based on position
        let direction = (target - new_position).normalize();
        camera.yaw = direction.y.atan2(direction.x).to_degrees();
        camera.pitch = direction.z.asin().to_degrees();
    }
    
    /// Interpolate camera position during animation
    fn interpolate_camera_to_view(&self, scene: &mut Scene, view: ViewDirection, progress: f32) {
        let camera = &mut scene.camera_manager.camera;
        let target = camera.target;
        
        let start_pos = camera.eye;
        let end_pos = target + view.get_camera_position(self.camera_distance);
        
        // Use smooth interpolation (ease-in-out)
        let smooth_progress = progress * progress * (3.0 - 2.0 * progress);
        let new_position = start_pos + (end_pos - start_pos) * smooth_progress;
        
        camera.eye = new_position;
        
        // Update camera parameters
        let direction = (target - new_position).normalize();
        camera.yaw = direction.y.atan2(direction.x).to_degrees();
        camera.pitch = direction.z.asin().to_degrees();
        camera.distance = (new_position - target).magnitude();
    }
}

impl Gizmo for ViewportGizmo {
    fn initialize(&mut self, scene: &mut Scene, _device: Option<&Device>, _queue: Option<&Queue>) {
        self.create_gizmo_faces(scene);
    }
    
    fn update(&mut self, delta_time: f32, scene: &mut Scene, _device: Option<&Device>, _queue: Option<&Queue>) {
        if !self.enabled {
            // Hide all faces when disabled
            for &index in &self.face_object_indices {
                if let Some(object) = scene.get_object_mut(index) {
                    object.visible = false;
                }
            }
            return;
        }
        
        // Update view animation
        self.update_view_animation(delta_time, scene);
        
        // Update gizmo transform
        self.update_gizmo_transform(scene);
    }
    
    fn render_ui(&mut self, ui: &Ui, _scene: &mut Scene) {
        ui.window("Viewport Gizmo")
            .size([300.0, 250.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Enable Viewport Gizmo", &mut self.enabled);
                ui.checkbox("Show Labels", &mut self.show_labels);
                
                ui.separator();
                
                ui.slider("Camera Distance", 2.0, 20.0, &mut self.camera_distance);
                ui.slider("Animation Speed", 0.5, 10.0, &mut self.animation_speed);
                
                ui.separator();
                ui.text("Quick View Buttons:");
                
                let views = [
                    ("Front", ViewDirection::Front),
                    ("Back", ViewDirection::Back),
                    ("Right", ViewDirection::Right),
                    ("Left", ViewDirection::Left),
                    ("Top", ViewDirection::Top),
                    ("Bottom", ViewDirection::Bottom),
                ];
                
                for (i, (label, view)) in views.iter().enumerate() {
                    if ui.button(label) {
                        self.switch_to_view(*view);
                    }
                    
                    // Three buttons per row
                    if i % 3 != 2 && i < views.len() - 1 {
                        ui.same_line();
                    }
                }
                
                ui.separator();
                ui.text("Position Controls:");
                ui.slider("X Position", 0.0, 1.0, &mut self.viewport_position.0);
                ui.slider("Y Position", 0.0, 1.0, &mut self.viewport_position.1);
                ui.slider("Size", 0.02, 0.2, &mut self.viewport_size);
            });
    }
    
    fn name(&self) -> &str {
        "Viewport Gizmo"
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    fn cleanup(&mut self, scene: &mut Scene) {
        self.cleanup_faces(scene);
    }
    
    fn get_priority(&self) -> i32 {
        1000 // Render viewport gizmo last (on top)
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for ViewportGizmo {
    fn default() -> Self {
        Self::new()
    }
}