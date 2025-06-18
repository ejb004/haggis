// Adapted shader with material support - based on your working shader

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Use material base color instead of hardcoded color
    let base_object_color = material.base_color.rgb;
    
    let normal = normalize(in.world_normal);
    let normal_color = (normal + 1.0) * 0.5;
    
    // Use material roughness to control normal influence
    let normal_influence = material.roughness * 0.5; // Less normal color on smooth surfaces
    let object_color = mix(base_object_color, normal_color, normal_influence);
    
    let light_dir = normalize(LIGHT.position - in.world_position);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    
    // Ambient lighting with material's occlusion strength
    let ambient_strength = 0.15;
    let ambient_base = LIGHT.color * ambient_strength;
    
    let fresnel_ao = abs(dot(view_dir, normal));
    let ao_strength = material.occlusion_strength * 0.4; // Use material's AO setting
    let ambient_occlusion = mix(1.0 - ao_strength, 1.0, fresnel_ao);
    let ambient_color = ambient_base * ambient_occlusion;
    
    // Diffuse lighting
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse_strength = n_dot_l;
    let diffuse_color = LIGHT.color * diffuse_strength;
    
    // Specular lighting based on material roughness
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let shininess = (1.0 - material.roughness) * 128.0 + 1.0; // Rough = low shininess
    let specular_strength = pow(n_dot_h, shininess);
    
    // Metallic materials have different specular behavior
    let specular_intensity = mix(0.8, 1.2, material.metallic); // Metals more reflective
    let specular_color = specular_strength * LIGHT.color * specular_intensity;
    
    // Rim lighting (less prominent on metals)
    let rim_power = 2.0;
    let rim_intensity = 0.3 * (1.0 - material.metallic * 0.5); // Reduce rim on metals
    let rim_factor = 1.0 - abs(dot(view_dir, normal));
    let rim_light = pow(rim_factor, rim_power) * rim_intensity * LIGHT.color;
    
    // Combine lighting
    let final_lighting = ambient_color + diffuse_color + specular_color + rim_light;
    var result = final_lighting * object_color;
    
    // Add emissive color
    result += material.emissive;
    
    // Fresnel effect (more prominent on metals)
    let fresnel = abs(dot(view_dir, normal));
    let fresnel_strength = mix(0.6, 0.3, material.metallic); // Metals have different fresnel
    let fresnel_mix = mix(fresnel_strength, 1.0, fresnel);
    
    return vec4<f32>(result * fresnel_mix, material.base_color.a);
}