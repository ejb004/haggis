// Depth to Color conversion shader
// Converts depth texture to color texture for blurring

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var depth_sampler: sampler;

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
    // Sample depth and convert to color
    let depth = textureSample(depth_texture, depth_sampler, in.uv);
    
    // VERY obvious debug: 
    // If depth is exactly 1.0 (cleared value) = RED
    // If depth is anything else = GREEN
    if (abs(depth - 1.0) < 0.0001) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0); // RED = no geometry rendered
    } else {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0); // GREEN = geometry was rendered
    }
}