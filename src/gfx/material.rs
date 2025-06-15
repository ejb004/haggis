use wgpu::Device;

use crate::wgpu_utils::{
    binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc},
    binding_types,
    uniform_buffer::UniformBuffer,
};

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

pub struct MaterialBindings {
    bind_group_layout: BindGroupLayoutWithDesc,
    bind_group: Option<wgpu::BindGroup>,
}

impl MaterialBindings {
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = BindGroupLayoutBuilder::new()
            .next_binding_rendering(binding_types::uniform())
            .create(&device, "Material Bind Group");

        MaterialBindings {
            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn create_bind_group(&mut self, device: &wgpu::Device, ubo: &MaterialUBO) {
        self.bind_group = Some(
            BindGroupBuilder::new(&self.bind_group_layout)
                .resource(ubo.binding_resource())
                .create(&device, "Global Bind Group"),
        );
    }

    pub fn bind_group_layouts(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout.layout
    }

    pub fn bind_groups(&self) -> &wgpu::BindGroup {
        &self
            .bind_group
            .as_ref()
            .expect("Bind group has not been created yet!")
    }
}

pub struct Material {
    // add id later !
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub emissive: [f32; 3],

    material_ubo: Option<MaterialUBO>,
    material_bindings: Option<MaterialBindings>,
}

impl Material {
    pub fn new(base_color: [f32; 4], metallic: f32, roughness: f32) -> Self {
        Self {
            base_color,
            metallic,
            roughness,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive: [0.0, 0.0, 0.0],
            material_ubo: None,
            material_bindings: None,
        }
    }

    pub fn update(&mut self, device: &Device) {
        let ubo = MaterialUBO::new(device);

        let mut bindings = MaterialBindings::new(device);
        bindings.create_bind_group(device, &ubo);

        self.material_ubo = Some(ubo);
        self.material_bindings = Some(bindings);
    }
}
