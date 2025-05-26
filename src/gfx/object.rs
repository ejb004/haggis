use std::ops::Range;

use wgpu::Device;

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

pub struct Object {
    pub meshes: Vec<Mesh>,
}

impl Object {
    pub fn init_gpu_resources(&mut self, device: &Device) {
        println!("=== GPU BUFFER CREATION DEBUG ===");
        for (mesh_idx, mesh) in self.meshes.iter_mut().enumerate() {
            println!("Creating buffers for mesh {}:", mesh_idx);
            println!(
                "  Vertices: {} (size: {} bytes)",
                mesh.vertices.len(),
                mesh.vertices.len() * std::mem::size_of::<Vertex3D>()
            );
            println!(
                "  Indices: {} (size: {} bytes)",
                mesh.indices.len(),
                mesh.indices.len() * std::mem::size_of::<u32>()
            );

            // Show what bytemuck will convert
            let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
            let index_bytes = bytemuck::cast_slice(&mesh.indices);
            println!("  Vertex buffer bytes: {}", vertex_bytes.len());
            println!("  Index buffer bytes: {}", index_bytes.len());

            // Show first few raw bytes
            println!(
                "  First vertex as bytes: {:?}",
                &vertex_bytes[0..24.min(vertex_bytes.len())]
            );
            println!(
                "  First few indices as bytes: {:?}",
                &index_bytes[0..12.min(index_bytes.len())]
            );

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
        println!("=== END GPU BUFFER DEBUG ===\n");
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

        // println!(
        //     "Drawing mesh: {} indices ({} triangles)",
        //     mesh.index_count,
        //     mesh.index_count / 3
        // );

        self.set_vertex_buffer(0, vertex_buffer.slice(..));
        self.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.index_count, 0, instances);
    }

    fn draw_object(&mut self, object: &'b Object) {
        self.draw_object_instanced(object, 0..1);
    }

    fn draw_object_instanced(&mut self, object: &'b Object, instances: Range<u32>) {
        for mesh in &object.meshes {
            self.draw_mesh_instanced(mesh, instances.clone());
        }
    }
}
