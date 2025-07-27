// Instanced PBR Shader for Haggis 3D Engine
// Supports efficient rendering of many similar objects with per-instance transforms and colors

struct GlobalUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding1: f32,
    light_dir: vec3<f32>,
    _padding2: f32,
    light_color: vec3<f32>,
    _padding3: f32,
}

struct MaterialUniforms {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _padding: vec2<f32>,
}

// Vertex input from mesh
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

// Instance input (per-instance data)
struct InstanceInput {
    @location(2) transform_0: vec4<f32>,
    @location(3) transform_1: vec4<f32>, 
    @location(4) transform_2: vec4<f32>,
    @location(5) transform_3: vec4<f32>,
    @location(6) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) instance_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> global: GlobalUniforms;

@group(1) @binding(0) 
var<uniform> material: MaterialUniforms;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // Reconstruct instance transform matrix
    let transform = mat4x4<f32>(
        instance.transform_0,
        instance.transform_1,
        instance.transform_2,
        instance.transform_3,
    );
    
    // Transform vertex position to world space
    let world_position = transform * vec4<f32>(vertex.position, 1.0);
    
    // Transform normal to world space (use 3x3 part of transform)
    let normal_matrix = mat3x3<f32>(
        transform[0].xyz,
        transform[1].xyz,
        transform[2].xyz,
    );
    let world_normal = normalize(normal_matrix * vertex.normal);
    
    // Project to clip space
    let clip_position = global.view_proj * world_position;
    
    return VertexOutput(
        clip_position,
        world_position.xyz,
        world_normal,
        instance.color,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(-global.light_dir);
    let view_dir = normalize(global.camera_pos - in.world_position);
    let normal = normalize(in.world_normal);
    
    // Simple Lambertian lighting
    let ndotl = max(dot(normal, light_dir), 0.0);
    
    // Combine material base color with instance color
    let base_color = material.base_color * in.instance_color;
    
    // Simple diffuse lighting
    let diffuse = base_color.rgb * ndotl * global.light_color;
    let ambient = base_color.rgb * 0.1;
    
    let final_color = diffuse + ambient;
    
    // Use instance color alpha for transparency
    return vec4<f32>(final_color, base_color.a);
}