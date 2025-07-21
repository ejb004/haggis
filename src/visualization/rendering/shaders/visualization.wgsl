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

// Texture-based rendering (CPU data path)
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

// GPU buffer-based rendering (direct compute buffer path)
@group(1) @binding(3)
var<storage, read> gpu_data_buffer: array<u32>;

// Fragment shader with dual mode support
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Check if we have valid texture dimensions (1x1 indicates dummy texture = GPU mode)
    let tex_dimensions = textureDimensions(t_diffuse);
    
    if (tex_dimensions.x == 1u && tex_dimensions.y == 1u) {
        // GPU buffer mode - use storage buffer data
        // This is a simple example assuming 128x128 grid - in production this should be parameterized
        let grid_size = 128u;
        let grid_x = u32(input.tex_coords.x * f32(grid_size));
        let grid_y = u32(input.tex_coords.y * f32(grid_size));
        let index = grid_y * grid_size + grid_x;
        
        // Bounds check to avoid buffer overrun
        if (index < arrayLength(&gpu_data_buffer)) {
            let cell_value = gpu_data_buffer[index];
            let intensity = f32(cell_value);
            // Simple visualization: dead cells = black, live cells = white
            return vec4<f32>(intensity, intensity, intensity, 1.0);
        } else {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0); // Black for out of bounds
        }
    } else {
        // Texture-based rendering (CPU data path)
        return textureSample(t_diffuse, s_diffuse, input.tex_coords);
    }
}