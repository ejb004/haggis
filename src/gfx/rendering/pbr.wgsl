struct GlobalUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    light_position: vec3<f32>,
    _padding1: f32,
    light_color: vec3<f32>,
    light_intensity: f32,
    light_view_proj: mat4x4<f32>,
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
@group(3) @binding(0) var shadow_map: texture_2d<f32>;
@group(3) @binding(1) var shadow_sampler: sampler;

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

    let world_position = transform.model * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = global.view_proj * world_position;
    out.light_space_position = global.light_view_proj * world_position;

    let normal_matrix = mat3x3<f32>(
        normalize(transform.model[0].xyz),
        normalize(transform.model[1].xyz),
        normalize(transform.model[2].xyz)
    );
    out.world_normal = normalize(normal_matrix * model.normal);

    return out;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let ggx2 = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let ggx1 = n_dot_l / (n_dot_l * (1.0 - k) + k);
    return ggx1 * ggx2;
}

fn calculate_shadow(in: VertexOutput, light_dir: vec3<f32>) -> f32 {
    let ndc = in.light_space_position.xyz / in.light_space_position.w;
    let shadow_coord = vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);

    if (shadow_coord.x < 0.0 || shadow_coord.x > 1.0 || 
        shadow_coord.y < 0.0 || shadow_coord.y > 1.0) {
        return 1.0;
    }

    let current_depth = ndc.z * 0.5 + 0.5;
    let bias = 0.001;

    // 2x2 tap to smooth out remaining blockiness from blur
    let texel_size = 1.0 / 2048.0;
    let offsets = array<vec2<f32>, 4>(
        vec2<f32>(-0.25, -0.25) * texel_size,
        vec2<f32>(0.25, -0.25) * texel_size,
        vec2<f32>(-0.25, 0.25) * texel_size,
        vec2<f32>(0.25, 0.25) * texel_size
    );
    
    var shadow_sum = 0.0;
    for (var i = 0; i < 4; i++) {
        let stored_depth = textureSample(shadow_map, shadow_sampler, shadow_coord + offsets[i]).r;
        let shadow_diff = current_depth - stored_depth - bias;
        
        if (shadow_diff <= 0.0) {
            shadow_sum += 1.0; // No shadow
        } else {
            // Wider smoothstep range for softer transitions
            let shadow_factor = smoothstep(0.0, 0.02, shadow_diff);
            shadow_sum += mix(1.0, 0.15, shadow_factor);
        }
    }
    
    return shadow_sum / 10.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(global.view_position.xyz - in.world_position);
    let light_dir = normalize(global.light_position - in.world_position);
    let halfway_dir = normalize(view_dir + light_dir);

    let albedo = material.base_color.rgb;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04);
    
    // Proper metallic workflow - metallic materials have high F0
    let dielectric_f0 = vec3<f32>(0.04);
    let f0 = mix(dielectric_f0, albedo, metallic);

    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let n_dot_h = max(dot(normal, halfway_dir), 0.0);
    let v_dot_h = max(dot(view_dir, halfway_dir), 0.0);

    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f = fresnel_schlick(v_dot_h, f0); // Removed artificial scaling

    let numerator = ndf * g * f;
    let denominator = 4.0 * n_dot_v * n_dot_l + 0.0001;
    let specular = numerator / denominator;

    // Energy conservation - metallic surfaces have no diffuse
    let ks = f;
    let kd = (vec3<f32>(1.0) - ks) * (1.0 - metallic);
    let diffuse = kd * albedo / 3.14159265;

    let distance = length(global.light_position - in.world_position);
    let attenuation = 1.0 / (distance * distance);
    let radiance = global.light_color * global.light_intensity * attenuation * 5.0;

    let shadow_factor = calculate_shadow(in, light_dir);
    let ambient = vec3<f32>(0.15) * albedo * (1.0 - metallic * 0.3); // Brighter ambient
    let lo = (diffuse + specular) * radiance * n_dot_l * shadow_factor;

    // Subtle rim lighting only for non-metals
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);
    let rim_light = rim * 0.3 * global.light_color * (1.0 - metallic); // Brighter rim

    let color = ambient + lo + material.emissive + rim_light;

    // Tone mapping and gamma correction
    let mapped = color / (color + vec3<f32>(1.0));
    let gamma_corrected = pow(mapped, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(gamma_corrected, material.base_color.a);
}