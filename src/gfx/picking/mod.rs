//! # Object Picking System
//!
//! This module provides 3D object picking functionality using mouse ray-casting.
//! It allows users to click on 3D objects in the scene to select them for 
//! manipulation or inspection.
//!
//! ## How it works
//!
//! 1. **Mouse to Ray**: Convert mouse coordinates to a 3D ray in world space
//! 2. **Ray-Object Intersection**: Test the ray against object bounding boxes/meshes
//! 3. **Selection**: Return the closest intersected object
//!
//! ## Usage
//!
//! ```rust
//! use haggis::gfx::picking::ObjectPicker;
//!
//! let picker = ObjectPicker::new();
//! if let Some(object_index) = picker.pick_object(mouse_pos, camera, scene) {
//!     println!("Selected object: {}", object_index);
//! }
//! ```

use cgmath::{Vector3, Vector4, Matrix4, InnerSpace, Zero, ElementWise, EuclideanSpace, SquareMatrix};
use crate::gfx::{
    scene::Scene,
    camera::orbit_camera::OrbitCamera,
};

/// A 3D ray for intersection testing
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// Ray origin point in world space
    pub origin: Vector3<f32>,
    /// Ray direction (normalized)
    pub direction: Vector3<f32>,
}

impl Ray {
    /// Create a new ray
    pub fn new(origin: Vector3<f32>, direction: Vector3<f32>) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Get a point along the ray at distance t
    pub fn point_at(&self, t: f32) -> Vector3<f32> {
        self.origin + self.direction * t
    }
}

/// Axis-aligned bounding box for intersection testing
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    /// Minimum corner of the bounding box
    pub min: Vector3<f32>,
    /// Maximum corner of the bounding box
    pub max: Vector3<f32>,
}

impl AABB {
    /// Create a new AABB
    pub fn new(min: Vector3<f32>, max: Vector3<f32>) -> Self {
        Self { min, max }
    }

    /// Create AABB from a set of vertices
    pub fn from_vertices(vertices: &[[f32; 3]]) -> Self {
        if vertices.is_empty() {
            return Self::new(Vector3::zero(), Vector3::zero());
        }

        let mut min = Vector3::new(vertices[0][0], vertices[0][1], vertices[0][2]);
        let mut max = min;

        for vertex in vertices.iter().skip(1) {
            let v = Vector3::new(vertex[0], vertex[1], vertex[2]);
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }

        Self::new(min, max)
    }

    /// Test ray-AABB intersection
    /// Returns the distance to intersection point, or None if no intersection
    pub fn intersect_ray(&self, ray: &Ray) -> Option<f32> {
        let inv_dir = Vector3::new(
            1.0 / ray.direction.x,
            1.0 / ray.direction.y,
            1.0 / ray.direction.z,
        );

        let t_min = (self.min - ray.origin).mul_element_wise(inv_dir);
        let t_max = (self.max - ray.origin).mul_element_wise(inv_dir);

        let t1 = Vector3::new(
            t_min.x.min(t_max.x),
            t_min.y.min(t_max.y),
            t_min.z.min(t_max.z),
        );
        let t2 = Vector3::new(
            t_min.x.max(t_max.x),
            t_min.y.max(t_max.y),
            t_min.z.max(t_max.z),
        );

        let t_near = t1.x.max(t1.y.max(t1.z));
        let t_far = t2.x.min(t2.y.min(t2.z));

        if t_near <= t_far && t_far >= 0.0 {
            Some(if t_near >= 0.0 { t_near } else { t_far })
        } else {
            None
        }
    }

    /// Apply a transformation matrix to the AABB
    pub fn transform(&self, matrix: &Matrix4<f32>) -> Self {
        // Transform all 8 corners of the AABB and compute new bounds
        let corners = [
            Vector3::new(self.min.x, self.min.y, self.min.z),
            Vector3::new(self.max.x, self.min.y, self.min.z),
            Vector3::new(self.min.x, self.max.y, self.min.z),
            Vector3::new(self.min.x, self.min.y, self.max.z),
            Vector3::new(self.max.x, self.max.y, self.min.z),
            Vector3::new(self.max.x, self.min.y, self.max.z),
            Vector3::new(self.min.x, self.max.y, self.max.z),
            Vector3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut transformed_corners = Vec::with_capacity(8);
        for corner in &corners {
            let homogeneous = Vector4::new(corner.x, corner.y, corner.z, 1.0);
            let transformed = matrix * homogeneous;
            transformed_corners.push([
                transformed.x / transformed.w,
                transformed.y / transformed.w,
                transformed.z / transformed.w,
            ]);
        }

        Self::from_vertices(&transformed_corners)
    }
}

/// Result of an object picking operation
#[derive(Debug, Clone)]
pub struct PickResult {
    /// Index of the picked object in the scene
    pub object_index: usize,
    /// Distance from camera to intersection point
    pub distance: f32,
    /// World space intersection point
    pub intersection_point: Vector3<f32>,
}

/// Object picker for 3D mouse selection
pub struct ObjectPicker {
    /// Cache bounding boxes to avoid recomputation
    cached_aabbs: Vec<Option<AABB>>,
}

impl ObjectPicker {
    /// Create a new object picker
    pub fn new() -> Self {
        Self {
            cached_aabbs: Vec::new(),
        }
    }

    /// Convert screen coordinates to a world-space ray
    pub fn screen_to_ray(
        &self,
        screen_pos: (f32, f32),
        screen_size: (f32, f32),
        camera: &OrbitCamera,
    ) -> Ray {
        let (mouse_x, mouse_y) = screen_pos;
        let (screen_width, screen_height) = screen_size;

        // Convert screen coordinates to normalized device coordinates (-1 to 1)
        let ndc_x = (2.0 * mouse_x) / screen_width - 1.0;
        let ndc_y = 1.0 - (2.0 * mouse_y) / screen_height; // Flip Y axis

        // Get camera view and projection matrices
        let eye = cgmath::Point3::from_vec(camera.eye);
        let target = cgmath::Point3::from_vec(camera.target);
        let view_matrix = cgmath::Matrix4::look_at_rh(eye, target, camera.up);
        
        let proj_matrix = cgmath::perspective(camera.fovy, camera.aspect, camera.znear, camera.zfar);
        
        // Calculate inverse matrices
        let view_proj_matrix = proj_matrix * view_matrix;
        let inv_view_proj = view_proj_matrix.invert().unwrap_or(Matrix4::from_scale(1.0));

        // Transform near and far points from NDC to world space
        let near_point = Vector4::new(ndc_x, ndc_y, -1.0, 1.0); // Near plane in NDC
        let far_point = Vector4::new(ndc_x, ndc_y, 1.0, 1.0);   // Far plane in NDC

        let world_near = inv_view_proj * near_point;
        let world_far = inv_view_proj * far_point;

        // Convert from homogeneous coordinates
        let near_3d = Vector3::new(
            world_near.x / world_near.w,
            world_near.y / world_near.w,
            world_near.z / world_near.w,
        );
        let far_3d = Vector3::new(
            world_far.x / world_far.w,
            world_far.y / world_far.w,
            world_far.z / world_far.w,
        );

        // Create ray from near to far point
        let direction = (far_3d - near_3d).normalize();
        Ray::new(near_3d, direction)
    }

    /// Pick an object from the scene using mouse coordinates
    pub fn pick_object(
        &mut self,
        screen_pos: (f32, f32),
        screen_size: (f32, f32),
        camera: &OrbitCamera,
        scene: &Scene,
    ) -> Option<PickResult> {
        let ray = self.screen_to_ray(screen_pos, screen_size, camera);
        
        // Ensure we have enough cached AABBs
        while self.cached_aabbs.len() < scene.objects.len() {
            self.cached_aabbs.push(None);
        }

        let mut closest_result: Option<PickResult> = None;

        for (i, object) in scene.objects.iter().enumerate() {
            // Get or compute AABB for this object
            let aabb = if let Some(cached) = &self.cached_aabbs[i] {
                *cached
            } else {
                // Compute AABB from object vertices
                let aabb = self.compute_object_aabb(object);
                self.cached_aabbs[i] = Some(aabb);
                aabb
            };

            // Apply object's transform to AABB
            let world_aabb = aabb.transform(&object.transform);

            // Test ray intersection
            if let Some(distance) = world_aabb.intersect_ray(&ray) {
                let intersection_point = ray.point_at(distance);
                
                // Keep the closest intersection
                if closest_result
                    .as_ref()
                    .map_or(true, |result| distance < result.distance)
                {
                    closest_result = Some(PickResult {
                        object_index: i,
                        distance,
                        intersection_point,
                    });
                }
            }
        }

        closest_result
    }

    /// Compute AABB for an object from its mesh data
    fn compute_object_aabb(&self, object: &crate::gfx::scene::object::Object) -> AABB {
        let mut all_vertices = Vec::new();

        // Collect vertices from all meshes in the object
        for mesh in &object.meshes {
            // Get vertices from mesh
            for vertex in mesh.vertices() {
                all_vertices.push(vertex.position);
            }
        }

        if all_vertices.is_empty() {
            // Fallback to unit cube if no vertices
            AABB::new(
                Vector3::new(-0.5, -0.5, -0.5),
                Vector3::new(0.5, 0.5, 0.5),
            )
        } else {
            AABB::from_vertices(&all_vertices)
        }
    }

    /// Invalidate cached AABBs (call when objects change)
    pub fn invalidate_cache(&mut self) {
        self.cached_aabbs.clear();
    }

    /// Invalidate AABB for a specific object
    pub fn invalidate_object(&mut self, object_index: usize) {
        if object_index < self.cached_aabbs.len() {
            self.cached_aabbs[object_index] = None;
        }
    }
}

impl Default for ObjectPicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_creation() {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [-1.0, -1.0, -1.0],
        ];
        let aabb = AABB::from_vertices(&vertices);
        
        assert_eq!(aabb.min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_ray_aabb_intersection() {
        let aabb = AABB::new(
            Vector3::new(-1.0, -1.0, -1.0),
            Vector3::new(1.0, 1.0, 1.0),
        );
        
        // Ray hitting the box
        let ray = Ray::new(
            Vector3::new(0.0, 0.0, -5.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        
        assert!(aabb.intersect_ray(&ray).is_some());
        
        // Ray missing the box
        let ray_miss = Ray::new(
            Vector3::new(5.0, 0.0, -5.0),
            Vector3::new(0.0, 0.0, 1.0),
        );
        
        assert!(aabb.intersect_ray(&ray_miss).is_none());
    }
}