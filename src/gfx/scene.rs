use wgpu::Device;

use crate::gfx::object::Mesh;

use super::{
    camera::camera_utils::CameraManager,
    material::Material,
    object::{self, Object},
};

pub struct Scene {
    pub camera_manager: CameraManager,
    pub objects: Vec<Object>,
    pub materials: Vec<Material>,
}

impl Scene {
    pub fn new(camera_manager: CameraManager) -> Self {
        Self {
            camera_manager,
            objects: Vec::new(),
            materials: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        self.camera_manager.camera.update_view_proj();
    }

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

        let materials = materials.expect("Failed to load MTL file");

        println!("Number of models          = {}", models.len());
        println!("Number of materials       = {}", materials.len());

        let mut meshes = Vec::new();

        for (i, m) in models.iter().enumerate() {
            let mesh = &m.mesh;

            println!("model[{}].name             = \'{}\'", i, m.name);
            println!("model[{}].mesh.material_id = {:?}", i, mesh.material_id);

            // DEBUG: Print what we're getting from tobj
            println!(
                "Positions: {} ({} vertices)",
                mesh.positions.len(),
                mesh.positions.len() / 3
            );
            println!(
                "Normals: {} ({} normals)",
                mesh.normals.len(),
                mesh.normals.len() / 3
            );
            println!(
                "Indices: {} ({} triangles)",
                mesh.indices.len(),
                mesh.indices.len() / 3
            );

            // Use normals from OBJ if available, otherwise calculate them
            let normals = if !mesh.normals.is_empty() && mesh.normals.len() == mesh.positions.len()
            {
                println!("Using normals from OBJ file");
                mesh.normals.clone()
            } else {
                println!("No valid normals in OBJ file, calculating face normals...");
                Mesh::calculate_face_normals(&mesh.positions, &mesh.indices)
            };

            let our_mesh = Mesh::new(mesh.positions.clone(), normals, mesh.indices.clone());
            meshes.push(our_mesh);
        }

        self.objects.push(Object::new(meshes));
    }

    pub fn add_material(&mut self, base_color: [f32; 4], metallic: f32, roughness: f32) {
        let material = Material::new(base_color, metallic, roughness);
        self.materials.push(material);
    }

    pub fn init_gpu_resources(&mut self, device: &Device) {
        for object in self.objects.iter_mut() {
            object.init_gpu_resources(device);
        }
    }

    pub fn update_all_transforms(&mut self, queue: &wgpu::Queue) {
        for object in &mut self.objects {
            if object.gpu_resources.is_some() {
                object.update_transform(queue);
            }
        }
    }

    // ui stuff

    pub fn get_object_names(&self) -> Vec<String> {
        self.objects.iter().map(|obj| obj.name.clone()).collect()
    }

    pub fn get_object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn get_object_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.objects.get_mut(index)
    }

    pub fn apply_ui_transforms_and_update_gpu(&mut self, queue: &wgpu::Queue) {
        for object in &mut self.objects {
            if object.visible {
                object.apply_ui_transform();
                object.update_transform(queue);
            }
        }
    }
}
