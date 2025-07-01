// Proper shadow pass shader - renders depth as color for shadow mapping

struct GlobalUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    light_position: vec3<f32>,
    _padding1: f32,
    light_color: vec3<f32>,
    light_intensity: f32,
    light_view_proj: mat4x4<f32>,
};

struct Transform {
    model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> global: GlobalUniform;
@group(1) @binding(0) var<uniform> transform: Transform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) depth: f32,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform vertex to world space
    let world_position = transform.model * vec4<f32>(model.position, 1.0);
    
    // Transform to light's view space for shadow mapping
    out.clip_position = global.light_view_proj * world_position;
    
    // Pass the depth value to fragment shader
    // Normalize from [-1, 1] to [0, 1] range
    out.depth = out.clip_position.z / out.clip_position.w;
    
    return out;
}

@fragment  
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert depth from [-1, 1] to [0, 1] range
    let normalized_depth = in.depth * 0.5 + 0.5;
    
    // Store depth as grayscale color
    // Closer objects = darker values, farther objects = lighter values
    return vec4<f32>(normalized_depth, normalized_depth, normalized_depth, 1.0);
}