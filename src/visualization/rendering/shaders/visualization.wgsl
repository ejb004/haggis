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

// Filter mode uniform (0 = sharp, 1 = smooth)
struct FilterUniforms {
    filter_mode: u32,  // 0 = nearest/sharp, 1 = linear/smooth
    grid_width: u32,
    grid_height: u32,
    _padding: u32,
};

@group(1) @binding(4)
var<uniform> filter_uniforms: FilterUniforms;

// Convert vorticity value to directional color
// Positive vorticity (counter-clockwise) = Red
// Negative vorticity (clockwise) = Green
// Zero vorticity = Black
fn vorticity_to_color(vorticity: f32) -> vec4<f32> {
    let max_vorticity = 5.0; // Adjusted for airfoil vorticity range
    let normalized = clamp(abs(vorticity) / max_vorticity, 0.0, 1.0);
    
    if (vorticity > 0.0) {
        // Positive vorticity: Red channel
        return vec4<f32>(normalized, 0.0, 0.0, 1.0);
    } else if (vorticity < 0.0) {
        // Negative vorticity: Green channel
        return vec4<f32>(0.0, normalized, 0.0, 1.0);
    } else {
        // Zero vorticity: Black
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}

// Fragment shader with dual mode support
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Check if we have valid texture dimensions (1x1 indicates dummy texture = GPU mode)
    let tex_dimensions = textureDimensions(t_diffuse);
    
    if (tex_dimensions.x == 1u && tex_dimensions.y == 1u) {
        // GPU buffer mode - use storage buffer data with configurable filtering
        let grid_width = filter_uniforms.grid_width;
        let grid_height = filter_uniforms.grid_height;
        
        if (filter_uniforms.filter_mode == 0u) {
            // Sharp/Nearest filtering - sample exact pixel
            let grid_x = u32(input.tex_coords.x * f32(grid_width));
            let grid_y = u32(input.tex_coords.y * f32(grid_height));
            let index = grid_y * grid_width + grid_x;
            
            if (index < arrayLength(&gpu_data_buffer)) {
                let cell_value = gpu_data_buffer[index];
                let vorticity = f32(cell_value);
                return vorticity_to_color(vorticity);
            } else {
                return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }
        } else {
            // Smooth/Linear filtering - bilinear interpolation between 4 neighboring pixels
            let x_scaled = input.tex_coords.x * f32(grid_width) - 0.5;
            let y_scaled = input.tex_coords.y * f32(grid_height) - 0.5;
            
            let x0 = u32(max(0.0, floor(x_scaled)));
            let y0 = u32(max(0.0, floor(y_scaled)));
            let x1 = min(x0 + 1u, grid_width - 1u);
            let y1 = min(y0 + 1u, grid_height - 1u);
            
            let fx = fract(x_scaled);
            let fy = fract(y_scaled);
            
            // Sample the 4 corners
            let idx_00 = y0 * grid_width + x0;
            let idx_01 = y0 * grid_width + x1;
            let idx_10 = y1 * grid_width + x0;
            let idx_11 = y1 * grid_width + x1;
            
            let val_00 = f32(gpu_data_buffer[idx_00]);
            let val_01 = f32(gpu_data_buffer[idx_01]);
            let val_10 = f32(gpu_data_buffer[idx_10]);
            let val_11 = f32(gpu_data_buffer[idx_11]);
            
            // Bilinear interpolation
            let top = mix(val_00, val_01, fx);
            let bottom = mix(val_10, val_11, fx);
            let vorticity = mix(top, bottom, fy);
            
            return vorticity_to_color(vorticity);
        }
    } else {
        // Texture-based rendering (CPU data path)
        return textureSample(t_diffuse, s_diffuse, input.tex_coords);
    }
}