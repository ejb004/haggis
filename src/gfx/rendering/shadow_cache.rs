//! Shadow map caching system to improve performance
//!
//! This module provides shadow map caching that only regenerates shadow maps when:
//! - Light position or direction changes
//! - Objects in the shadow-casting area move
//! - Manual cache invalidation is requested
//!
//! This significantly improves performance for static or mostly-static scenes.

use cgmath::{Matrix4, Point3, Vector3};
use std::collections::{HashMap, HashSet};

/// Represents a bounding volume for shadow map calculations
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowBounds {
    /// Minimum bounds of the shadow volume
    pub min: Vector3<f32>,
    /// Maximum bounds of the shadow volume
    pub max: Vector3<f32>,
}

impl ShadowBounds {
    /// Creates a new shadow bounds from min/max coordinates
    pub fn new(min: Vector3<f32>, max: Vector3<f32>) -> Self {
        Self { min, max }
    }

    /// Creates shadow bounds from an orthographic projection matrix
    /// This creates a simple bounding box around the light's projection area
    pub fn from_light_projection(_light_pos: Point3<f32>, _light_target: Point3<f32>, bounds: f32, _near: f32, _far: f32) -> Self {
        // For simplicity, create a box around the origin with the given bounds
        // In a more sophisticated implementation, we'd calculate the actual frustum
        Self {
            min: Vector3::new(-bounds, -bounds, -bounds),
            max: Vector3::new(bounds, bounds, bounds),
        }
    }

    /// Checks if a point is within the shadow bounds
    pub fn contains_point(&self, point: Vector3<f32>) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }

    /// Checks if an object with transform intersects with the shadow bounds
    pub fn intersects_object(&self, object_transform: &Matrix4<f32>) -> bool {
        // Extract position from transform matrix
        let position = Vector3::new(
            object_transform[3][0],
            object_transform[3][1],
            object_transform[3][2],
        );

        // For now, use a simple point-in-bounds check
        // In a more sophisticated implementation, we'd check the full bounding box
        self.contains_point(position)
    }
}

/// Tracks the state of a light source for shadow map caching
#[derive(Debug, Clone, PartialEq)]
pub struct LightState {
    /// Light position
    pub position: [f32; 3],
    /// Light color
    pub color: [f32; 3],
    /// Light intensity
    pub intensity: f32,
    /// Light view-projection matrix
    pub view_proj_matrix: Matrix4<f32>,
}

impl LightState {
    /// Creates a new light state from light configuration
    pub fn from_light_config(config: &crate::gfx::resources::global_bindings::LightConfig) -> Self {
        // Calculate light view-projection matrix
        let light_pos = Point3::new(config.position[0], config.position[1], config.position[2]);
        let light_view = Matrix4::look_at_rh(
            light_pos,
            Point3::new(0.0, -1.0, 0.0), // Look at between monkey and cube
            Vector3::unit_y(),
        );

        let light_proj = cgmath::ortho(-25.0, 25.0, -25.0, 25.0, 5.0, 50.0);
        let light_view_proj = light_proj * light_view;

        Self {
            position: config.position,
            color: config.color,
            intensity: config.intensity,
            view_proj_matrix: light_view_proj,
        }
    }

    /// Checks if this light state differs significantly from another
    pub fn differs_from(&self, other: &LightState) -> bool {
        const EPSILON: f32 = 0.001;

        // Check position
        for i in 0..3 {
            if (self.position[i] - other.position[i]).abs() > EPSILON {
                return true;
            }
        }

        // Check color
        for i in 0..3 {
            if (self.color[i] - other.color[i]).abs() > EPSILON {
                return true;
            }
        }

        // Check intensity
        if (self.intensity - other.intensity).abs() > EPSILON {
            return true;
        }

        false
    }
}

/// Tracks the transform state of an object for shadow map caching
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTransformState {
    /// Object transform matrix
    pub transform: Matrix4<f32>,
    /// Whether the object is visible
    pub visible: bool,
}

impl ObjectTransformState {
    /// Creates a new object transform state
    pub fn new(transform: Matrix4<f32>, visible: bool) -> Self {
        Self { transform, visible }
    }

    /// Checks if this transform state differs from another
    pub fn differs_from(&self, other: &ObjectTransformState) -> bool {
        if self.visible != other.visible {
            return true;
        }

        if !self.visible {
            // If both are invisible, they don't affect shadows
            return false;
        }

        // Check if transform matrices differ significantly
        const EPSILON: f32 = 0.001;
        let self_mat: &[f32; 16] = self.transform.as_ref();
        let other_mat: &[f32; 16] = other.transform.as_ref();

        for i in 0..16 {
            if (self_mat[i] - other_mat[i]).abs() > EPSILON {
                return true;
            }
        }

        false
    }
}

/// Shadow map cache manager
pub struct ShadowCache {
    /// Whether the shadow map is currently valid
    is_valid: bool,
    /// Last known light state
    last_light_state: Option<LightState>,
    /// Last known object transform states (indexed by object name/id)
    last_object_states: HashMap<String, ObjectTransformState>,
    /// Shadow bounds for determining which objects affect shadows
    shadow_bounds: Option<ShadowBounds>,
    /// Objects that are currently within shadow bounds
    objects_in_shadow_bounds: HashSet<String>,
    /// Manual invalidation flag
    force_invalidate: bool,
}

impl ShadowCache {
    /// Creates a new shadow cache
    pub fn new() -> Self {
        Self {
            is_valid: false,
            last_light_state: None,
            last_object_states: HashMap::new(),
            shadow_bounds: None,
            objects_in_shadow_bounds: HashSet::new(),
            force_invalidate: false,
        }
    }

    /// Checks if the shadow map needs to be regenerated
    pub fn needs_update(
        &mut self,
        current_light: &crate::gfx::resources::global_bindings::LightConfig,
        objects: &[crate::gfx::scene::object::Object],
    ) -> bool {
        // Always regenerate if manually invalidated
        if self.force_invalidate {
            // #[cfg(debug_assertions)]
            // println!("🔄 Shadow cache: Manual invalidation requested");
            self.force_invalidate = false;
            self.is_valid = false;
            return true;
        }

        // Always regenerate if cache is invalid
        if !self.is_valid {
            // #[cfg(debug_assertions)]
            // println!("🔄 Shadow cache: Cache invalid - first render");
            return true;
        }

        let current_light_state = LightState::from_light_config(current_light);

        // Check if light has changed
        if let Some(ref last_light) = self.last_light_state {
            if current_light_state.differs_from(last_light) {
                // #[cfg(debug_assertions)]
                // println!("💡 Shadow cache: Light changed - position/color/intensity");
                self.is_valid = false;
                return true;
            }
        } else {
            // No previous light state, need to update
            self.is_valid = false;
            return true;
        }

        // Shadow bounds are already up to date since light didn't change

        // Check if any objects in shadow bounds have changed
        for object in objects {
            let object_id = &object.name;
            let current_state = ObjectTransformState::new(object.transform, object.visible);

            // Check if object is in shadow bounds
            let in_bounds = if let Some(ref bounds) = self.shadow_bounds {
                bounds.intersects_object(&object.transform)
            } else {
                true // If no bounds, assume all objects affect shadows
            };

            if in_bounds {
                self.objects_in_shadow_bounds.insert(object_id.clone());

                // Check if object state has changed
                if let Some(ref last_state) = self.last_object_states.get(object_id) {
                    if current_state.differs_from(last_state) {
                        // #[cfg(debug_assertions)]
                        // println!("📦 Shadow cache: Object '{}' moved/changed in shadow area", object_id);
                        self.is_valid = false;
                        return true;
                    }
                } else {
                    // New object in shadow bounds
                    // #[cfg(debug_assertions)]
                    // println!("🆕 Shadow cache: New object '{}' entered shadow area", object_id);
                    self.is_valid = false;
                    return true;
                }
            } else {
                // Object is no longer in bounds, but was it before?
                if self.objects_in_shadow_bounds.contains(object_id) {
                    self.objects_in_shadow_bounds.remove(object_id);
                    self.is_valid = false;
                    return true;
                }
            }
        }

        // Check if any previously tracked objects are now gone
        let current_object_names: HashSet<String> = objects.iter().map(|o| o.name.clone()).collect();
        let removed_objects: Vec<String> = self.last_object_states.keys()
            .filter(|name| !current_object_names.contains(*name))
            .cloned()
            .collect();

        if !removed_objects.is_empty() {
            for name in removed_objects {
                self.last_object_states.remove(&name);
                self.objects_in_shadow_bounds.remove(&name);
            }
            self.is_valid = false;
            return true;
        }

        false
    }

    /// Marks the shadow map as valid and updates cached state
    pub fn mark_valid(
        &mut self,
        current_light: &crate::gfx::resources::global_bindings::LightConfig,
        objects: &[crate::gfx::scene::object::Object],
    ) {
        self.is_valid = true;
        self.last_light_state = Some(LightState::from_light_config(current_light));

        // Update cached object states
        self.last_object_states.clear();  // Clear old states
        for object in objects {
            let state = ObjectTransformState::new(object.transform, object.visible);
            self.last_object_states.insert(object.name.clone(), state);
        }

        // Update shadow bounds
        if let Some(light_state) = &self.last_light_state {
            let light_pos = Point3::new(
                light_state.position[0],
                light_state.position[1],
                light_state.position[2],
            );
            let light_target = Point3::new(0.0, -1.0, 0.0);

            self.shadow_bounds = Some(ShadowBounds::from_light_projection(
                light_pos,
                light_target,
                25.0, // bounds
                5.0,  // near
                50.0, // far
            ));
        }
    }

    /// Forces the cache to be invalidated on the next check
    pub fn invalidate(&mut self) {
        self.force_invalidate = true;
        self.is_valid = false;
    }

    /// Returns whether the cache is currently valid
    pub fn is_valid(&self) -> bool {
        self.is_valid && !self.force_invalidate
    }

    /// Clears all cached state
    pub fn clear(&mut self) {
        self.is_valid = false;
        self.last_light_state = None;
        self.last_object_states.clear();
        self.shadow_bounds = None;
        self.objects_in_shadow_bounds.clear();
        self.force_invalidate = false;
    }


    /// Gets statistics about the cache
    pub fn get_stats(&self) -> ShadowCacheStats {
        ShadowCacheStats {
            is_valid: self.is_valid,
            tracked_objects: self.last_object_states.len(),
            objects_in_shadow_bounds: self.objects_in_shadow_bounds.len(),
            has_light_state: self.last_light_state.is_some(),
        }
    }
}

/// Statistics about the shadow cache for debugging
#[derive(Debug)]
pub struct ShadowCacheStats {
    pub is_valid: bool,
    pub tracked_objects: usize,
    pub objects_in_shadow_bounds: usize,
    pub has_light_state: bool,
}

impl Default for ShadowCache {
    fn default() -> Self {
        Self::new()
    }
}