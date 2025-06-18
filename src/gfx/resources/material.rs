//! Material system for PBR rendering
//!
//! Provides material definitions and centralized management with GPU resource handling.
//! Materials are stored in MaterialManager and objects reference them by ID.

use std::collections::HashMap;
use wgpu::Device;

use crate::wgpu_utils::{
    binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc},
    binding_types,
    uniform_buffer::UniformBuffer,
};

/// Material ID for referencing materials
pub type MaterialId = String;

/// GPU uniform data for materials
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive: [f32; 3],
    _padding: f32,
}

type MaterialUBO = UniformBuffer<MaterialUniform>;

/// Material bind group management
pub struct MaterialBindings {
    bind_group_layout: BindGroupLayoutWithDesc,
    bind_group: Option<wgpu::BindGroup>,
}

impl MaterialBindings {
    pub fn new(device: &Device) -> Self {
        // Create the layout using your existing builder but with explicit FRAGMENT visibility
        let bind_group_layout = BindGroupLayoutBuilder::new()
            .next_binding_fragment(binding_types::uniform()) // Use fragment-only binding
            .create(device, "Material Bind Group");

        MaterialBindings {
            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn create_bind_group(&mut self, device: &Device, ubo: &MaterialUBO) {
        self.bind_group = Some(
            BindGroupBuilder::new(&self.bind_group_layout)
                .resource(ubo.binding_resource())
                .create(device, "Material Bind Group"),
        );
    }

    pub fn bind_group_layouts(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout.layout
    }

    pub fn bind_groups(&self) -> &wgpu::BindGroup {
        self.bind_group
            .as_ref()
            .expect("Bind group has not been created yet!")
    }
}

/// Material definition with PBR properties
///
/// Contains material properties and GPU resources. Materials are stored
/// centrally in MaterialManager and shared between objects.
pub struct Material {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive: [f32; 3],

    // GPU resources - shared by all objects using this material
    material_ubo: Option<MaterialUBO>,
    material_bindings: Option<MaterialBindings>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive: [0.0, 0.0, 0.0],
            material_ubo: None,
            material_bindings: None,
        }
    }
}

impl Material {
    /// Creates a new material with basic PBR properties
    ///
    /// # Arguments
    /// * `name` - Unique name for this material
    /// * `base_color` - RGBA base color
    /// * `metallic` - Metallic factor (0.0 = dielectric, 1.0 = metallic)
    /// * `roughness` - Surface roughness (0.0 = mirror, 1.0 = rough)
    pub fn new(name: &str, base_color: [f32; 4], metallic: f32, roughness: f32) -> Self {
        Self {
            name: name.to_string(),
            base_color,
            metallic: metallic.clamp(0.0, 1.0),
            roughness: roughness.clamp(0.0, 1.0),
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive: [0.0, 0.0, 0.0],
            material_ubo: None,
            material_bindings: None,
        }
    }

    /// Builder pattern: Set base color from RGB values
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.base_color = [r, g, b, self.base_color[3]];
        self
    }

    /// Builder pattern: Set alpha transparency
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.base_color[3] = alpha.clamp(0.0, 1.0);
        self
    }

    /// Builder pattern: Set metallic factor
    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Builder pattern: Set roughness factor
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Builder pattern: Set emissive color
    pub fn with_emission(mut self, r: f32, g: f32, b: f32) -> Self {
        self.emissive = [r, g, b];
        self
    }

    /// Updates GPU resources for this material
    ///
    /// Must be called after material properties change to sync with GPU.
    pub fn update_gpu_resources(&mut self, device: &Device, queue: &wgpu::Queue) {
        // Create uniform buffer if needed
        if self.material_ubo.is_none() {
            self.material_ubo = Some(MaterialUBO::new(device));
        } else {
        }

        // Create bindings if needed
        if self.material_bindings.is_none() {
            let mut bindings = MaterialBindings::new(device);

            bindings.create_bind_group(device, self.material_ubo.as_ref().unwrap());

            self.material_bindings = Some(bindings);
        }

        // Update uniform data
        let uniform_data = MaterialUniform {
            base_color: self.base_color,
            metallic: self.metallic,
            roughness: self.roughness,
            normal_scale: self.normal_scale,
            occlusion_strength: self.occlusion_strength,
            emissive: self.emissive,
            _padding: 0.0,
        };

        if let Some(ubo) = &mut self.material_ubo {
            ubo.update_content(queue, uniform_data);
        }
    }
    /// Gets the bind group for rendering
    pub fn get_bind_group(&self) -> Option<&wgpu::BindGroup> {
        match &self.material_bindings {
            Some(bindings) => match bindings.bind_groups() {
                bind_group => Some(bind_group),
            },
            None => {
                println!(
                    "DEBUG: get_bind_group() for '{}' - no material_bindings",
                    self.name
                );
                None
            }
        }
    }

    /// Gets the bind group layout for pipeline creation
    pub fn get_bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.material_bindings
            .as_ref()
            .map(|b| b.bind_group_layouts())
    }
}

/// Manages all materials in the engine
///
/// Centralized storage for all materials. Objects reference materials by ID
/// rather than storing material data directly, enabling efficient sharing
/// of GPU resources between objects.
pub struct MaterialManager {
    materials: HashMap<MaterialId, Material>,
    default_material_id: MaterialId,
}

impl MaterialManager {
    /// Creates a new material manager with a default material
    pub fn new() -> Self {
        let mut manager = Self {
            materials: HashMap::new(),
            default_material_id: "default".to_string(),
        };

        // Create default material
        let default_material = Material::default();
        manager
            .materials
            .insert("default".to_string(), default_material);

        manager
    }

    /// Adds a material to the library
    ///
    /// # Arguments
    /// * `material` - Material to add
    pub fn add_material(&mut self, material: Material) {
        self.materials.insert(material.name.clone(), material);
    }

    /// Gets a material by ID
    ///
    /// # Arguments
    /// * `id` - Material ID
    ///
    /// # Returns
    /// Reference to the material if found
    pub fn get_material(&self, id: &MaterialId) -> Option<&Material> {
        self.materials.get(id)
    }

    /// Gets a mutable material by ID
    ///
    /// # Arguments
    /// * `id` - Material ID
    ///
    /// # Returns
    /// Mutable reference to the material if found
    pub fn get_material_mut(&mut self, id: &MaterialId) -> Option<&mut Material> {
        self.materials.get_mut(id)
    }

    /// Gets the default material
    pub fn get_default_material(&self) -> &Material {
        self.materials.get(&self.default_material_id).unwrap()
    }

    /// Gets material for an object with fallback to default
    ///
    /// This is the main method used during rendering to get the appropriate
    /// material for an object, handling cases where the object has no material
    /// assigned or the material doesn't exist.
    ///
    /// # Arguments
    /// * `material_id` - Optional material ID from object
    ///
    /// # Returns
    /// Reference to the material (either requested or default)
    pub fn get_material_for_object(&self, material_id: Option<&MaterialId>) -> &Material {
        match material_id {
            Some(id) => self
                .get_material(id)
                .unwrap_or_else(|| self.get_default_material()),
            None => self.get_default_material(),
        }
    }

    /// Creates a new material and adds it to the library
    ///
    /// # Arguments
    /// * `name` - Unique name for the material
    ///
    /// # Returns
    /// Mutable reference to the created material
    pub fn create_material(&mut self, name: &str) -> &mut Material {
        let material = Material::new(name, [0.8, 0.8, 0.8, 1.0], 0.0, 0.5);
        self.materials.insert(name.to_string(), material);
        self.materials.get_mut(name).unwrap()
    }

    /// Lists all material IDs
    pub fn list_materials(&self) -> Vec<&MaterialId> {
        self.materials.keys().collect()
    }

    /// Updates GPU resources for all materials
    ///
    /// Should be called when the GPU context is available or when
    /// materials have been modified.
    pub fn update_all_gpu_resources(&mut self, device: &Device, queue: &wgpu::Queue) {
        for material in self.materials.values_mut() {
            material.update_gpu_resources(device, queue);
        }
    }

    /// Gets material bind group layout for pipeline creation
    ///
    /// Uses the default material's layout as all materials share the same layout.
    pub fn get_bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.get_default_material().get_bind_group_layout()
    }
}

/// Builder for fluent material configuration
pub struct MaterialBuilder<'a> {
    manager: &'a mut MaterialManager,
    material_id: MaterialId,
}

impl<'a> MaterialBuilder<'a> {
    pub(crate) fn new(manager: &'a mut MaterialManager, material_id: &str) -> Self {
        Self {
            manager,
            material_id: material_id.to_string(),
        }
    }

    /// Sets the base color
    pub fn with_color(self, r: f32, g: f32, b: f32) -> Self {
        if let Some(material) = self.manager.get_material_mut(&self.material_id) {
            material.base_color = [r, g, b, material.base_color[3]];
        }
        self
    }

    /// Sets metallic factor
    pub fn with_metallic(self, metallic: f32) -> Self {
        if let Some(material) = self.manager.get_material_mut(&self.material_id) {
            material.metallic = metallic.clamp(0.0, 1.0);
        }
        self
    }

    /// Sets roughness factor
    pub fn with_roughness(self, roughness: f32) -> Self {
        if let Some(material) = self.manager.get_material_mut(&self.material_id) {
            material.roughness = roughness.clamp(0.0, 1.0);
        }
        self
    }

    /// Sets emissive color
    pub fn with_emission(self, r: f32, g: f32, b: f32) -> Self {
        if let Some(material) = self.manager.get_material_mut(&self.material_id) {
            material.emissive = [r, g, b];
        }
        self
    }
}
