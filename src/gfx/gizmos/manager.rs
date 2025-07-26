//! # Gizmo Manager
//!
//! This module provides the management system for gizmos in the Haggis engine.
//! The GizmoManager handles the lifecycle, updates, and rendering of multiple gizmos.

use crate::gfx::gizmos::traits::Gizmo;
use crate::gfx::scene::Scene;
use imgui::Ui;
use std::collections::HashMap;
use wgpu::{Device, Queue};

/// Manager for handling multiple gizmo instances
pub struct GizmoManager {
    /// Collection of registered gizmos
    gizmos: HashMap<String, Box<dyn Gizmo>>,
    
    /// Whether the gizmo system is globally enabled
    enabled: bool,
    
    /// Whether to show the gizmo manager UI
    show_ui: bool,
}

impl GizmoManager {
    /// Create a new gizmo manager
    pub fn new() -> Self {
        Self {
            gizmos: HashMap::new(),
            enabled: true,
            show_ui: true,
        }
    }
    
    /// Add a new gizmo to the manager
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the gizmo
    /// * `gizmo` - The gizmo instance to add
    /// * `scene` - Scene reference for initialization
    /// * `device` - Optional GPU device
    /// * `queue` - Optional GPU queue
    pub fn add_gizmo(
        &mut self,
        name: String,
        mut gizmo: Box<dyn Gizmo>,
        scene: &mut Scene,
        device: Option<&Device>,
        queue: Option<&Queue>,
    ) {
        gizmo.initialize(scene, device, queue);
        self.gizmos.insert(name, gizmo);
    }
    
    /// Remove a gizmo from the manager
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the gizmo to remove
    /// * `scene` - Scene reference for cleanup
    pub fn remove_gizmo(&mut self, name: &str, scene: &mut Scene) {
        if let Some(mut gizmo) = self.gizmos.remove(name) {
            gizmo.cleanup(scene);
        }
    }
    
    /// Check if a gizmo exists by name
    pub fn has_gizmo(&self, name: &str) -> bool {
        self.gizmos.contains_key(name)
    }
    
    /// Update all gizmos
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last frame
    /// * `scene` - Scene reference for updates
    /// * `device` - Optional GPU device
    /// * `queue` - Optional GPU queue
    pub fn update(
        &mut self,
        delta_time: f32,
        scene: &mut Scene,
        device: Option<&Device>,
        queue: Option<&Queue>,
    ) {
        if !self.enabled {
            return;
        }
        
        // Collect visible gizmos sorted by priority
        let mut visible_gizmos: Vec<_> = self.gizmos.iter_mut()
            .filter(|(_, gizmo)| gizmo.should_be_visible(scene))
            .collect();
        
        visible_gizmos.sort_by_key(|(_, gizmo)| gizmo.get_priority());
        
        // Update each visible gizmo
        for (_, gizmo) in visible_gizmos {
            gizmo.update(delta_time, scene, device, queue);
        }
    }
    
    /// Render UI for all gizmos
    ///
    /// # Arguments
    ///
    /// * `ui` - ImGui UI context
    /// * `scene` - Scene reference for UI rendering
    pub fn render_ui(&mut self, ui: &Ui, scene: &mut Scene) {
        if !self.show_ui {
            return;
        }
        
        // Render global gizmo manager controls
        ui.window("Gizmo Manager")
            .size([300.0, 150.0], imgui::Condition::FirstUseEver)
            .position([20.0, 20.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Enable Gizmos", &mut self.enabled);
                ui.separator();
                
                ui.text(format!("Active Gizmos: {}", self.gizmos.len()));
                
                // List all gizmos with enable/disable controls
                for (name, gizmo) in &mut self.gizmos {
                    let mut enabled = gizmo.is_enabled();
                    if ui.checkbox(&format!("{}", name), &mut enabled) {
                        gizmo.set_enabled(enabled);
                    }
                }
                
                if ui.button("Disable All") {
                    for (_, gizmo) in &mut self.gizmos {
                        gizmo.set_enabled(false);
                    }
                }
                
                ui.same_line();
                if ui.button("Enable All") {
                    for (_, gizmo) in &mut self.gizmos {
                        gizmo.set_enabled(true);
                    }
                }
            });
        
        // Render individual gizmo UIs
        if self.enabled {
            for (_, gizmo) in &mut self.gizmos {
                if gizmo.should_be_visible(scene) {
                    gizmo.render_ui(ui, scene);
                }
            }
        }
    }
    
    /// Check if the gizmo system is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Set the enabled state of the gizmo system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Check if the gizmo manager UI is visible
    pub fn is_ui_visible(&self) -> bool {
        self.show_ui
    }
    
    /// Set the visibility of the gizmo manager UI
    pub fn set_ui_visible(&mut self, visible: bool) {
        self.show_ui = visible;
    }
    
    /// Get the number of registered gizmos
    pub fn gizmo_count(&self) -> usize {
        self.gizmos.len()
    }
    
    /// Get a list of all gizmo names
    pub fn get_gizmo_names(&self) -> Vec<&String> {
        self.gizmos.keys().collect()
    }
    
    /// Clean up all gizmos
    pub fn cleanup(&mut self, scene: &mut Scene) {
        for (_, mut gizmo) in self.gizmos.drain() {
            gizmo.cleanup(scene);
        }
    }
}

impl Default for GizmoManager {
    fn default() -> Self {
        Self::new()
    }
}