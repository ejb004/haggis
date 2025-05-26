struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

// Hard-coded material properties (for now)
const MATERIAL_BASE_COLOR: vec3<f32> = vec3<f32>(0.7, 0.7, 0.8);  // Light blue-gray
const MATERIAL_METALLIC: f32 = 0.1;      // Slightly metallic
const MATERIAL_ROUGHNESS: f32 = 0.4;     // Medium roughness
const MATERIAL_EMISSIVE: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);  // No emission

// Hard-coded lighting setup
const LIGHT_POSITION: vec3<f32> = vec3<f32>(3.0, 4.0, 2.0);
const LIGHT_COLOR: vec3<f32> = vec3<f32>(1.0, 0.95, 0.8);  // Warm white
const LIGHT_INTENSITY: f32 = 15.0;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) view_position: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let world_position = vec4<f32>(model.position, 1.0);
    
    out.clip_position = camera.view_proj * world_position;
    out.world_position = world_position.xyz;
    out.world_normal = normalize(model.normal);
    out.view_position = camera.view_pos.xyz;
    
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
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);
    return ggx1 * ggx2;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn calculate_pbr_lighting(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_pos: vec3<f32>,
    light_color: vec3<f32>,
    light_intensity: f32,
    base_color: vec3<f32>,
    metallic: f32,
    roughness: f32
) -> vec3<f32> {
    let light_dir = normalize(light_pos - world_pos);
    let half_dir = normalize(view_dir + light_dir);
    
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let v_dot_h = max(dot(view_dir, half_dir), 0.0);
    
    // Calculate distance attenuation
    let distance = length(light_pos - world_pos);
    let attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);
    let radiance = light_color * light_intensity * attenuation;
    
    // Calculate F0 (surface reflection at zero incidence)
    var f0 = vec3<f32>(0.04);
    f0 = mix(f0, base_color, metallic);
    
    // Cook-Torrance BRDF
    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(v_dot_h, f0);
    
    let numerator = ndf * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.0001; // Prevent divide by zero
    let specular = numerator / denominator;
    
    // Energy conservation
    let ks = f; // Specular contribution
    var kd = vec3<f32>(1.0) - ks; // Diffuse contribution
    kd *= 1.0 - metallic; // Metals have no diffuse lighting
    
    let diffuse = base_color / 3.14159265;
    
    return (kd * diffuse + specular) * radiance * n_dot_l;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(in.view_position - in.world_position);
    
    // Use hard-coded material values
    let base_color = MATERIAL_BASE_COLOR;
    let metallic = MATERIAL_METALLIC;
    let roughness = max(MATERIAL_ROUGHNESS, 0.04); // Prevent division by zero
    
    // Ambient lighting (simple IBL approximation)
    let ambient_strength = 0.03;
    let ambient = vec3<f32>(0.1, 0.1, 0.15) * base_color * ambient_strength;
    
    // Calculate lighting from single hard-coded light
    let total_lighting = calculate_pbr_lighting(
        in.world_position,
        normal,
        view_dir,
        LIGHT_POSITION,
        LIGHT_COLOR,
        LIGHT_INTENSITY,
        base_color,
        metallic,
        roughness
    );
    
    // Add emissive (currently zero)
    let emissive_contribution = MATERIAL_EMISSIVE;
    
    // Final color with ambient
    var final_color = ambient + total_lighting + emissive_contribution;
    
    // Simple tone mapping (Reinhard)
    final_color = final_color / (final_color + vec3<f32>(1.0));
    
    // Gamma correction
    final_color = pow(final_color, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(final_color, 1.0);
}