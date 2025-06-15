use std::ops::Range;

use wgpu::Device;

use crate::app::HaggisApp;

use super::vertex::Vertex3D;

pub struct Mesh {
    vertices: Vec<Vertex3D>,
    indices: Vec<u32>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    index_count: u32,
}

impl Mesh {
    pub fn new(positions: Vec<f32>, normals: Vec<f32>, indices: Vec<u32>) -> Self {
        let index_count = indices.len() as u32;

        // Create Vec<Vertex3D> instead of interleaved Vec<f32>
        let mut vertices = Vec::new();
        for i in 0..positions.len() / 3 {
            vertices.push(Vertex3D {
                position: [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]],
                normal: [normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]],
            });
        }

        Self {
            vertices,
            indices,
            vertex_buffer: None,
            index_buffer: None,
            index_count,
        }
    }

    // Helper function to calculate face normals if OBJ doesn't have them
    pub fn calculate_face_normals(positions: &[f32], indices: &[u32]) -> Vec<f32> {
        println!("please no normals no!!!");
        let vertex_count = positions.len() / 3;
        let mut normals = vec![0.0; positions.len()]; // Same length as positions
        let mut counts = vec![0; vertex_count]; // Count contributions per vertex

        // For each triangle, calculate face normal and add to vertices
        for triangle in indices.chunks(3) {
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;

            // Get triangle vertices
            let v0 = [
                positions[i0 * 3],
                positions[i0 * 3 + 1],
                positions[i0 * 3 + 2],
            ];
            let v1 = [
                positions[i1 * 3],
                positions[i1 * 3 + 1],
                positions[i1 * 3 + 2],
            ];
            let v2 = [
                positions[i2 * 3],
                positions[i2 * 3 + 1],
                positions[i2 * 3 + 2],
            ];

            // Calculate face normal using cross product
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let face_normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];

            // Add face normal to each vertex of the triangle
            for &vertex_idx in &[i0, i1, i2] {
                normals[vertex_idx * 3] += face_normal[0];
                normals[vertex_idx * 3 + 1] += face_normal[1];
                normals[vertex_idx * 3 + 2] += face_normal[2];
                counts[vertex_idx] += 1;
            }
        }

        // Average and normalize the normals
        for i in 0..vertex_count {
            if counts[i] > 0 {
                normals[i * 3] /= counts[i] as f32;
                normals[i * 3 + 1] /= counts[i] as f32;
                normals[i * 3 + 2] /= counts[i] as f32;

                // Normalize the normal vector
                let length = (normals[i * 3].powi(2)
                    + normals[i * 3 + 1].powi(2)
                    + normals[i * 3 + 2].powi(2))
                .sqrt();
                if length > 0.0 {
                    normals[i * 3] /= length;
                    normals[i * 3 + 1] /= length;
                    normals[i * 3 + 2] /= length;
                }
            }
        }

        normals
    }
}

use cgmath::{Deg, Matrix4, SquareMatrix, Vector3};

// 1. Create a builder struct that holds a reference to the app and the object index
pub struct ObjectBuilder<'a> {
    app: &'a mut HaggisApp,
    object_index: usize,
}

impl<'a> ObjectBuilder<'a> {
    pub fn new(app: &'a mut HaggisApp, object_index: usize) -> Self {
        Self { app, object_index }
    }

    pub fn with_transform(self, position: [f32; 3], scale: f32, rotation_y: f32) -> Self {
        // Apply transform to the object
        if let Some(object) = self.app.app_state.scene.objects.get_mut(self.object_index) {
            use cgmath::{Deg, Vector3};

            object.set_transform_trs(
                Vector3::new(position[0], position[1], position[2]),
                Deg(rotation_y),
                scale,
            );
        }
        self
    }

    pub fn with_position(self, position: [f32; 3]) -> Self {
        if let Some(object) = self.app.app_state.scene.objects.get_mut(self.object_index) {
            use cgmath::Vector3;
            object.set_translation(Vector3::new(position[0], position[1], position[2]));
        }
        self
    }

    pub fn with_scale(self, scale: f32) -> Self {
        if let Some(object) = self.app.app_state.scene.objects.get_mut(self.object_index) {
            object.set_scale(scale);
        }
        self
    }

    pub fn with_rotation_y(self, rotation_y: f32) -> Self {
        if let Some(object) = self.app.app_state.scene.objects.get_mut(self.object_index) {
            use cgmath::Deg;
            object.set_rotation_y(Deg(rotation_y));
        }
        self
    }

    pub fn with_rotation_xyz(self, rotation: [f32; 3]) -> Self {
        if let Some(object) = self.app.app_state.scene.objects.get_mut(self.object_index) {
            use cgmath::Deg;
            object.reset_transform();
            object.rotate_x(Deg(rotation[0]));
            object.rotate_y(Deg(rotation[1]));
            object.rotate_z(Deg(rotation[2]));
        }
        self
    }
}

// GPU resources struct to hold all uniform buffers and bind groups
pub struct ObjectGpuResources {
    pub transform_buffer: wgpu::Buffer,
    pub transform_bind_group: wgpu::BindGroup,
    // Future material support
    pub material_buffer: Option<wgpu::Buffer>,
    pub material_bind_group: Option<wgpu::BindGroup>,
}

#[derive(Clone)]
pub struct UiTransformState {
    pub position: [f32; 3],
    pub rotation: [f32; 3], // degrees
    pub scale: f32,
}

impl Default for UiTransformState {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
        }
    }
}

pub struct Object {
    pub meshes: Vec<Mesh>,
    pub transform: Matrix4<f32>, // cgmath 4x4 transformation matrix
    pub gpu_resources: Option<ObjectGpuResources>, // None until init_gpu_resources called

    pub name: String,
    pub ui_transform: UiTransformState,
    pub visible: bool,
}

impl Object {
    /// Create a new Object with identity transformation
    pub fn new(meshes: Vec<Mesh>) -> Self {
        Self {
            meshes,
            transform: Matrix4::identity(),
            gpu_resources: None,
            name: "Object".to_string(),
            ui_transform: UiTransformState::default(),
            visible: true,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Apply UI transform state to the actual transform matrix
    pub fn apply_ui_transform(&mut self) {
        use cgmath::{Deg, Vector3};

        self.reset_transform();

        // Apply TRS from UI state
        self.set_transform_trs(
            Vector3::new(
                self.ui_transform.position[0],
                self.ui_transform.position[1],
                self.ui_transform.position[2],
            ),
            Deg(self.ui_transform.rotation[1]), // Y rotation
            self.ui_transform.scale,
        );

        // Apply X and Z rotations
        self.rotate_x(Deg(self.ui_transform.rotation[0]));
        self.rotate_z(Deg(self.ui_transform.rotation[2]));
    }

    /// Sync current transform matrix back to UI state (for initialization)
    pub fn sync_transform_to_ui(&mut self) {
        // Extract translation from transform matrix
        let transform_data: &[f32; 16] = self.transform.as_ref();
        self.ui_transform.position[0] = transform_data[12];
        self.ui_transform.position[1] = transform_data[13];
        self.ui_transform.position[2] = transform_data[14];

        // Extract scale (assuming uniform scale)
        let scale_x =
            (transform_data[0].powi(2) + transform_data[1].powi(2) + transform_data[2].powi(2))
                .sqrt();
        self.ui_transform.scale = scale_x;

        // Note: Extracting rotation from matrix is complex, so we'll keep it simple for now
        // Rotations will start at 0 when UI is opened
    }

    /// Set translation
    pub fn set_translation(&mut self, translation: Vector3<f32>) {
        self.transform = Matrix4::from_translation(translation);
    }

    /// Apply translation (multiplies with existing transform)
    pub fn translate(&mut self, translation: Vector3<f32>) {
        self.transform = self.transform * Matrix4::from_translation(translation);
    }

    /// Set uniform scale
    pub fn set_scale(&mut self, scale: f32) {
        self.transform = Matrix4::from_scale(scale);
    }

    /// Set non-uniform scale
    pub fn set_scale_xyz(&mut self, scale: Vector3<f32>) {
        self.transform = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
    }

    /// Set rotation around X axis
    pub fn set_rotation_x(&mut self, angle: Deg<f32>) {
        self.transform = Matrix4::from_angle_x(angle);
    }

    /// Set rotation around Y axis
    pub fn set_rotation_y(&mut self, angle: Deg<f32>) {
        self.transform = Matrix4::from_angle_y(angle);
    }

    /// Set rotation around Z axis
    pub fn set_rotation_z(&mut self, angle: Deg<f32>) {
        self.transform = Matrix4::from_angle_z(angle);
    }

    /// Apply rotation around X axis
    pub fn rotate_x(&mut self, angle: Deg<f32>) {
        self.transform = self.transform * Matrix4::from_angle_x(angle);
    }

    /// Apply rotation around Y axis
    pub fn rotate_y(&mut self, angle: Deg<f32>) {
        self.transform = self.transform * Matrix4::from_angle_y(angle);
    }

    /// Apply rotation around Z axis
    pub fn rotate_z(&mut self, angle: Deg<f32>) {
        self.transform = self.transform * Matrix4::from_angle_z(angle);
    }

    /// Create a complete transform from translation, rotation, and scale
    pub fn set_transform_trs(
        &mut self,
        translation: Vector3<f32>,
        rotation_y: Deg<f32>,
        scale: f32,
    ) {
        let t = Matrix4::from_translation(translation);
        let r = Matrix4::from_angle_y(rotation_y);
        let s = Matrix4::from_scale(scale);
        self.transform = t * r * s; // Order matters: T * R * S
    }

    /// Reset to identity matrix
    pub fn reset_transform(&mut self) {
        self.transform = Matrix4::identity();
    }

    /// Update the transformation matrix and sync to GPU if resources exist
    pub fn update_transform(&mut self, queue: &wgpu::Queue) {
        if let Some(gpu_resources) = &self.gpu_resources {
            // cgmath matrices are column-major, which is what GPU expects
            let transform_data: &[f32; 16] = self.transform.as_ref();

            queue.write_buffer(
                &gpu_resources.transform_buffer,
                0,
                bytemuck::cast_slice(transform_data),
            );
        }
    }

    /// Get the transform bind group for rendering
    pub fn get_transform_bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.gpu_resources
            .as_ref()
            .map(|res| &res.transform_bind_group)
    }

    pub fn init_gpu_resources(&mut self, device: &Device) {
        println!("=== GPU BUFFER CREATION DEBUG ===");

        // Initialize mesh buffers
        for (mesh_idx, mesh) in self.meshes.iter_mut().enumerate() {
            let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
            let index_bytes = bytemuck::cast_slice(&mesh.indices);

            let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: vertex_bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                },
            );

            let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: index_bytes,
                    usage: wgpu::BufferUsages::INDEX,
                },
            );

            mesh.vertex_buffer = Some(vertex_buffer);
            mesh.index_buffer = Some(index_buffer);

            println!("  ✓ Buffers created successfully");
        }

        // Create transform uniform buffer and bind group
        println!("Creating transform uniform resources...");

        // cgmath matrices are already column-major for GPU
        let transform_data: &[f32; 16] = self.transform.as_ref();

        let transform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Transform Uniform Buffer"),
                contents: bytemuck::cast_slice(transform_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
        });

        // Store GPU resources
        self.gpu_resources = Some(ObjectGpuResources {
            transform_buffer,
            transform_bind_group,
            material_buffer: None,
            material_bind_group: None,
        });
    }
}

pub trait DrawObject<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, instances: Range<u32>);
    fn draw_object(&mut self, object: &'a Object);
    fn draw_object_instanced(&mut self, object: &'a Object, instances: Range<u32>);
}

impl<'a, 'b> DrawObject<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, instances: Range<u32>) {
        let vertex_buffer = match &mesh.vertex_buffer {
            Some(buffer) => buffer,
            None => return, // Skip drawing if not uploaded
        };
        let index_buffer = match &mesh.index_buffer {
            Some(buffer) => buffer,
            None => return,
        };

        self.set_vertex_buffer(0, vertex_buffer.slice(..));
        self.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.index_count, 0, instances);
    }

    fn draw_object(&mut self, object: &'b Object) {
        self.draw_object_instanced(object, 0..1);
    }

    fn draw_object_instanced(&mut self, object: &'b Object, instances: Range<u32>) {
        // IMPORTANT: Bind transform for this object (Group 1) before drawing meshes
        if let Some(gpu_resources) = &object.gpu_resources {
            self.set_bind_group(1, &gpu_resources.transform_bind_group, &[]);
        }

        for mesh in &object.meshes {
            self.draw_mesh_instanced(mesh, instances.clone());
        }
    }
}
