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

// Shadow calculation
fn calculate_shadow(light_space_pos: vec4<f32>) -> f32 {
    // Perspective divide
    var ndc = light_space_pos.xyz / light_space_pos.w;
    
    // Transform to texture coordinates [0, 1]
    let shadow_coord = vec3<f32>(
        ndc.x * 0.5 + 0.5,
        -ndc.y * 0.5 + 0.5, // Flip Y
        ndc.z
    );
    
    // Check if we're outside the shadow map
    if (shadow_coord.x < 0.0 || shadow_coord.x > 1.0 || 
        shadow_coord.y < 0.0 || shadow_coord.y > 1.0) {
        return 1.0; // No shadow
    }
    
    // Sample shadow map with comparison
    let shadow_factor = textureSampleCompare(
        shadow_map, 
        shadow_sampler, 
        shadow_coord.xy, 
        shadow_coord.z - 0.001 // Reduced bias to prevent shadow acne
    );

    // let shadow_factor = 1.0;
    
    return shadow_factor;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = material.base_color.rgb;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04); // Prevent division by zero
    
    let n = normalize(in.world_normal);
    let v = normalize(global.view_position.xyz - in.world_position);
    let l = normalize(global.light_position - in.world_position);
    let h = normalize(v + l);
    
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);
    
    // Calculate shadow factor
    let shadow_factor = calculate_shadow(in.light_space_position);
    
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
    
    // Lighting with shadow and distance attenuation
    let light_distance = length(global.light_position - in.world_position);
    let attenuation = 1.0 / (1.0 + 0.02 * light_distance);
    let radiance = global.light_color * global.light_intensity * attenuation * 10.0;
    
    let diffuse = albedo / 3.14159265;
    let color = (kd * diffuse + specular) * radiance * n_dot_l * shadow_factor; // Apply shadow
    
    // Fresnel-based fake shadows for depth perception (reduced when in real shadow)
    let fresnel_depth = 1.0 - n_dot_v;
    let shadow_strength = 0.9 * shadow_factor; // Reduce fake shadows in real shadows
    let fake_shadow = 1.0 - (fresnel_depth * shadow_strength);
    
    // Ambient lighting (slightly reduced in shadows)
    let ambient_factor = mix(0.8, 1.0, shadow_factor); // Darker ambient in shadows
    let ambient = vec3<f32>(0.15) * albedo * fake_shadow * ambient_factor;
    let final_color = ambient + color;
    
    // Add emissive
    let result = final_color + material.emissive;
    
    // Tone mapping
    let exposure = 0.8;
    let mapped = 1.0 - exp(-result * exposure);
    
    // Gamma correction
    let gamma_corrected = pow(mapped, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(gamma_corrected, material.base_color.a);
}