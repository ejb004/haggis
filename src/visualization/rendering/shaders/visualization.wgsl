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

// Filter mode and coloring mode uniforms
struct FilterUniforms {
    filter_mode: u32,   // 0 = nearest/sharp, 1 = linear/smooth
    coloring_mode: u32, // 0 = vorticity, 1 = air_speed
    grid_width: u32,
    grid_height: u32,
};

@group(1) @binding(4)
var<uniform> filter_uniforms: FilterUniforms;

// Convert vorticity value to scientific diverging colormap
// Negative vorticity (clockwise) = Green -> Black (at zero) -> Red = Positive vorticity (counter-clockwise)
// Smooth scientific color scale with proper interpolation
fn vorticity_to_color(vorticity: f32) -> vec4<f32> {
    let max_vorticity = 0.3; // Tripled range for LBM vorticity
    let normalized = clamp(vorticity / max_vorticity, -1.0, 1.0);

    // Scientific diverging colormap: Green (-1) -> Black (0) -> Red (+1)
    if (normalized < 0.0) {
        // Negative vorticity: interpolate from black to green
        let intensity = -normalized; // Convert to positive for interpolation
        return vec4<f32>(0.0, intensity, 0.0, 1.0);
    } else if (normalized > 0.0) {
        // Positive vorticity: interpolate from black to red
        let intensity = normalized;
        return vec4<f32>(intensity, 0.0, 0.0, 1.0);
    } else {
        // Zero vorticity: Black
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}

// Convert velocity magnitude to speed color
// Low speed = Blue, High speed = Red
// Smooth gradient through spectrum
fn velocity_to_color(speed: f32) -> vec4<f32> {
    let max_speed = 0.9; // Tripled range for realistic LBM velocity magnitudes
    let normalized = clamp(speed / max_speed, 0.0, 1.0);

    // Blue to red color mapping for speed visualization
    // Blue (0,0,1) -> Cyan (0,1,1) -> Green (0,1,0) -> Yellow (1,1,0) -> Red (1,0,0)
    let r = clamp(2.0 * normalized - 0.5, 0.0, 1.0);
    let g = clamp(2.0 * (1.0 - abs(normalized - 0.5)), 0.0, 1.0);
    let b = clamp(1.5 - 2.0 * normalized, 0.0, 1.0);

    return vec4<f32>(r, g, b, 1.0);
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
                let value = f32(cell_value);

                // Apply coloring based on mode
                if (filter_uniforms.coloring_mode == 0u) {
                    // Vorticity mode
                    return vorticity_to_color(value);
                } else {
                    // Air speed mode
                    return velocity_to_color(value);
                }
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
            let value = mix(top, bottom, fy);

            // Apply coloring based on mode
            if (filter_uniforms.coloring_mode == 0u) {
                // Vorticity mode
                return vorticity_to_color(value);
            } else {
                // Air speed mode
                return velocity_to_color(value);
            }
        }
    } else {
        // Texture-based rendering (CPU data path) with coloring support
        let raw_value = textureSample(t_diffuse, s_diffuse, input.tex_coords);

        // Use the red channel as the encoded data value
        let encoded_value = raw_value.r;

        // Apply coloring based on mode
        if (filter_uniforms.coloring_mode == 0u) {
            // Vorticity mode - decode from [0,1] back to [-max,max]
            let max_vorticity = 0.3; // Tripled range for LBM vorticity
            let decoded_vorticity = (encoded_value * 2.0 - 1.0) * max_vorticity;
            return vorticity_to_color(decoded_vorticity);
        } else {
            // Air speed mode - decode from [0,1] back to [0,max]
            let max_speed = 0.9; // Tripled range for realistic LBM velocity magnitudes
            let decoded_speed = encoded_value * max_speed;
            return velocity_to_color(decoded_speed);
        }
    }
}