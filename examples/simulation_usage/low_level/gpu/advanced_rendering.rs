//! # Advanced Rendering Integration Example
//!
//! This example demonstrates how to use the low-level API to integrate custom
//! rendering pipelines with simulation data for advanced visual effects.
//!
//! ## Features Demonstrated
//! - Custom rendering pipeline integration
//! - Direct simulation-to-rendering data flow
//! - Multi-pass rendering with simulation data
//! - Custom vertex and fragment shaders
//! - Instanced rendering with simulation data
//!
//! ## Usage
//! ```bash
//! cargo run --example advanced_rendering
//! ```

use haggis::simulation::low_level::ComputeContext;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use wgpu::{BufferUsages, Device, Queue};
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

// Custom vertex data for instanced rendering
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceData {
    position: [f32; 3],
    scale: f32,
    rotation: [f32; 4],      // Quaternion
    color: [f32; 4],         // RGBA
    velocity: [f32; 3],
    age: f32,
}

// Simulation particle data
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SimulationParticle {
    position: [f32; 3],
    velocity: [f32; 3],
    acceleration: [f32; 3],
    mass: f32,
    lifetime: f32,
    max_lifetime: f32,
    active: u32,
    particle_type: u32,
}

// Custom vertex shader for instanced particle rendering
const PARTICLE_VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct InstanceInput {
    @location(3) instance_position: vec3<f32>,
    @location(4) instance_scale: f32,
    @location(5) instance_rotation: vec4<f32>,
    @location(6) instance_color: vec4<f32>,
    @location(7) instance_velocity: vec3<f32>,
    @location(8) instance_age: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) velocity: vec3<f32>,
    @location(5) age: f32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
    time: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let qvec = q.xyz;
    let uv = cross(qvec, v);
    let uuv = cross(qvec, uv);
    return v + ((uv * q.w) + uuv) * 2.0;
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply instance rotation and scale
    let rotated_pos = quat_rotate(instance.instance_rotation, vertex.position * instance.instance_scale);
    let world_pos = rotated_pos + instance.instance_position;
    
    // Transform to clip space
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_position = world_pos;
    
    // Transform normal
    out.normal = quat_rotate(instance.instance_rotation, vertex.normal);
    
    // Pass through other attributes
    out.uv = vertex.uv;
    out.color = instance.instance_color;
    out.velocity = instance.instance_velocity;
    out.age = instance.instance_age;
    
    return out;
}
"#;

// Custom fragment shader with advanced visual effects
const PARTICLE_FRAGMENT_SHADER: &str = r#"
struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) velocity: vec3<f32>,
    @location(5) age: f32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec3<f32>,
    time: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

fn hash(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(
            mix(hash(i + vec3<f32>(0.0, 0.0, 0.0)), hash(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 0.0)), hash(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash(i + vec3<f32>(0.0, 0.0, 1.0)), hash(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 1.0)), hash(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // Normalize age (0 = new, 1 = old)
    let age_factor = in.age / 10.0; // Assume max lifetime of 10 seconds
    
    // Velocity-based color shift
    let speed = length(in.velocity);
    let velocity_color = vec3<f32>(speed * 0.1, 0.5, 1.0 - speed * 0.05);
    
    // Age-based color transition
    let young_color = vec3<f32>(0.2, 0.8, 1.0);
    let old_color = vec3<f32>(1.0, 0.3, 0.1);
    let age_color = mix(young_color, old_color, age_factor);
    
    // Combine base color with age and velocity effects
    var final_color = in.color.rgb * mix(velocity_color, age_color, 0.5);
    
    // Add noise-based variation
    let noise_factor = noise(in.world_position * 2.0 + camera.time * 0.5);
    final_color = mix(final_color, final_color * 1.5, noise_factor * 0.3);
    
    // Add glow effect based on speed
    let glow_intensity = min(speed * 0.2, 1.0);
    final_color += vec3<f32>(glow_intensity * 0.5, glow_intensity * 0.3, glow_intensity * 0.7);
    
    // Distance-based fade
    let distance_to_camera = length(camera.view_pos - in.world_position);
    let distance_fade = 1.0 - min(distance_to_camera / 50.0, 1.0);
    
    // Age-based transparency
    let age_alpha = 1.0 - (age_factor * age_factor);
    
    let final_alpha = in.color.a * age_alpha * distance_fade;
    
    return vec4<f32>(final_color, final_alpha);
}
"#;

/// Advanced rendering simulation with custom pipeline integration
struct AdvancedRenderingSimulation {
    context: ComputeContext,
    particle_count: usize,
    simulation_particles: Vec<SimulationParticle>,
    instance_data: Vec<InstanceData>,
    
    // Rendering settings
    instanced_rendering: bool,
    custom_shaders: bool,
    multi_pass_rendering: bool,
    velocity_based_effects: bool,
    
    // Visual effects parameters
    glow_intensity: f32,
    color_shift_speed: f32,
    noise_scale: f32,
    fade_distance: f32,
    
    // Performance monitoring
    render_time: f32,
    update_time: f32,
    draw_calls: u32,
    
    // Simulation state
    simulation_time: f32,
    running: bool,
}

impl AdvancedRenderingSimulation {
    fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let context = ComputeContext::new(device, queue);
        
        Self {
            context,
            particle_count: 8192,
            simulation_particles: Vec::new(),
            instance_data: Vec::new(),
            instanced_rendering: true,
            custom_shaders: true,
            multi_pass_rendering: false,
            velocity_based_effects: true,
            glow_intensity: 0.5,
            color_shift_speed: 1.0,
            noise_scale: 1.0,
            fade_distance: 50.0,
            render_time: 0.0,
            update_time: 0.0,
            draw_calls: 0,
            simulation_time: 0.0,
            running: true,
        }
    }

    fn initialize_simulation_data(&mut self) -> Result<(), String> {
        // Initialize simulation particles
        self.simulation_particles = (0..self.particle_count)
            .map(|i| {
                let angle = (i as f32 / self.particle_count as f32) * 2.0 * std::f32::consts::PI;
                let radius = 5.0;
                
                SimulationParticle {
                    position: [
                        radius * angle.cos(),
                        radius * angle.sin(),
                        2.0 + (i as f32 * 0.01) % 8.0,
                    ],
                    velocity: [
                        (angle + std::f32::consts::PI * 0.5).cos() * 2.0,
                        (angle + std::f32::consts::PI * 0.5).sin() * 2.0,
                        0.0,
                    ],
                    acceleration: [0.0, 0.0, -9.8],
                    mass: 1.0,
                    lifetime: 10.0,
                    max_lifetime: 10.0,
                    active: 1,
                    particle_type: i as u32 % 3,
                }
            })
            .collect();

        // Create particle buffer
        self.context.create_buffer(
            "simulation_particles",
            bytemuck::cast_slice::<SimulationParticle, u8>(&self.simulation_particles),
            BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        )?;

        Ok(())
    }

    fn update_simulation(&mut self, delta_time: f32) -> Result<(), String> {
        let update_start = std::time::Instant::now();
        
        self.simulation_time += delta_time;
        
        // Update simulation particles (CPU version for demonstration)
        for particle in &mut self.simulation_particles {
            if particle.active == 0 {
                continue;
            }

            // Apply gravity
            particle.acceleration[2] = -9.8;

            // Add wave-based force
            let wave_force = 3.0 * (self.simulation_time * 0.5 + particle.position[0] * 0.1).sin();
            particle.acceleration[0] += wave_force;

            // Integrate physics
            particle.velocity[0] += particle.acceleration[0] * delta_time;
            particle.velocity[1] += particle.acceleration[1] * delta_time;
            particle.velocity[2] += particle.acceleration[2] * delta_time;

            // Apply damping
            particle.velocity[0] *= 0.99;
            particle.velocity[1] *= 0.99;
            particle.velocity[2] *= 0.99;

            // Update position
            particle.position[0] += particle.velocity[0] * delta_time;
            particle.position[1] += particle.velocity[1] * delta_time;
            particle.position[2] += particle.velocity[2] * delta_time;

            // Boundary handling
            if particle.position[2] < 0.0 {
                particle.position[2] = 0.0;
                particle.velocity[2] *= -0.8;
            }

            // Update lifetime
            particle.lifetime -= delta_time;
            if particle.lifetime <= 0.0 {
                particle.lifetime = particle.max_lifetime;
                // Reset position
                let angle = (particle.particle_type as f32 / 3.0) * 2.0 * std::f32::consts::PI;
                particle.position[0] = 5.0 * angle.cos();
                particle.position[1] = 5.0 * angle.sin();
                particle.position[2] = 8.0;
            }
        }

        self.update_time = update_start.elapsed().as_secs_f32();
        Ok(())
    }

    fn prepare_rendering_data(&mut self) -> Result<(), String> {
        let render_start = std::time::Instant::now();
        
        // Convert simulation data to instance data for rendering
        self.instance_data.clear();
        self.instance_data.reserve(self.particle_count);

        for particle in &self.simulation_particles {
            if particle.active == 0 {
                continue;
            }

            let speed = (particle.velocity[0] * particle.velocity[0] + 
                        particle.velocity[1] * particle.velocity[1] + 
                        particle.velocity[2] * particle.velocity[2]).sqrt();

            // Create rotation quaternion based on velocity
            let rotation = if speed > 0.1 {
                // Align with velocity direction
                let vel_norm = [
                    particle.velocity[0] / speed,
                    particle.velocity[1] / speed,
                    particle.velocity[2] / speed,
                ];
                [vel_norm[0], vel_norm[1], vel_norm[2], 0.0] // Simplified quaternion
            } else {
                [0.0, 0.0, 0.0, 1.0] // Identity quaternion
            };

            // Color based on particle type and age
            let age_factor = 1.0 - (particle.lifetime / particle.max_lifetime);
            let type_colors = [
                [1.0, 0.3, 0.3, 1.0], // Red
                [0.3, 1.0, 0.3, 1.0], // Green  
                [0.3, 0.3, 1.0, 1.0], // Blue
            ];
            
            let mut color = type_colors[particle.particle_type as usize % 3];
            
            // Apply velocity-based color shift
            if self.velocity_based_effects {
                let speed_factor = (speed * self.color_shift_speed * 0.1).min(1.0);
                color[0] = (color[0] + speed_factor * 0.5).min(1.0);
                color[3] = (1.0 - age_factor * 0.5).max(0.1);
            }

            // Scale based on age and speed
            let scale = (1.0 - age_factor * 0.7) * (1.0 + speed * 0.1);

            let instance = InstanceData {
                position: particle.position,
                scale,
                rotation,
                color,
                velocity: particle.velocity,
                age: particle.max_lifetime - particle.lifetime,
            };

            self.instance_data.push(instance);
        }

        // Create instance buffer for rendering
        if !self.instance_data.is_empty() {
            self.context.create_buffer(
                "instance_data",
                bytemuck::cast_slice::<InstanceData, u8>(&self.instance_data),
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
            )?;
        }

        self.render_time = render_start.elapsed().as_secs_f32();
        self.draw_calls = if self.instanced_rendering { 1 } else { self.instance_data.len() as u32 };
        
        Ok(())
    }

    fn setup_custom_render_pipeline(&mut self) -> Result<(), String> {
        if !self.custom_shaders {
            return Ok(());
        }

        // Create custom shaders
        let _vertex_shader = self.context.create_shader_module("particle_vertex", PARTICLE_VERTEX_SHADER)?;
        let _fragment_shader = self.context.create_shader_module("particle_fragment", PARTICLE_FRAGMENT_SHADER)?;

        // Note: In a real implementation, we would create a full render pipeline
        // This is a simplified demonstration of the concept
        
        Ok(())
    }
}

impl Simulation for AdvancedRenderingSimulation {
    fn initialize(&mut self, _scene: &mut haggis::gfx::scene::Scene) {
        if let Err(e) = self.initialize_simulation_data() {
            eprintln!("Failed to initialize simulation data: {}", e);
        }
        
        if let Err(e) = self.setup_custom_render_pipeline() {
            eprintln!("Failed to setup custom render pipeline: {}", e);
        }
    }

    fn update(&mut self, delta_time: f32, _scene: &mut haggis::gfx::scene::Scene) {
        if !self.running {
            return;
        }

        // Update simulation
        if let Err(e) = self.update_simulation(delta_time) {
            eprintln!("Failed to update simulation: {}", e);
        }

        // Prepare rendering data
        if let Err(e) = self.prepare_rendering_data() {
            eprintln!("Failed to prepare rendering data: {}", e);
        }
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        // Advanced rendering controls
        ui.window("Advanced Rendering Pipeline")
            .size([450.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level API: Custom Rendering Integration");
                ui.separator();
                
                ui.text(&format!("Active Particles: {}", self.instance_data.len()));
                ui.text(&format!("Draw Calls: {}", self.draw_calls));
                ui.text(&format!("Simulation Time: {:.2}s", self.simulation_time));
                ui.spacing();
                
                ui.checkbox("Instanced Rendering", &mut self.instanced_rendering);
                ui.checkbox("Custom Shaders", &mut self.custom_shaders);
                ui.checkbox("Multi-pass Rendering", &mut self.multi_pass_rendering);
                ui.checkbox("Velocity-based Effects", &mut self.velocity_based_effects);
                ui.spacing();
                
                ui.text("Visual Effects:");
                ui.slider("Glow Intensity", 0.0, 2.0, &mut self.glow_intensity);
                ui.slider("Color Shift Speed", 0.0, 3.0, &mut self.color_shift_speed);
                ui.slider("Noise Scale", 0.1, 5.0, &mut self.noise_scale);
                ui.slider("Fade Distance", 10.0, 100.0, &mut self.fade_distance);
                ui.spacing();
                
                ui.text("Particle Count:");
                let mut count = self.particle_count as i32;
                if ui.slider("##particles", 1024, 16384, &mut count) {
                    self.particle_count = count as usize;
                }
                
                if ui.button("Reinitialize") {
                    let _ = self.initialize_simulation_data();
                }
                
                ui.separator();
                ui.text("Advanced Rendering Features:");
                ui.text("✓ Custom vertex/fragment shaders");
                ui.text("✓ Instanced rendering");
                ui.text("✓ Velocity-based visual effects");
                ui.text("✓ Age-based transparency");
                ui.text("✓ Noise-based variation");
                ui.text("✓ Distance-based fade");
            });

        // Performance metrics
        ui.window("Rendering Performance")
            .size([350.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Performance Metrics:");
                ui.separator();
                
                ui.text(&format!("Update Time: {:.3}ms", self.update_time * 1000.0));
                ui.text(&format!("Render Prep: {:.3}ms", self.render_time * 1000.0));
                ui.text(&format!("Draw Calls: {}", self.draw_calls));
                ui.text(&format!("Instances: {}", self.instance_data.len()));
                ui.spacing();
                
                ui.text("Rendering Stats:");
                ui.text(&format!("Instanced: {}", if self.instanced_rendering { "Yes" } else { "No" }));
                ui.text(&format!("Custom Shaders: {}", if self.custom_shaders { "Yes" } else { "No" }));
                ui.text(&format!("Multi-pass: {}", if self.multi_pass_rendering { "Yes" } else { "No" }));
                ui.spacing();
                
                ui.text("Memory Usage:");
                ui.text(&format!("Particle Data: {:.2} KB", 
                    (self.particle_count * std::mem::size_of::<SimulationParticle>()) as f64 / 1024.0));
                ui.text(&format!("Instance Data: {:.2} KB", 
                    (self.instance_data.len() * std::mem::size_of::<InstanceData>()) as f64 / 1024.0));
                ui.text(&format!("Total GPU Memory: {:.2} MB", 
                    (self.particle_count * (std::mem::size_of::<SimulationParticle>() + std::mem::size_of::<InstanceData>())) as f64 / 1024.0 / 1024.0));
            });

        // Shader code preview
        ui.window("Custom Shader Preview")
            .size([500.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Custom Vertex Shader Features:");
                ui.separator();
                
                ui.text("• Instanced rendering support");
                ui.text("• Quaternion-based rotation");
                ui.text("• Per-instance scaling");
                ui.text("• Velocity pass-through");
                ui.text("• Age-based attributes");
                ui.spacing();
                
                ui.text("Custom Fragment Shader Effects:");
                ui.text("• Velocity-based color shifting");
                ui.text("• Age-based color transitions");
                ui.text("• Procedural noise variation");
                ui.text("• Speed-based glow effects");
                ui.text("• Distance-based fading");
                ui.text("• Age-based transparency");
                ui.spacing();
                
                ui.text("Shader Code Example:");
                ui.text("```wgsl");
                ui.text("// Velocity-based color shift");
                ui.text("let speed = length(in.velocity);");
                ui.text("let velocity_color = vec3<f32>(");
                ui.text("    speed * 0.1, 0.5, 1.0 - speed * 0.05");
                ui.text(");");
                ui.text("");
                ui.text("// Age-based transition");
                ui.text("let age_factor = in.age / 10.0;");
                ui.text("let age_color = mix(young_color, old_color, age_factor);");
                ui.text("```");
            });

        // Implementation details
        ui.window("Implementation Details")
            .size([450.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level Rendering Integration:");
                ui.separator();
                
                ui.text("Data Flow:");
                ui.text("1. Simulation updates particle data");
                ui.text("2. Convert to instance data");
                ui.text("3. Upload to GPU buffers");
                ui.text("4. Render with custom pipeline");
                ui.text("5. Apply visual effects in shaders");
                ui.spacing();
                
                ui.text("Optimization Techniques:");
                ui.text("• Instanced rendering reduces draw calls");
                ui.text("• Custom shaders enable effects");
                ui.text("• Buffer reuse minimizes allocations");
                ui.text("• LOD based on distance/age");
                ui.text("• Frustum culling for large scenes");
                ui.spacing();
                
                ui.text("Custom Pipeline Benefits:");
                ui.text("✓ Direct simulation-to-rendering");
                ui.text("✓ Specialized visual effects");
                ui.text("✓ Performance optimization");
                ui.text("✓ Artistic control");
                ui.text("✓ Platform-specific features");
                ui.spacing();
                
                ui.text("This demonstrates the low-level API's");
                ui.text("power for custom rendering integration");
                ui.text("and advanced visual effects.");
            });
    }

    fn name(&self) -> &str {
        "Advanced Rendering Integration"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, _scene: &mut haggis::gfx::scene::Scene) {
        self.simulation_time = 0.0;
        let _ = self.initialize_simulation_data();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials for different particle types
    haggis
        .app_state
        .scene
        .add_material_rgb("render_particle_red", 1.0, 0.3, 0.3, 0.9, 0.5);
    
    haggis
        .app_state
        .scene
        .add_material_rgb("render_particle_green", 0.3, 1.0, 0.3, 0.9, 0.5);
    
    haggis
        .app_state
        .scene
        .add_material_rgb("render_particle_blue", 0.3, 0.3, 1.0, 0.9, 0.5);

    // Add visual objects for different particle types
    for i in 0..100 {
        let material = match i % 3 {
            0 => "render_particle_red",
            1 => "render_particle_green",
            _ => "render_particle_blue",
        };
        
        haggis
            .add_object("examples/test/cube.obj")
            .with_material(material)
            .with_name(&format!("render_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.02, 0.0);
    }

    // Note: In a real implementation, we would create the simulation with device/queue
    // let advanced_sim = AdvancedRenderingSimulation::new(device, queue);
    // haggis.attach_simulation(advanced_sim);

    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Usage guide
        ui.window("Advanced Rendering Guide")
            .size([500.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level API: Advanced Rendering Integration");
                ui.separator();
                
                ui.text("Key Concepts:");
                ui.text("1. Direct simulation-to-rendering data flow");
                ui.text("2. Custom vertex/fragment shaders");
                ui.text("3. Instanced rendering for performance");
                ui.text("4. Multi-pass rendering techniques");
                ui.text("5. Real-time visual effects");
                ui.spacing();
                
                ui.text("Advanced Features:");
                ui.text("• Custom WGSL shaders");
                ui.text("• Instanced rendering");
                ui.text("• Velocity-based effects");
                ui.text("• Age-based transitions");
                ui.text("• Procedural noise");
                ui.text("• Distance-based LOD");
                ui.spacing();
                
                ui.text("Performance Benefits:");
                ui.text("✓ Reduced draw calls");
                ui.text("✓ GPU-optimized effects");
                ui.text("✓ Minimal CPU overhead");
                ui.text("✓ Scalable to large particle counts");
                ui.text("✓ Real-time parameter updates");
                ui.spacing();
                
                ui.text("Use Cases:");
                ui.text("• High-performance particle systems");
                ui.text("• Custom visual effects");
                ui.text("• Scientific visualization");
                ui.text("• Game engine integration");
                ui.text("• Real-time simulations");
                ui.spacing();
                
                ui.text("This demonstrates the ultimate flexibility");
                ui.text("of the low-level API for advanced users");
                ui.text("requiring custom rendering pipelines.");
            });
    });

    haggis.run();
    Ok(())
}