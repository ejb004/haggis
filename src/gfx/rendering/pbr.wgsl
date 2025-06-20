// PBR shader with shadow mapping support

struct GlobalUniform {
    view_position: vec4<f32>,         // Camera position (homogeneous coordinates)
    view_proj: mat4x4<f32>,           // Camera view-projection matrix
    light_position: vec3<f32>,        // Light position
    _padding1: f32,
    light_color: vec3<f32>,           // Light color
    light_intensity: f32,             // Light intensity
    light_view_proj: mat4x4<f32>,     // Light's view-projection matrix for shadows
};

struct Transform {
    model: mat4x4<f32>,
};

struct Material {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    normal_scale: f32,
    occlusion_strength: f32,
    emissive: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(0) var<uniform> global: GlobalUniform;
@group(1) @binding(0) var<uniform> transform: Transform;
@group(2) @binding(0) var<uniform> material: Material;
@group(3) @binding(0) var shadow_map: texture_depth_2d;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) light_space_position: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply transform to position
    let world_position: vec4<f32> = transform.model * vec4<f32>(model.position, 1.0);
    
    out.world_position = world_position.xyz;
    out.clip_position = global.view_proj * world_position;
    out.light_space_position = global.light_view_proj * world_position;
    
    // Apply transform to normal (extract normal matrix from model matrix)
    let normal_matrix = mat3x3<f32>(
        normalize(transform.model[0].xyz),
        normalize(transform.model[1].xyz),
        normalize(transform.model[2].xyz)
    );
    out.world_normal = normalize(normal_matrix * model.normal);
    
    return out;
}

// PBR utility functions
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
    return ggx1 * ggx2;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

// Simple 4-sample PCF focused on anti-acne
fn calculate_shadow_simple_pcf(light_space_pos: vec4<f32>, normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    var ndc = light_space_pos.xyz / light_space_pos.w;
    let shadow_coord = vec3<f32>(
        ndc.x * 0.5 + 0.5,
        -ndc.y * 0.5 + 0.5,
        ndc.z
    );
    
    if (shadow_coord.x < 0.0 || shadow_coord.x > 1.0 || 
        shadow_coord.y < 0.0 || shadow_coord.y > 1.0) {
        return 1.0;
    }
    
    // Advanced bias calculation to prevent shadow acne
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    
    // Multiple bias techniques combined:
    // 1. Base bias for floating point precision
    let base_bias = 0.0002;
    
    // 2. Slope-dependent bias (higher bias for steep angles)
    let slope_bias = 0.003 * sqrt(1.0 - n_dot_l * n_dot_l);
    
    // 3. Distance-dependent bias (objects further from light need more bias)
    let light_distance = length(global.light_position - shadow_coord.xyz);
    let distance_bias = 0.00001 * light_distance;
    
    let total_bias = base_bias + slope_bias + distance_bias;
    
    let texel_size = 1.0 / 2048.0;
    
    // Simple 2x2 PCF for basic soft shadows
    let s1 = textureSampleCompare(shadow_map, shadow_sampler, 
        shadow_coord.xy + vec2<f32>(-0.5, -0.5) * texel_size, 
        shadow_coord.z - total_bias);
    let s2 = textureSampleCompare(shadow_map, shadow_sampler, 
        shadow_coord.xy + vec2<f32>(0.5, -0.5) * texel_size, 
        shadow_coord.z - total_bias);
    let s3 = textureSampleCompare(shadow_map, shadow_sampler, 
        shadow_coord.xy + vec2<f32>(-0.5, 0.5) * texel_size, 
        shadow_coord.z - total_bias);
    let s4 = textureSampleCompare(shadow_map, shadow_sampler, 
        shadow_coord.xy + vec2<f32>(0.5, 0.5) * texel_size, 
        shadow_coord.z - total_bias);
    
    return (s1 + s2 + s3 + s4) / 4.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = material.base_color.rgb;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04);
    
    let n = normalize(in.world_normal);
    let v = normalize(global.view_position.xyz - in.world_position);
    let l = normalize(global.light_position - in.world_position);
    let h = normalize(v + l);
    
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);
    
    // Calculate shadow factor
    let shadow_factor = calculate_shadow_simple_pcf(in.light_space_position, n, l);
    
    // Calculate F0 for dielectric/metallic materials
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    
    // Cook-Torrance BRDF
    let d = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(v_dot_h, f0);
    
    let numerator = d * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.001;
    let specular = numerator / denominator;
    
    let ks = f;
    let kd = (vec3<f32>(1.0) - ks) * (1.0 - metallic);
    
    // APPROACH: Stronger directional contrast + secondary light source
    
    // Main light (stronger and more directional)
    let light_distance = length(global.light_position - in.world_position);
    let attenuation = 1.0 / (1.0 + 0.015 * light_distance); // Slightly less falloff
    let radiance = global.light_color * global.light_intensity * attenuation * 12.0; // Stronger main light
    
    let diffuse = albedo / 3.14159265;
    let main_lighting = (kd * diffuse + specular) * radiance * n_dot_l * shadow_factor;
    
    // Secondary "fill" light from opposite direction (slightly stronger)
    let fill_light_dir = normalize(vec3<f32>(-global.light_position.x, global.light_position.y * 0.5, -global.light_position.z));
    let n_dot_fill = max(dot(n, fill_light_dir), 0.0);
    let fill_strength = 0.3; // Slightly stronger fill light
    let fill_lighting = (kd * diffuse) * global.light_color * fill_strength * n_dot_fill;
    
    // Slightly higher ambient
    let ambient = vec3<f32>(0.10) * albedo; // Bumped from 0.08 to 0.10
    
    // Enhanced normal-based shading for surface detail
    let normal_detail = abs(dot(n, normalize(vec3<f32>(1.0, 1.0, 1.0)))) * 0.1;
    let detail_lighting = normal_detail * albedo;
    
    // Combine lighting with stronger contrast
    let final_color = main_lighting + fill_lighting + ambient + detail_lighting + material.emissive;
    
    // Adjusted tone mapping to preserve contrast
    let exposure = 0.9;
    let mapped = 1.0 - exp(-final_color * exposure);
    
    // Slight contrast boost to make lighting more dramatic
    let contrast_mapped = mapped * mapped * (3.0 - 2.0 * mapped); // S-curve for contrast
    
    // Gamma correction
    let gamma_corrected = pow(contrast_mapped, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(gamma_corrected, material.base_color.a);
}