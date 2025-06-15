struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct Transform {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> transform: Transform;  // ADD THIS

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
    
    // CHANGE THIS: Apply transform to position
    let world_position: vec4<f32> = transform.model * vec4<f32>(model.position, 1.0);
    
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    
    // CHANGE THIS: Apply transform to normal (use inverse transpose for proper normal transformation)
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
    // Your existing fragment shader code stays the same
    let base_object_color = vec3<f32>(0.9, 0.8, 0.8);
    
    let normal = normalize(in.world_normal);
    let normal_color = (normal + 1.0) * 0.5;
    let normal_influence = 0.7;
    let object_color = mix(base_object_color, normal_color, normal_influence);
    
    let light_dir = normalize(LIGHT.position - in.world_position);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(view_dir + light_dir);
    
    let ambient_strength = 0.15;
    let ambient_base = LIGHT.color * ambient_strength;
    
    let fresnel_ao = abs(dot(view_dir, normal));
    let ao_strength = 0.4;
    let ambient_occlusion = mix(1.0 - ao_strength, 1.0, fresnel_ao);
    let ambient_color = ambient_base * ambient_occlusion;
    
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse_strength = n_dot_l;
    let diffuse_color = LIGHT.color * diffuse_strength;
    
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let specular_strength = pow(n_dot_h, 64.0);
    let specular_color = specular_strength * LIGHT.color * 0.8;
    
    let rim_power = 2.0;
    let rim_intensity = 0.3;
    let rim_factor = 1.0 - abs(dot(view_dir, normal));
    let rim_light = pow(rim_factor, rim_power) * rim_intensity * LIGHT.color;
    
    let final_lighting = ambient_color + diffuse_color + specular_color + rim_light;
    let result = final_lighting * object_color;
    
    let fresnel = abs(dot(view_dir, normal));
    let fresnel_mix = mix(0.6, 1.0, fresnel);
    
    return vec4<f32>(result * fresnel_mix, 1.0);
}