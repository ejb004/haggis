// Shadow depth pass - renders scene from light's perspective
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
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let world_position = transform.model * vec4<f32>(model.position, 1.0);
    out.clip_position = global.light_view_proj * world_position;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Store linear depth in red channel
    let depth = in.clip_position.z / in.clip_position.w;
    let linear_depth = depth * 0.5 + 0.5; // Convert from [-1,1] to [0,1]
    
    return vec4<f32>(linear_depth, 0.0, 0.0, 1.0);
}