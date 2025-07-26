//! # Camera Gizmo
//!
//! This module provides a gizmo for visualizing camera positions in 3D space.
//! It displays the current camera position and optionally tracks camera movement history.

use crate::gfx::gizmos::traits::Gizmo;
use crate::gfx::scene::Scene;
use crate::gfx::geometry::primitives::{generate_cube, generate_sphere};
use cgmath::{InnerSpace, Vector3};
use imgui::Ui;
use std::any::Any;
use std::collections::VecDeque;
use wgpu::{Device, Queue};

/// Maximum number of historical camera positions to track
const MAX_HISTORY_SIZE: usize = 100;

/// Distance threshold for adding new positions to history (to avoid spam)
const MIN_MOVEMENT_DISTANCE: f32 = 0.1;

/// Camera gizmo that shows camera positions as 3D objects
pub struct CameraGizmo {
    /// Whether the gizmo is currently enabled
    enabled: bool,
    
    /// Whether to show the current camera position
    show_current: bool,
    
    /// Whether to show camera movement history trail
    show_history: bool,
    
    /// Current camera position
    current_position: Vector3<f32>,
    
    /// History of camera positions
    position_history: VecDeque<Vector3<f32>>,
    
    /// Object index for the current camera position gizmo
    current_gizmo_index: Option<usize>,
    
    /// Object indices for historical position gizmos
    history_gizmo_indices: Vec<usize>,
    
    /// Size of the gizmo objects
    gizmo_size: f32,
    
    /// Color for current position (RGB)
    current_color: [f32; 3],
    
    /// Color for history positions (RGB)
    history_color: [f32; 3],
    
    /// Counter for generating unique object IDs
    id_counter: usize,
}

impl CameraGizmo {
    /// Create a new camera gizmo
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_current: true,
            show_history: false,
            current_position: Vector3::new(0.0, 0.0, 0.0),
            position_history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            current_gizmo_index: None,
            history_gizmo_indices: Vec::new(),
            gizmo_size: 0.2,
            current_color: [1.0, 0.0, 0.0], // Red for current position
            history_color: [0.5, 0.5, 1.0], // Light blue for history
            id_counter: 0,
        }
    }
    
    /// Generate a unique object ID for gizmos
    fn generate_id(&mut self) -> String {
        self.id_counter += 1;
        format!("camera_gizmo_{}", self.id_counter)
    }
    
    /// Check if a position is far enough from the last recorded position
    fn should_add_to_history(&self, new_position: Vector3<f32>) -> bool {
        if let Some(last_pos) = self.position_history.back() {
            let distance = (new_position - last_pos).magnitude();
            distance >= MIN_MOVEMENT_DISTANCE
        } else {
            true
        }
    }
    
    /// Update the current camera position gizmo
    fn update_current_gizmo(&mut self, scene: &mut Scene) {
        if !self.show_current {
            // Remove current gizmo if it exists
            if let Some(index) = self.current_gizmo_index {
                self.remove_object_from_scene(scene, index);
                self.current_gizmo_index = None;
            }
            return;
        }
        
        // Create or update current position gizmo
        if self.current_gizmo_index.is_none() {
            let material_name = format!("camera_gizmo_current_{}", self.id_counter);
            self.create_material(scene, &material_name, self.current_color);
            
            let object_index = self.create_cube_object(scene, self.gizmo_size);
            self.set_object_material_and_position(scene, object_index, &material_name, self.current_position);
            
            // Scale the cube to the desired size
            if let Some(object) = scene.get_object_mut(object_index) {
                object.set_scale(self.gizmo_size);
            }
            
            self.current_gizmo_index = Some(object_index);
        } else if let Some(index) = self.current_gizmo_index {
            // Update position of existing gizmo
            self.update_object_position(scene, index, self.current_position);
            
            // Update scale if it changed
            if let Some(object) = scene.get_object_mut(index) {
                object.set_scale(self.gizmo_size);
            }
        }
    }
    
    /// Update history gizmos
    fn update_history_gizmos(&mut self, scene: &mut Scene) {
        if !self.show_history {
            // Remove all history gizmos (in reverse order to maintain indices)
            for &index in self.history_gizmo_indices.iter().rev() {
                self.remove_object_from_scene(scene, index);
            }
            self.history_gizmo_indices.clear();
            return;
        }
        
        // Remove excess gizmos if we have more than positions
        while self.history_gizmo_indices.len() > self.position_history.len() {
            if let Some(index) = self.history_gizmo_indices.pop() {
                self.remove_object_from_scene(scene, index);
            }
        }
        
        // Add new gizmos for new positions  
        while self.history_gizmo_indices.len() < self.position_history.len() {
            let material_name = format!("camera_gizmo_history_{}", self.id_counter);
            self.create_material(scene, &material_name, self.history_color);
            
            let object_index = self.create_sphere_object(scene, self.gizmo_size * 0.7); // Slightly smaller for history
            scene.assign_material_to_object(object_index, &material_name);
            
            // Scale the sphere to the desired size
            if let Some(object) = scene.get_object_mut(object_index) {
                object.set_scale(self.gizmo_size * 0.7);
            }
            
            self.history_gizmo_indices.push(object_index);
        }
        
        // Update positions of all history gizmos
        for (i, &index) in self.history_gizmo_indices.iter().enumerate() {
            if let Some(pos) = self.position_history.get(i) {
                self.update_object_position(scene, index, *pos);
                
                // Update scale
                if let Some(object) = scene.get_object_mut(index) {
                    object.set_scale(self.gizmo_size * 0.7);
                }
            }
        }
    }
    
    /// Create a cube object for the gizmo
    fn create_cube_object(&mut self, scene: &mut Scene, _size: f32) -> usize {
        let cube_geometry = generate_cube();
        let object_name = self.generate_id();
        scene.add_procedural_object(cube_geometry, &object_name);
        scene.get_object_count() - 1 // Return the index of the newly added object
    }
    
    /// Create a sphere object for the gizmo
    fn create_sphere_object(&mut self, scene: &mut Scene, _size: f32) -> usize {
        let sphere_geometry = generate_sphere(16, 8); // 16 longitude, 8 latitude segments
        let object_name = self.generate_id();
        scene.add_procedural_object(sphere_geometry, &object_name);
        scene.get_object_count() - 1 // Return the index of the newly added object
    }
    
    /// Create material with specified color
    fn create_material(&self, scene: &mut Scene, name: &str, color: [f32; 3]) {
        scene.add_material_rgb(
            name,
            color[0], color[1], color[2],
            0.0, // Not metallic
            0.3, // Slightly rough for better visibility
        );
    }
    
    /// Set object material and position
    fn set_object_material_and_position(&self, scene: &mut Scene, object_index: usize, material_name: &str, position: Vector3<f32>) {
        scene.assign_material_to_object(object_index, material_name);
        if let Some(object) = scene.get_object_mut(object_index) {
            object.set_translation(position);
        }
    }
    
    /// Update object position
    fn update_object_position(&self, scene: &mut Scene, object_index: usize, position: Vector3<f32>) {
        if let Some(object) = scene.get_object_mut(object_index) {
            object.set_translation(position);
        }
    }
    
    /// Remove object from scene by index
    fn remove_object_from_scene(&self, scene: &mut Scene, object_index: usize) {
        if object_index < scene.objects.len() {
            scene.objects.remove(object_index);
        }
    }
    
    /// Clear all history
    pub fn clear_history(&mut self, scene: &mut Scene) {
        self.position_history.clear();
        // Remove in reverse order to maintain indices
        for &index in self.history_gizmo_indices.iter().rev() {
            self.remove_object_from_scene(scene, index);
        }
        self.history_gizmo_indices.clear();
    }
}

impl Gizmo for CameraGizmo {
    fn initialize(&mut self, _scene: &mut Scene, _device: Option<&Device>, _queue: Option<&Queue>) {
        // No special initialization needed
    }
    
    fn update(&mut self, _delta_time: f32, scene: &mut Scene, _device: Option<&Device>, _queue: Option<&Queue>) {
        if !self.enabled {
            return;
        }
        
        // Get current camera position
        let camera_pos = scene.camera_manager.camera.eye;
        
        // Check if camera has moved significantly
        if (camera_pos - self.current_position).magnitude() > 0.001 {
            // Add to history if moved far enough and history is enabled
            if self.show_history && self.should_add_to_history(camera_pos) {
                self.position_history.push_back(self.current_position);
                
                // Limit history size
                if self.position_history.len() > MAX_HISTORY_SIZE {
                    self.position_history.pop_front();
                }
            }
            
            self.current_position = camera_pos;
        }
        
        // Update gizmos
        self.update_current_gizmo(scene);
        self.update_history_gizmos(scene);
    }
    
    fn render_ui(&mut self, ui: &Ui, scene: &mut Scene) {
        ui.window("Camera Gizmo")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Show Current Position", &mut self.show_current);
                ui.checkbox("Show Movement History", &mut self.show_history);
                
                ui.separator();
                
                // Gizmo size control
                ui.slider("Gizmo Size", 0.05, 1.0, &mut self.gizmo_size);
                
                // Current position color
                ui.color_edit3("Current Color", &mut self.current_color);
                
                // History color
                ui.color_edit3("History Color", &mut self.history_color);
                
                ui.separator();
                
                // Camera position display
                ui.text(format!("Position: ({:.2}, {:.2}, {:.2})", 
                    self.current_position.x, 
                    self.current_position.y, 
                    self.current_position.z
                ));
                
                ui.text(format!("History Points: {}", self.position_history.len()));
                
                if ui.button("Clear History") {
                    self.clear_history(scene);
                }
            });
    }
    
    fn name(&self) -> &str {
        "Camera Gizmo"
    }
    
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    fn cleanup(&mut self, scene: &mut Scene) {
        // Remove current gizmo
        if let Some(index) = self.current_gizmo_index {
            self.remove_object_from_scene(scene, index);
        }
        
        // Remove history gizmos (in reverse order to maintain indices)
        for &index in self.history_gizmo_indices.iter().rev() {
            self.remove_object_from_scene(scene, index);
        }
        
        self.current_gizmo_index = None;
        self.history_gizmo_indices.clear();
        self.position_history.clear();
    }
    
    fn get_priority(&self) -> i32 {
        100 // Render camera gizmos on top of most other things
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for CameraGizmo {
    fn default() -> Self {
        Self::new()
    }
}