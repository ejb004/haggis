// Gaussian blur shader for shadow softening
// Applies a 9-tap gaussian blur to depth-based shadow maps

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Generate fullscreen triangle without vertex buffers
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Generate fullscreen triangle coordinates
    let x = f32(i32(vertex_index) - 1);
    let y = f32(i32(vertex_index & 1u) * 2 - 1);
    
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_size = textureDimensions(input_texture);
    let texel_size = vec2<f32>(1.0) / vec2<f32>(texture_size);
    
    // 9-tap Gaussian blur weights (normalized)
    let weights = array<f32, 9>(
        0.0625, 0.125, 0.0625,
        0.125,  0.25,  0.125,
        0.0625, 0.125, 0.0625
    );
    
    // Sample offsets for 3x3 kernel
    var offsets = array<vec2<f32>, 9>(
        vec2<f32>(-2.0, -2.0), vec2<f32>(0.0, -2.0), vec2<f32>(2.0, -2.0),
        vec2<f32>(-2.0,  0.0), vec2<f32>(0.0,  0.0), vec2<f32>(2.0,  0.0),
        vec2<f32>(-2.0,  2.0), vec2<f32>(0.0,  2.0), vec2<f32>(2.0,  2.0)
    );


    
    var result = vec4<f32>(0.0);
    
    // Apply Gaussian blur
    for (var i = 0; i < 9; i++) {
        let sample_uv = in.uv + offsets[i] * texel_size;
        let sample_color = textureSample(input_texture, input_sampler, sample_uv);
        // result += sample_color * weights[i];
    }
    
    return result;
}