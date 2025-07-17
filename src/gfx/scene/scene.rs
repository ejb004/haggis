use wgpu::Device;

use crate::gfx::{
    camera::camera_utils::CameraManager,
    resources::material::{Material, MaterialManager},
    scene::object::Mesh,
};

use super::object::Object;

/// Main scene containing objects, materials, and camera
pub struct Scene {
    pub camera_manager: CameraManager,
    pub objects: Vec<Object>,
    pub material_manager: MaterialManager, // Centralized material storage
}

impl Scene {
    /// Creates a new scene with the given camera manager
    pub fn new(camera_manager: CameraManager) -> Self {
        Self {
            camera_manager,
            objects: Vec::new(),
            material_manager: MaterialManager::new(), // Initialize with default material
        }
    }

    /// Updates the scene (camera matrices, etc.)
    pub fn update(&mut self) {
        self.camera_manager.camera.update_view_proj();
    }

    /// Loads a 3D object from an OBJ file with automatic material extraction
    ///
    /// Loads both geometry and materials from the OBJ/MTL files and automatically
    /// assigns materials to objects based on the material IDs in the OBJ file.
    pub fn add_object(&mut self, object_path: &str) {
        let (models, materials) = tobj::load_obj(
            object_path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )
        .expect("Failed to OBJ load file");

        let materials = materials.unwrap_or_else(|_| {
            println!("No MTL file found, using default materials");
            Vec::new()
        });

        // Load materials from OBJ file into material manager
        for (i, mtl) in materials.iter().enumerate() {
            let material_name = if mtl.name.is_empty() {
                format!("material_{}", i)
            } else {
                mtl.name.clone()
            };

            // Skip if material already exists
            if self.material_manager.get_material(&material_name).is_some() {
                continue;
            }

            let diffuse = mtl.diffuse.unwrap_or([0.8, 0.8, 0.8]);
            let material = Material::new(
                &material_name,
                [
                    diffuse[0],
                    diffuse[1],
                    diffuse[2],
                    mtl.dissolve.unwrap_or(1.0), // Alpha from dissolve
                ],
                0.0, // Default metallic (MTL doesn't have direct metallic values)
                1.0 - (mtl.shininess.unwrap_or(32.0) / 128.0).clamp(0.0, 1.0), // Convert shininess to roughness
            );

            self.material_manager.add_material(material);
        }

        let mut meshes = Vec::new();

        for m in models.iter() {
            let mesh = &m.mesh;

            // DEBUG: Print what we're getting from tobj
            // println!(
            //     "Positions: {} ({} vertices)",
            //     mesh.positions.len(),
            //     mesh.positions.len() / 3
            // );
            // println!(
            //     "Normals: {} ({} normals)",
            //     mesh.normals.len(),
            //     mesh.normals.len() / 3
            // );
            // println!(
            //     "Indices: {} ({} triangles)",
            //     mesh.indices.len(),
            //     mesh.indices.len() / 3
            // );

            // Use normals from OBJ if available, otherwise calculate them
            let normals = if !mesh.normals.is_empty() && mesh.normals.len() == mesh.positions.len()
            {
                mesh.normals.clone()
            } else {
                Mesh::calculate_face_normals(&mesh.positions, &mesh.indices)
            };

            let our_mesh = Mesh::new(mesh.positions.clone(), normals, mesh.indices.clone());
            meshes.push(our_mesh);
        }

        // Create object and assign material if available
        let mut object = Object::new(meshes);

        // Set object name from the first model
        if let Some(first_model) = models.first() {
            if !first_model.name.is_empty() {
                object.set_name(first_model.name.clone());
            }

            // Assign material from OBJ file if available
            if let Some(material_id) = first_model.mesh.material_id {
                if material_id < materials.len() {
                    let material_name = if materials[material_id].name.is_empty() {
                        format!("material_{}", material_id)
                    } else {
                        materials[material_id].name.clone()
                    };
                    object.set_material(&material_name);
                }
            }
        }

        self.objects.push(object);
    }

    /// Creates a new material and adds it to the material manager
    ///
    /// # Arguments
    /// * `name` - Unique name for the material
    /// * `base_color` - RGBA base color
    /// * `metallic` - Metallic factor
    /// * `roughness` - Roughness factor
    ///
    /// # Returns
    /// Mutable reference to the created material
    pub fn add_material(
        &mut self,
        name: &str,
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
    ) -> &mut Material {
        let material_name = name.to_string();
        let material = Material::new(&material_name, base_color, metallic, roughness);
        self.material_manager.add_material(material);
        self.material_manager
            .get_material_mut(&material_name)
            .unwrap()
    }

    /// Convenience method for creating materials with RGB colors
    ///
    /// # Arguments
    /// * `name` - Unique name for the material
    /// * `r`, `g`, `b` - RGB color components (0.0-1.0)
    /// * `metallic` - Metallic factor (0.0-1.0)
    /// * `roughness` - Roughness factor (0.0-1.0)
    pub fn add_material_rgb(
        &mut self,
        name: &str,
        r: f32,
        g: f32,
        b: f32,
        metallic: f32,
        roughness: f32,
    ) -> &mut Material {
        self.add_material(name, [r, g, b, 1.0], metallic, roughness)
    }

    /// Initializes GPU resources for all objects and materials
    ///
    /// Must be called after the GPU context is available and before rendering.
    pub fn init_gpu_resources(&mut self, device: &Device, queue: &wgpu::Queue) {
        // Initialize object GPU resources
        for object in self.objects.iter_mut() {
            object.init_gpu_resources(device);
        }

        // Initialize material GPU resources
        self.material_manager
            .update_all_gpu_resources(device, queue);
    }

    /// Updates all object transforms and syncs to GPU
    pub fn update_all_transforms(&mut self, queue: &wgpu::Queue) {
        for object in &mut self.objects {
            if object.gpu_resources.is_some() {
                object.update_transform(queue);
            }
        }
    }

    /// Updates material GPU resources when materials have changed
    ///
    /// Call this after modifying material properties to sync changes to GPU.
    pub fn update_materials(&mut self, device: &Device, queue: &wgpu::Queue) {
        self.material_manager
            .update_all_gpu_resources(device, queue);
    }

    /// Gets material for rendering an object
    ///
    /// Returns the material assigned to the object, or the default material
    /// if no material is assigned or the assigned material doesn't exist.
    pub fn get_material_for_object(&self, object: &Object) -> &Material {
        self.material_manager
            .get_material_for_object(object.get_material_id())
    }

    /// Lists all available materials
    pub fn list_materials(&self) -> Vec<&String> {
        self.material_manager.list_materials()
    }

    /// Gets the material manager for advanced material operations
    pub fn get_material_manager(&self) -> &MaterialManager {
        &self.material_manager
    }

    /// Gets mutable access to the material manager
    pub fn get_material_manager_mut(&mut self) -> &mut MaterialManager {
        &mut self.material_manager
    }

    // UI helper methods

    /// Gets all object names for UI display
    pub fn get_object_names(&self) -> Vec<String> {
        self.objects.iter().map(|obj| obj.name.clone()).collect()
    }

    /// Gets the total number of objects
    pub fn get_object_count(&self) -> usize {
        self.objects.len()
    }

    /// Gets mutable reference to an object by index
    pub fn get_object_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.objects.get_mut(index)
    }

    /// Gets immutable reference to an object by index
    pub fn get_object(&self, index: usize) -> Option<&Object> {
        self.objects.get(index)
    }

    /// Applies UI transform changes and updates GPU buffers
    ///
    /// Should be called each frame after UI updates to sync transform
    /// changes from the UI to the actual object transforms and GPU.
    pub fn apply_ui_transforms_and_update_gpu(&mut self, queue: &wgpu::Queue) {
        for object in &mut self.objects {
            if object.visible {
                object.apply_ui_transform();
                object.update_transform(queue);
            }
        }
    }

    /// Assigns a material to an object by index
    ///
    /// # Arguments
    /// * `object_index` - Index of the object
    /// * `material_id` - ID of the material to assign
    pub fn assign_material_to_object(&mut self, object_index: usize, material_id: &str) {
        if let Some(object) = self.objects.get_mut(object_index) {
            object.set_material(material_id);
        }
    }

    /// Gets statistics about the scene
    pub fn get_statistics(&self) -> SceneStatistics {
        let total_triangles: u32 = self
            .objects
            .iter()
            .map(|obj| obj.meshes.iter().map(|m| m.index_count / 3).sum::<u32>())
            .sum();

        let total_vertices: u32 = self
            .objects
            .iter()
            .map(|obj| obj.meshes.iter().map(|m| m.vertex_count).sum::<u32>())
            .sum();

        SceneStatistics {
            object_count: self.objects.len(),
            material_count: self.material_manager.list_materials().len(),
            total_triangles,
            total_vertices,
        }
    }

    pub fn ensure_unique_name(&mut self, desired_name: &str) -> String {
        let mut counter = 0;
        let mut test_name = desired_name.to_string();

        while self.objects.iter().any(|obj| obj.name == test_name) {
            counter += 1;
            test_name = format!("{} ({})", desired_name, counter);
        }

        test_name
    }
}

/// Scene statistics for debugging and UI display
#[derive(Debug)]
pub struct SceneStatistics {
    pub object_count: usize,
    pub material_count: usize,
    pub total_triangles: u32,
    pub total_vertices: u32,
}
