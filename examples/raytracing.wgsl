// Basic GPU Ray Tracing Compute Shader
// Implements ray-sphere intersection with Lambert shading

struct Sphere {
    center: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
    material: f32, // 0.0 = diffuse, 1.0 = reflective
}

struct RayTracingParams {
    camera_pos: vec3<f32>,
    camera_dir: vec3<f32>,
    camera_up: vec3<f32>,
    camera_right: vec3<f32>,
    fov: f32,
    screen_width: f32,
    screen_height: f32,
    sphere_count: f32,
}

struct Pixel {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct HitInfo {
    hit: bool,
    distance: f32,
    point: vec3<f32>,
    normal: vec3<f32>,
    color: vec3<f32>,
    material: f32,
}

@group(0) @binding(0) var<storage, read> spheres: array<Sphere, 8>;
@group(0) @binding(1) var<storage, read> params: RayTracingParams;
@group(0) @binding(2) var<storage, read_write> output: array<Pixel>;

// Ray-sphere intersection
fn intersect_sphere(ray: Ray, sphere: Sphere) -> HitInfo {
    var hit_info: HitInfo;
    hit_info.hit = false;
    hit_info.distance = 1000000.0;
    
    let oc = ray.origin - sphere.center;
    let a = dot(ray.direction, ray.direction);
    let b = 2.0 * dot(oc, ray.direction);
    let c = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant = b * b - 4.0 * a * c;
    
    if (discriminant >= 0.0) {
        let sqrt_discriminant = sqrt(discriminant);
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);
        
        var t = t1;
        if (t1 < 0.001) {
            t = t2;
        }
        
        if (t > 0.001) {
            hit_info.hit = true;
            hit_info.distance = t;
            hit_info.point = ray.origin + t * ray.direction;
            hit_info.normal = normalize(hit_info.point - sphere.center);
            hit_info.color = sphere.color;
            hit_info.material = sphere.material;
        }
    }
    
    return hit_info;
}

// Find closest intersection with scene
fn trace_ray(ray: Ray) -> HitInfo {
    var closest_hit: HitInfo;
    closest_hit.hit = false;
    closest_hit.distance = 1000000.0;
    
    for (var i: u32 = 0u; i < u32(params.sphere_count); i = i + 1u) {
        let hit = intersect_sphere(ray, spheres[i]);
        if (hit.hit && hit.distance < closest_hit.distance) {
            closest_hit = hit;
        }
    }
    
    return closest_hit;
}

// Simple Lambert shading
fn calculate_lighting(hit: HitInfo) -> vec3<f32> {
    let light_pos = vec3<f32>(2.0, 4.0, 1.0);
    let light_color = vec3<f32>(1.0, 1.0, 1.0);
    let ambient = vec3<f32>(0.1, 0.1, 0.1);
    
    // Check if point is in shadow
    let light_dir = normalize(light_pos - hit.point);
    let shadow_ray = Ray(hit.point + hit.normal * 0.001, light_dir);
    let shadow_hit = trace_ray(shadow_ray);
    
    var diffuse = vec3<f32>(0.0, 0.0, 0.0);
    if (!shadow_hit.hit) {
        let n_dot_l = max(dot(hit.normal, light_dir), 0.0);
        diffuse = hit.color * light_color * n_dot_l;
    }
    
    // Simple reflection
    var reflection = vec3<f32>(0.0, 0.0, 0.0);
    if (hit.material > 0.0) {
        let view_dir = normalize(-hit.point); // Camera is at origin
        let reflect_dir = reflect(-light_dir, hit.normal);
        let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
        reflection = light_color * spec * hit.material;
    }
    
    return ambient + diffuse + reflection;
}

// Generate ray for given pixel
fn get_camera_ray(pixel_x: f32, pixel_y: f32) -> Ray {
    let aspect_ratio = params.screen_width / params.screen_height;
    let fov_rad = params.fov * 3.14159 / 180.0;
    let half_height = tan(fov_rad / 2.0);
    let half_width = aspect_ratio * half_height;
    
    // Convert pixel coordinates to normalized device coordinates
    let u = (pixel_x / params.screen_width) * 2.0 - 1.0;
    let v = 1.0 - (pixel_y / params.screen_height) * 2.0;
    
    // Calculate ray direction
    let direction = normalize(
        params.camera_dir +
        u * half_width * params.camera_right +
        v * half_height * params.camera_up
    );
    
    return Ray(params.camera_pos, direction);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_x = global_id.x;
    let pixel_y = global_id.y;
    
    if (pixel_x >= u32(params.screen_width) || pixel_y >= u32(params.screen_height)) {
        return;
    }
    
    let pixel_index = pixel_y * u32(params.screen_width) + pixel_x;
    
    // Generate camera ray
    let ray = get_camera_ray(f32(pixel_x), f32(pixel_y));
    
    // Trace ray
    let hit = trace_ray(ray);
    
    var color = vec3<f32>(0.2, 0.3, 0.8); // Sky color
    
    if (hit.hit) {
        color = calculate_lighting(hit);
    }
    
    // Clamp color values
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    
    // Write to output buffer
    output[pixel_index] = Pixel(color.r, color.g, color.b, 1.0);
}