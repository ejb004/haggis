//! # Scene Building Patterns
//!
//! Consistent builder patterns for scene objects and materials

use crate::builder::{Builder, CommonConfig, ConfigurableBuilder};
use crate::gfx::{
    resources::material::MaterialId,
    scene::{object::{Mesh, Object, UiTransformState}, vertex::Vertex3D},
};
use cgmath::{Matrix4, Vector3, Deg, Rad};

/// Builder for 3D objects in the scene
pub struct ObjectBuilder {
    pub(crate) common: CommonConfig,
    pub(crate) meshes: Vec<Mesh>,
    pub(crate) position: [f32; 3],
    pub(crate) rotation: [f32; 3], // degrees
    pub(crate) scale: f32,
    pub(crate) material_id: Option<MaterialId>,
    pub(crate) visible: bool,
}

impl Default for ObjectBuilder {
    fn default() -> Self {
        Self {
            common: CommonConfig::default(),
            meshes: Vec::new(),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
            material_id: None,
            visible: true,
        }
    }
}

impl ObjectBuilder {
    /// Create new object builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add mesh from vertices and indices
    pub fn with_mesh(mut self, vertices: Vec<Vertex3D>, indices: Vec<u32>) -> Self {
        // Convert Vertex3D to separate position/normal arrays for Mesh::new
        let mut positions = Vec::new();
        let mut normals = Vec::new();

        for vertex in vertices {
            positions.extend_from_slice(&vertex.position);
            normals.extend_from_slice(&vertex.normal);
        }

        let mesh = Mesh::new(positions, normals, indices);
        self.meshes.push(mesh);
        self
    }

    /// Add mesh from position and normal arrays
    pub fn with_mesh_arrays(mut self, positions: Vec<f32>, normals: Vec<f32>, indices: Vec<u32>) -> Self {
        let mesh = Mesh::new(positions, normals, indices);
        self.meshes.push(mesh);
        self
    }

    /// Create a cube mesh
    pub fn with_cube(mut self) -> Self {
        let positions = vec![
            // Front face
            -1.0, -1.0,  1.0,
             1.0, -1.0,  1.0,
             1.0,  1.0,  1.0,
            -1.0,  1.0,  1.0,
            // Back face
            -1.0, -1.0, -1.0,
            -1.0,  1.0, -1.0,
             1.0,  1.0, -1.0,
             1.0, -1.0, -1.0,
        ];

        let normals = vec![
            // Front face
             0.0,  0.0,  1.0,
             0.0,  0.0,  1.0,
             0.0,  0.0,  1.0,
             0.0,  0.0,  1.0,
            // Back face
             0.0,  0.0, -1.0,
             0.0,  0.0, -1.0,
             0.0,  0.0, -1.0,
             0.0,  0.0, -1.0,
        ];

        let indices = vec![
            0, 1, 2,  0, 2, 3,    // Front face
            4, 5, 6,  4, 6, 7,    // Back face
            5, 0, 3,  5, 3, 6,    // Left face
            1, 4, 7,  1, 7, 2,    // Right face
            3, 2, 6,  3, 6, 5,    // Top face
            4, 1, 0,  4, 0, 5,    // Bottom face
        ];

        let mesh = Mesh::new(positions, normals, indices);
        self.meshes.push(mesh);
        self
    }

    /// Create a sphere mesh with given subdivisions
    pub fn with_sphere(mut self, subdivisions: u32) -> Self {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        let subdivisions = subdivisions.max(3); // Minimum 3 subdivisions

        // Generate sphere vertices
        for i in 0..=subdivisions {
            let theta = std::f32::consts::PI * (i as f32) / (subdivisions as f32);
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for j in 0..=subdivisions {
                let phi = 2.0 * std::f32::consts::PI * (j as f32) / (subdivisions as f32);
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = sin_theta * cos_phi;
                let y = cos_theta;
                let z = sin_theta * sin_phi;

                positions.extend_from_slice(&[x, y, z]);
                normals.extend_from_slice(&[x, y, z]); // Normal is same as position for unit sphere
            }
        }

        // Generate indices
        for i in 0..subdivisions {
            for j in 0..subdivisions {
                let first = (i * (subdivisions + 1) + j) as u32;
                let second = first + subdivisions + 1;

                indices.extend_from_slice(&[first, second, first + 1]);
                indices.extend_from_slice(&[second, second + 1, first + 1]);
            }
        }

        let mesh = Mesh::new(positions, normals, indices);
        self.meshes.push(mesh);
        self
    }

    /// Set object position
    pub fn with_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    /// Set object rotation in degrees
    pub fn with_rotation(mut self, rotation: [f32; 3]) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set object scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Set object transform (position, scale, y-rotation for compatibility)
    pub fn with_transform(mut self, position: [f32; 3], scale: f32, rotation_y: f32) -> Self {
        self.position = position;
        self.scale = scale;
        self.rotation[1] = rotation_y;
        self
    }

    /// Set material ID
    pub fn with_material_id(mut self, material_id: MaterialId) -> Self {
        self.material_id = Some(material_id);
        self
    }

    /// Set visibility
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

impl Builder<Object> for ObjectBuilder {
    fn build(self) -> Object {
        // Calculate transform matrix
        let translation = Matrix4::from_translation(Vector3::new(
            self.position[0],
            self.position[1], 
            self.position[2],
        ));
        let rotation_x = Matrix4::from_angle_x(Deg(self.rotation[0]));
        let rotation_y = Matrix4::from_angle_y(Deg(self.rotation[1]));
        let rotation_z = Matrix4::from_angle_z(Deg(self.rotation[2]));
        let scale_matrix = Matrix4::from_scale(self.scale);

        let transform = translation * rotation_z * rotation_y * rotation_x * scale_matrix;

        let ui_transform = UiTransformState {
            position: self.position,
            rotation: self.rotation,
            scale: self.scale,
        };

        Object {
            meshes: self.meshes,
            transform,
            gpu_resources: None,
            name: self.common.name.unwrap_or_else(|| "Object".to_string()),
            ui_transform,
            visible: self.visible,
            material_id: self.material_id,
        }
    }
}

impl ConfigurableBuilder<Object> for ObjectBuilder {
    fn merge(mut self, other: Self) -> Self {
        // Merge meshes
        self.meshes.extend(other.meshes);
        
        // Other takes precedence for properties
        if other.position != [0.0, 0.0, 0.0] {
            self.position = other.position;
        }
        if other.rotation != [0.0, 0.0, 0.0] {
            self.rotation = other.rotation;
        }
        if other.scale != 1.0 {
            self.scale = other.scale;
        }
        if other.material_id.is_some() {
            self.material_id = other.material_id;
        }
        if !other.visible {
            self.visible = other.visible;
        }
        
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.meshes.is_empty() {
            return Err("At least one mesh is required".to_string());
        }
        if self.scale <= 0.0 {
            return Err("Scale must be positive".to_string());
        }
        Ok(())
    }
}

// Implement common builder methods using macro
crate::impl_common_builder_methods!(ObjectBuilder);

/// Builder for material configuration
pub struct MaterialBuilder {
    pub(crate) common: CommonConfig,
    pub(crate) color: [f32; 3],
    pub(crate) metallic: f32,
    pub(crate) roughness: f32,
}

impl Default for MaterialBuilder {
    fn default() -> Self {
        Self {
            common: CommonConfig::default(),
            color: [0.8, 0.8, 0.8],
            metallic: 0.0,
            roughness: 0.5,
        }
    }
}

impl MaterialBuilder {
    /// Create new material builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set material color
    pub fn with_color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }

    /// Set metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Set roughness factor (0.0 = mirror, 1.0 = rough)
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set material as metal with given color
    pub fn as_metal(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self.metallic = 1.0;
        self.roughness = 0.1;
        self
    }

    /// Set material as plastic with given color
    pub fn as_plastic(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self.metallic = 0.0;
        self.roughness = 0.3;
        self
    }

    /// Set material as rubber with given color
    pub fn as_rubber(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self.metallic = 0.0;
        self.roughness = 0.8;
        self
    }
}

// Material doesn't have a simple build target since it's managed by the material system
// This would be used by scene builders that need to create materials

// Implement common builder methods using macro
crate::impl_common_builder_methods!(MaterialBuilder);