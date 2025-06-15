struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

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
    
    let world_position: vec4<f32> = vec4<f32>(model.position, 1.0);
    
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = normalize(model.normal); // Ensure normalized normals
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Enhanced base color with normal influence
    let base_object_color = vec3<f32>(0.9, 0.8, 0.8);
    
    // Add subtle normal-based color variation for better definition
    let normal = normalize(in.world_normal);
    let normal_color = (normal + 1.0) * 0.5; // Convert to [0,1] range
    let normal_influence = 0.7; // Subtle but noticeable
    let object_color = mix(base_object_color, normal_color, normal_influence);
    
    let light_dir = normalize(LIGHT.position - in.world_position);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    
    // Enhanced ambient with fresnel-based ambient occlusion
    let ambient_strength = 0.15;
    let ambient_base = LIGHT.color * ambient_strength;
    
    // Fresnel-based ambient occlusion (edges get more ambient light)
    let fresnel_ao = abs(dot(view_dir, normal));
    let ao_strength = 0.4; // How much AO effect to apply
    let ambient_occlusion = mix(1.0 - ao_strength, 1.0, fresnel_ao);
    let ambient_color = ambient_base * ambient_occlusion;
    
    // Enhanced diffuse with better normal definition
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse_strength = n_dot_l;
    let diffuse_color = LIGHT.color * diffuse_strength;
    
    // Enhanced specular
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let specular_strength = pow(n_dot_h, 64.0); // Increased shininess for sharper highlights
    let specular_color = specular_strength * LIGHT.color * 0.8; // Slightly reduced specular intensity
    
    // Additional rim lighting for better edge definition
    let rim_power = 2.0;
    let rim_intensity = 0.3;
    let rim_factor = 1.0 - abs(dot(view_dir, normal));
    let rim_light = pow(rim_factor, rim_power) * rim_intensity * LIGHT.color;
    
    // Combine all lighting components
    let final_lighting = ambient_color + diffuse_color + specular_color + rim_light;
    let result = final_lighting * object_color;
    
    // Enhanced fresnel effect for additional surface definition
    let fresnel = abs(dot(view_dir, normal));
    let fresnel_mix = mix(0.6, 1.0, fresnel); // Less extreme than original (0.5 to 1.0)
    
    return vec4<f32>(result * fresnel_mix, 1.0);
}