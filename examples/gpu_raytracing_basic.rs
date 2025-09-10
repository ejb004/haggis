//! # Basic GPU Ray Tracing Example
//!
//! Demonstrates GPU-based ray tracing using compute shaders for real-time rendering.
//! Features ray-sphere intersection, basic lighting (Lambert shading), and multiple spheres.

use haggis::ComputeEngine;
use haggis::compute::{BufferConfig, PipelineConfig, BufferBinding, BufferAccessMode};
use bytemuck::{Pod, Zeroable};
use cgmath::{Vector3, InnerSpace};
use std::sync::Arc;
use std::any::Any;
use wgpu::BufferUsages;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;
const MAX_SPHERES: usize = 8;

/// Ray tracing sphere data
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Sphere {
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
    material: f32, // 0.0 = diffuse, 1.0 = reflective
}

/// Ray tracing scene parameters
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RayTracingParams {
    camera_pos: [f32; 3],
    _padding1: f32,
    camera_dir: [f32; 3],
    _padding2: f32,
    camera_up: [f32; 3],
    _padding3: f32,
    camera_right: [f32; 3],
    fov: f32,
    screen_width: f32,
    screen_height: f32,
    sphere_count: f32,
    _padding4: f32,
}

/// Output pixel data
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Pixel {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

/// GPU Ray Tracing Simulation
struct GpuRayTracingSimulation {
    compute_engine: Option<ComputeEngine>,
    spheres: Vec<Sphere>,
    params: RayTracingParams,
    output_pixels: Vec<Pixel>,
    camera_angle: f32,
    animation_time: f32,
    running: bool,
}

impl GpuRayTracingSimulation {
    fn new() -> Self {
        // Initialize scene with multiple spheres
        let spheres = vec![
            Sphere {
                center: [0.0, 0.0, -5.0],
                radius: 1.0,
                color: [1.0, 0.2, 0.2], // Red
                material: 0.1,
            },
            Sphere {
                center: [-2.0, 0.0, -6.0],
                radius: 0.8,
                color: [0.2, 1.0, 0.2], // Green
                material: 0.0,
            },
            Sphere {
                center: [2.0, 1.0, -4.0],
                radius: 0.6,
                color: [0.2, 0.2, 1.0], // Blue
                material: 0.3,
            },
            Sphere {
                center: [0.0, -100.5, -5.0],
                radius: 100.0,
                color: [0.8, 0.8, 0.8], // Ground sphere (large)
                material: 0.0,
            },
        ];

        let params = RayTracingParams {
            camera_pos: [0.0, 0.0, 0.0],
            _padding1: 0.0,
            camera_dir: [0.0, 0.0, -1.0],
            _padding2: 0.0,
            camera_up: [0.0, 1.0, 0.0],
            _padding3: 0.0,
            camera_right: [1.0, 0.0, 0.0],
            fov: 45.0,
            screen_width: SCREEN_WIDTH as f32,
            screen_height: SCREEN_HEIGHT as f32,
            sphere_count: spheres.len() as f32,
            _padding4: 0.0,
        };

        let output_pixels = vec![
            Pixel { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }; 
            (SCREEN_WIDTH * SCREEN_HEIGHT) as usize
        ];

        Self {
            compute_engine: None,
            spheres,
            params,
            output_pixels,
            camera_angle: 0.0,
            animation_time: 0.0,
            running: true,
        }
    }

    fn setup_compute_engine(&mut self, device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<(), String> {
        let mut engine = ComputeEngine::new(device, queue);

        // Create buffers for spheres, parameters, and output
        let sphere_buffer = BufferConfig {
            name: "spheres".to_string(),
            element_size: std::mem::size_of::<Sphere>() as u32,
            element_count: MAX_SPHERES as u32,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            initial_data: Some({
                let mut data = vec![Sphere {
                    center: [0.0, 0.0, 0.0],
                    radius: 0.0,
                    color: [0.0, 0.0, 0.0],
                    material: 0.0,
                }; MAX_SPHERES];
                
                for (i, sphere) in self.spheres.iter().enumerate() {
                    if i < MAX_SPHERES {
                        data[i] = *sphere;
                    }
                }
                bytemuck::cast_slice(&data).to_vec()
            }),
        };

        let params_buffer = BufferConfig {
            name: "params".to_string(),
            element_size: std::mem::size_of::<RayTracingParams>() as u32,
            element_count: 1,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            initial_data: Some(bytemuck::cast_slice(&[self.params]).to_vec()),
        };

        let output_buffer = BufferConfig {
            name: "output".to_string(),
            element_size: std::mem::size_of::<Pixel>() as u32,
            element_count: (SCREEN_WIDTH * SCREEN_HEIGHT) as u32,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            initial_data: None,
        };

        engine.create_buffer(sphere_buffer)?;
        engine.create_buffer(params_buffer)?;
        engine.create_buffer(output_buffer)?;

        // Create ray tracing compute pipeline
        let shader_source = include_str!("raytracing.wgsl");
        let pipeline_config = PipelineConfig {
            name: "raytracing".to_string(),
            shader_source: shader_source.to_string(),
            entry_point: "main".to_string(),
            workgroup_size: [16, 16, 1],
            buffer_bindings: vec![
                BufferBinding {
                    name: "spheres".to_string(),
                    access: BufferAccessMode::ReadOnly,
                },
                BufferBinding {
                    name: "params".to_string(),
                    access: BufferAccessMode::ReadOnly,
                },
                BufferBinding {
                    name: "output".to_string(),
                    access: BufferAccessMode::ReadWrite,
                },
            ],
        };

        engine.create_pipeline(pipeline_config)?;

        self.compute_engine = Some(engine);
        Ok(())
    }

    fn update_camera(&mut self, dt: f32) {
        self.animation_time += dt;
        self.camera_angle += dt * 0.5; // Rotate camera slowly

        // Orbit camera around the scene
        let radius = 3.0;
        self.params.camera_pos = [
            radius * self.camera_angle.cos(),
            1.0 + (self.animation_time * 0.5).sin() * 0.5, // Slight vertical movement
            radius * self.camera_angle.sin(),
        ];

        // Look at center
        let center = Vector3::new(0.0, 0.0, -5.0);
        let pos = Vector3::new(self.params.camera_pos[0], self.params.camera_pos[1], self.params.camera_pos[2]);
        let dir = (center - pos).normalize();
        let up = Vector3::new(0.0, 1.0, 0.0);
        let right = dir.cross(up).normalize();
        let actual_up = right.cross(dir).normalize();

        self.params.camera_dir = [dir.x, dir.y, dir.z];
        self.params.camera_up = [actual_up.x, actual_up.y, actual_up.z];
        self.params.camera_right = [right.x, right.y, right.z];
    }

    fn animate_spheres(&mut self, _dt: f32) {
        // Animate sphere positions
        for (i, sphere) in self.spheres.iter_mut().enumerate() {
            if i < 3 { // Only animate the first 3 spheres, not the ground
                let time_offset = i as f32 * 1.3;
                sphere.center[1] = (self.animation_time + time_offset).sin() * 0.5;
                sphere.center[0] += (self.animation_time * 0.3 + time_offset).cos() * 0.01;
            }
        }
    }

    fn run_raytracing(&mut self) -> Result<(), String> {
        if let Some(engine) = &self.compute_engine {
            // Update parameters buffer
            engine.update_buffer("params", &[self.params])?;
            
            // Update spheres buffer
            let mut sphere_data = vec![Sphere {
                center: [0.0, 0.0, 0.0],
                radius: 0.0,
                color: [0.0, 0.0, 0.0],
                material: 0.0,
            }; MAX_SPHERES];
            
            for (i, sphere) in self.spheres.iter().enumerate() {
                if i < MAX_SPHERES {
                    sphere_data[i] = *sphere;
                }
            }
            engine.update_buffer("spheres", &sphere_data)?;

            // Dispatch ray tracing compute shader
            let workgroups_x = (SCREEN_WIDTH + 15) / 16;
            let workgroups_y = (SCREEN_HEIGHT + 15) / 16;
            engine.dispatch("raytracing", [workgroups_x, workgroups_y, 1])?;
        }
        Ok(())
    }
}

impl haggis::simulation::traits::Simulation for GpuRayTracingSimulation {
    fn initialize(&mut self, _scene: &mut haggis::gfx::scene::Scene) {
        println!("Initializing GPU Ray Tracing simulation...");
    }

    fn update(&mut self, dt: f32, _scene: &mut haggis::gfx::scene::Scene) {
        if self.running {
            self.update_camera(dt);
            self.animate_spheres(dt);
            
            if let Err(e) = self.run_raytracing() {
                eprintln!("Ray tracing error: {}", e);
            }
        }
    }

    fn initialize_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Err(e) = self.setup_compute_engine(Arc::new(device.clone()), Arc::new(queue.clone())) {
            eprintln!("Failed to setup compute engine: {}", e);
            return;
        }
        println!("Ray tracing compute engine initialized!");
    }

    fn is_gpu_ready(&self) -> bool {
        self.compute_engine.is_some()
    }

    fn name(&self) -> &str {
        "GPU Ray Tracing"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, _scene: &mut haggis::gfx::scene::Scene) {
        self.camera_angle = 0.0;
        self.animation_time = 0.0;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("Ray Tracing Controls").build(|| {
            ui.text("GPU Ray Tracing Demo");
            ui.separator();
            
            ui.text(format!("Camera Position: ({:.2}, {:.2}, {:.2})", 
                self.params.camera_pos[0], self.params.camera_pos[1], self.params.camera_pos[2]));
            
            ui.text(format!("Spheres: {}", self.spheres.len()));
            ui.text(format!("Resolution: {}x{}", SCREEN_WIDTH, SCREEN_HEIGHT));
            
            ui.separator();
            ui.text("Features:");
            ui.bullet_text("Real-time ray-sphere intersection");
            ui.bullet_text("Lambert diffuse shading");
            ui.bullet_text("Multiple material types");
            ui.bullet_text("Animated camera and objects");
            ui.bullet_text("GPU compute shader implementation");
            
            if ui.button("Reset Camera") {
                self.camera_angle = 0.0;
                self.animation_time = 0.0;
            }
            
            if ui.button(if self.running { "Pause" } else { "Resume" }) {
                self.running = !self.running;
            }
        });
    }
}

fn main() {
    env_logger::init();

    let mut app = haggis::default();
    
    // Attach the ray tracing simulation
    app.attach_simulation(GpuRayTracingSimulation::new());
    
    app.run();
}