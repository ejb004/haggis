// Visualization Shader
// Dedicated shader for visualization components

struct CameraUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct TransformUniform {
    model: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(2) 
var<uniform> transform: TransformUniform;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = input.tex_coords;
    
    // Apply model transform first, then camera transform (same as PBR shader)
    let world_position = transform.model * vec4<f32>(input.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, input.tex_coords);
}