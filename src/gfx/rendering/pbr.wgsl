// Clean PBR shader without normal coloring

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct Transform {
    model: mat4x4<f32>,
}

struct Material {
    base_color: vec4<f32>,          // RGBA base color
    metallic: f32,                  // Metallic factor
    roughness: f32,                 // Roughness factor  
    normal_scale: f32,              // Normal map strength
    occlusion_strength: f32,        // Ambient occlusion strength
    emissive: vec3<f32>,            // Emissive color (RGB)
    _padding: f32,                  // Padding for alignment
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> transform: Transform;
@group(2) @binding(0) var<uniform> material: Material;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}

const LIGHT: Light = Light(
    vec3<f32>(10.0, 10.0, 10.0),
    vec3<f32>(1.0, 1.0, 1.0)
);

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply transform to position
    let world_position: vec4<f32> = transform.model * vec4<f32>(model.position, 1.0);
    
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    
    // Apply transform to normal
    let normal_matrix = mat3x3<f32>(
        transform.model[0].xyz,
        transform.model[1].xyz,
        transform.model[2].xyz
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = material.base_color.rgb;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04); // Prevent division by zero
    
    let n = normalize(in.world_normal);
    let v = normalize(camera.view_pos.xyz - in.world_position);
    let l = normalize(LIGHT.position - in.world_position);
    let h = normalize(v + l);
    
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);
    
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
    
    // Reduced brightness lighting
    let light_distance = length(LIGHT.position - in.world_position);
    let attenuation = 1.0 / (1.0 + 0.02 * light_distance); // Slightly more falloff
    let radiance = LIGHT.color * attenuation * 10.0; // Reduced from 15.0 to 8.0 -----------------> Can Change
    
    let diffuse = albedo / 3.14159265;
    let color = (kd * diffuse + specular) * radiance * n_dot_l;
    
    // Fresnel-based fake shadows for depth perception
    let fresnel_depth = 1.0 - n_dot_v; // Edge surfaces are darker
    let shadow_strength = 0.9; // How strong the fake shadows are -------------------------------> Can Change
    let fake_shadow = 1.0 - (fresnel_depth * shadow_strength);
    
    // Moderate ambient lighting with fake shadows applied
    let ambient = vec3<f32>(0.15) * albedo * fake_shadow;
    let final_color = ambient + color * fake_shadow;
    
    // Add emissive
    let result = final_color + material.emissive;
    
    // Simpler tone mapping to reduce banding
    let exposure = 0.8; // Slightly darker exposure
    let mapped = 1.0 - exp(-result * exposure);
    
    // Gamma correction
    let gamma_corrected = pow(mapped, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(gamma_corrected, material.base_color.a);
}