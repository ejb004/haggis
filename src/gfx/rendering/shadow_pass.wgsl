// Shadow pass - depth only rendering
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct GlobalUniform {
    view_position: vec4<f32>,         // Camera position (homogeneous coordinates)
    view_proj: mat4x4<f32>,           // Camera view-projection matrix
    light_position: vec3<f32>,        // Light position
    _padding1: f32,
    light_color: vec3<f32>,           // Light color
    light_intensity: f32,             // Light intensity
    light_view_proj: mat4x4<f32>,     // Light's view-projection matrix for shadows
};

struct ModelUniform {
    model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> global: GlobalUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    let world_position = model.model * vec4<f32>(input.position, 1.0);
    return global.light_view_proj * world_position;
}

// No fragment shader - depth is written automatically