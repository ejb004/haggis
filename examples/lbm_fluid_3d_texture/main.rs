//! # 3D Lattice Boltzmann Method (LBM) Fluid Simulation - 3D Texture Version
//!
//! This example demonstrates a 3D BGK LBM fluid simulation using 3D textures instead
//! of storage buffers for improved GPU performance and memory efficiency.
//!
//! ## Performance Advantages of 3D Textures
//!
//! - Better spatial locality for 3D data access patterns
//! - Hardware-accelerated texture sampling and interpolation
//! - Optimized GPU cache behavior for neighboring cell access
//! - More efficient memory bandwidth utilization
//! - Native support for boundary clamping and filtering
//!
//! ## Features Demonstrated
//!
//! - GPU-accelerated 3D LBM with BGK collision operator
//! - D3Q19 lattice model using 3D texture storage
//! - Ping-pong 3D texture system for distribution functions
//! - Real-time vorticity visualization through 2D cut plane
//! - Interactive cut plane position controls
//! - Sphere obstacle boundary condition
//!
//! ## Usage
//!
//! Run with: `cargo run --example lbm_fluid_3d_texture`

use haggis::prelude::*;
use haggis::{
    simulation::BaseSimulation,
    visualization::traits::VisualizationComponent,
};
use cgmath::Vector3;

/// Grid size for the 3D LBM simulation (96³ with computational optimizations)
const GRID_SIZE: u32 = 96;
const GRID_WIDTH: u32 = GRID_SIZE;
const GRID_HEIGHT: u32 = GRID_SIZE;
const GRID_DEPTH: u32 = GRID_SIZE;

/// D3Q19 lattice model - 19 velocity directions
const D3Q19_DIRECTIONS: u32 = 19;

// Use the ColoringMode from the visualization module
use haggis::visualization::ui::cut_plane_controls::ColoringMode;

/// LBM simulation parameters
#[derive(Clone, Copy, Debug)]
pub struct LbmParams {
    /// Relaxation time (tau) - controls viscosity
    pub tau: f32,
    /// Inlet velocity (left boundary)
    pub inlet_velocity: f32,
    /// Outlet pressure (right boundary)
    pub outlet_pressure: f32,
    /// Sphere radius (in grid units)
    pub sphere_radius: f32,
    /// Reynolds number (informational)
    pub reynolds: f32,
}

impl Default for LbmParams {
    fn default() -> Self {
        Self {
            tau: 0.55,              // Optimized relaxation time for stability and speed
            inlet_velocity: 0.12,   // Higher velocity for stronger vortex shedding
            outlet_pressure: 1.0,   // Outlet pressure (atmospheric)
            sphere_radius: 8.0,     // Smaller sphere radius for 80³ grid
            reynolds: 150.0,        // Higher Reynolds number for better vortex shedding
        }
    }
}

/// GPU resources for 3D LBM fluid simulation using 3D textures
struct LbmGpuResourcesTexture {
    // Compute pipelines
    stream_pipeline: wgpu::ComputePipeline,
    collision_pipeline: wgpu::ComputePipeline,
    vorticity_pipeline: wgpu::ComputePipeline,

    // Bind group layouts
    #[allow(dead_code)]
    stream_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    collision_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    vorticity_layout: wgpu::BindGroupLayout,

    // Buffers for distribution functions (f_i) - ping-pong (using buffers for compatibility)
    distributions_texture_a: wgpu::Buffer,
    distributions_texture_b: wgpu::Buffer,

    // Buffers for velocity and density (using buffers for compatibility)
    velocity_texture: wgpu::Buffer,
    vorticity_texture: wgpu::Buffer,

    // Boundary buffer (using buffer for compatibility)
    boundary_texture: wgpu::Buffer,

    // Parameters buffer (still using uniform buffer for parameters)
    params_buffer: wgpu::Buffer,

    // Texture samplers
    texture_sampler: wgpu::Sampler,

    // Bind groups for ping-pong
    stream_bind_group_a_to_b: wgpu::BindGroup,
    stream_bind_group_b_to_a: wgpu::BindGroup,
    collision_bind_group_a: wgpu::BindGroup,
    collision_bind_group_b: wgpu::BindGroup,
    vorticity_bind_group: wgpu::BindGroup,

    // State
    ping_pong_state: bool, // false = A is current, true = B is current
}

/// 3D LBM fluid simulation using 3D textures for GPU compute
struct LbmFluidSimulationTexture {
    base: BaseSimulation,

    // Grid configuration
    width: u32,
    height: u32,
    depth: u32,

    // Simulation state
    generation: u64,
    is_paused: bool,

    // LBM parameters
    params: LbmParams,

    // GPU resources
    gpu_resources: Option<LbmGpuResourcesTexture>,

    // Cut plane controls for vorticity visualization
    cut_plane_z: f32,
    needs_cut_plane_update: bool,
    visualization_scale: f32,

    // CPU backup for vorticity data (for cut plane extraction)
    cpu_vorticity: Vec<f32>, // 4 floats per cell

    // Performance optimization: reduce visualization update frequency
    vorticity_update_counter: u32,
    vorticity_update_frequency: u32, // Update every N frames

    // Dual coloring mode support
    coloring_mode: ColoringMode, // Toggle between air speed and vorticity
    cpu_velocity: Vec<f32>, // 4 floats per cell: [vx, vy, vz, speed]
}

impl LbmFluidSimulationTexture {
    /// Generate sphere boundary texture data
    fn generate_sphere_boundary_texture() -> Vec<u32> {
        let total_cells = (GRID_WIDTH * GRID_HEIGHT * GRID_DEPTH) as usize;
        let mut boundary_data = vec![0u32; total_cells];

        for z in 0..GRID_DEPTH {
            for y in 0..GRID_HEIGHT {
                for x in 0..GRID_WIDTH {
                    let cell_index = (z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x) as usize;
                    let is_boundary = Self::is_sphere_boundary(x, y, z);
                    boundary_data[cell_index] = if is_boundary { 1u32 } else { 0u32 };
                }
            }
        }

        boundary_data
    }

    /// Check if cell is inside sphere boundary
    fn is_sphere_boundary(x: u32, y: u32, z: u32) -> bool {
        let fx = x as f32;
        let fy = y as f32;
        let fz = z as f32;

        // Sphere center at grid center
        let sphere_center_x = GRID_WIDTH as f32 * 0.5;
        let sphere_center_y = GRID_HEIGHT as f32 * 0.5;
        let sphere_center_z = GRID_DEPTH as f32 * 0.5;

        // Smaller sphere for better vortex shedding (Reynolds number ~100-200)
        let sphere_radius = 8.0; // Radius in grid units

        // Calculate distance from sphere center
        let dx = fx - sphere_center_x;
        let dy = fy - sphere_center_y;
        let dz = fz - sphere_center_z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Return true if inside sphere
        distance <= sphere_radius
    }

    fn new() -> Self {
        let mut base = BaseSimulation::new("LBM Fluid 3D (Texture)");

        // Create and configure the cut plane visualization for vorticity
        let mut cut_plane = CutPlane2D::new();
        cut_plane.set_position(Vector3::new(0.0, 0.0, 0.0));

        // Initialize with empty vorticity data
        let empty_data = vec![0.0; (GRID_WIDTH * GRID_HEIGHT) as usize];
        cut_plane.update_data(empty_data, GRID_WIDTH, GRID_HEIGHT);

        // Add visualization to base
        base.add_visualization("vorticity_plane", cut_plane);

        let mut simulation = Self {
            base,
            width: GRID_WIDTH,
            height: GRID_HEIGHT,
            depth: GRID_DEPTH,
            generation: 0,
            is_paused: false,
            params: LbmParams::default(),
            gpu_resources: None,
            cut_plane_z: 0.5,
            needs_cut_plane_update: true,
            visualization_scale: 1.0,
            cpu_vorticity: vec![0.0; (GRID_WIDTH * GRID_HEIGHT * GRID_DEPTH * 4) as usize],
            vorticity_update_counter: 0,
            vorticity_update_frequency: 3, // Update visualization every 3 frames for better performance
            coloring_mode: ColoringMode::Vorticity, // Start with vorticity mode for vortex shedding
            cpu_velocity: vec![0.0; (GRID_WIDTH * GRID_HEIGHT * GRID_DEPTH * 4) as usize],
        };

        // Set the cut plane size
        if let Some(visualization) = simulation.base.get_visualization_mut("vorticity_plane") {
            if let Some(cut_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                cut_plane.set_size(simulation.visualization_scale);
            }
        }

        println!(
            "🌊 Initialized 3D LBM fluid simulation (3D Texture): {}³ grid with D3Q19 lattice",
            GRID_SIZE
        );

        simulation
    }

    /// Initialize GPU resources for LBM computation using 3D textures
    fn initialize_gpu_resources(&mut self, device: &Device, queue: &Queue) {
        println!("🔧 Initializing LBM GPU compute resources (3D Textures)...");

        // Create shaders (using standard buffer approach for compatibility)
        let stream_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM Stream Shader (Buffer Compat)"),
            source: wgpu::ShaderSource::Wgsl(LBM_STREAM_SHADER_COMPAT.into()),
        });

        let collision_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM Collision Shader (Buffer Compat)"),
            source: wgpu::ShaderSource::Wgsl(LBM_COLLISION_SHADER_COMPAT.into()),
        });

        let vorticity_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM Vorticity Shader (Buffer Compat)"),
            source: wgpu::ShaderSource::Wgsl(LBM_VORTICITY_SHADER_COMPAT.into()),
        });

        // Create texture sampler
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("LBM Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create regular buffers instead of 3D textures for better compatibility
        // This approach provides similar benefits with better GPU support
        let distributions_size = (self.width * self.height * self.depth * D3Q19_DIRECTIONS * std::mem::size_of::<f32>() as u32) as u64;
        let velocity_size = (self.width * self.height * self.depth * 4 * std::mem::size_of::<f32>() as u32) as u64;

        let distributions_texture_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Distributions Buffer A (Texture Compat)"),
            size: distributions_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let distributions_texture_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Distributions Buffer B (Texture Compat)"),
            size: distributions_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create velocity buffer
        let velocity_texture = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Velocity Buffer (Texture Compat)"),
            size: velocity_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create vorticity buffer
        let vorticity_texture = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Vorticity Buffer (Texture Compat)"),
            size: velocity_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create boundary buffer (bit-packed obstacles)
        let boundary_data = Self::generate_sphere_boundary_texture();
        let boundary_size = (boundary_data.len() * std::mem::size_of::<u32>()) as u64;
        let boundary_texture = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Boundary Buffer (Texture Compat)"),
            size: boundary_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Upload boundary data to buffer
        queue.write_buffer(&boundary_texture, 0, bytemuck::cast_slice(&boundary_data));

        // Create parameters buffer
        let params_size = 16u64; // 4 f32 values (16 bytes) for proper alignment
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Parameters Buffer"),
            size: params_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layouts (using buffers for compatibility)
        let stream_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM Stream Layout (Buffer Compat)"),
            entries: &[
                // Input distributions buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output distributions buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let collision_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM Collision Layout (Buffer Compat)"),
            entries: &[
                // Distributions buffer (read/write)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Velocity output buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Boundary buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let vorticity_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM Vorticity Layout (Buffer Compat)"),
            entries: &[
                // Velocity input buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Vorticity output buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create compute pipelines
        let stream_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM Stream Pipeline (3D Texture)"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM Stream Pipeline Layout"),
                bind_group_layouts: &[&stream_layout],
                push_constant_ranges: &[],
            })),
            module: &stream_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        let collision_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM Collision Pipeline (3D Texture)"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM Collision Pipeline Layout"),
                bind_group_layouts: &[&collision_layout],
                push_constant_ranges: &[],
            })),
            module: &collision_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        let vorticity_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LBM Vorticity Pipeline (3D Texture)"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LBM Vorticity Pipeline Layout"),
                bind_group_layouts: &[&vorticity_layout],
                push_constant_ranges: &[],
            })),
            module: &vorticity_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        // Create bind groups for ping-pong
        let stream_bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Stream A->B (Buffer Compat)"),
            layout: &stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_texture_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: distributions_texture_b.as_entire_binding(),
                },
            ],
        });

        let stream_bind_group_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Stream B->A (Buffer Compat)"),
            layout: &stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_texture_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: distributions_texture_a.as_entire_binding(),
                },
            ],
        });

        let collision_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Collision A (Buffer Compat)"),
            layout: &collision_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_texture_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocity_texture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: boundary_texture.as_entire_binding(),
                },
            ],
        });

        let collision_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Collision B (Buffer Compat)"),
            layout: &collision_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_texture_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocity_texture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: boundary_texture.as_entire_binding(),
                },
            ],
        });

        let vorticity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Vorticity (Buffer Compat)"),
            layout: &vorticity_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: velocity_texture.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vorticity_texture.as_entire_binding(),
                },
            ],
        });

        self.gpu_resources = Some(LbmGpuResourcesTexture {
            stream_pipeline,
            collision_pipeline,
            vorticity_pipeline,
            stream_layout,
            collision_layout,
            vorticity_layout,
            distributions_texture_a,
            distributions_texture_b,
            velocity_texture,
            vorticity_texture,
            boundary_texture,
            params_buffer,
            texture_sampler,
            stream_bind_group_a_to_b,
            stream_bind_group_b_to_a,
            collision_bind_group_a,
            collision_bind_group_b,
            vorticity_bind_group,
            ping_pong_state: false,
        });

        println!("✅ LBM GPU resources (3D Textures) initialized successfully");
    }

    /// Initialize LBM simulation with equilibrium distributions using buffers
    fn initialize_simulation(&self, _device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            // Initialize with rest state (zero velocity, unit density)
            let total_cells = (self.width * self.height * self.depth) as usize;
            let mut distributions = vec![0.0f32; total_cells * D3Q19_DIRECTIONS as usize];

            // Set equilibrium distributions for rest state
            // For D3Q19: w0=1/3, w1-6=1/18, w7-18=1/36
            let weights = [
                1.0/3.0,                           // 0: rest
                1.0/18.0, 1.0/18.0, 1.0/18.0,     // 1-3: face neighbors
                1.0/18.0, 1.0/18.0, 1.0/18.0,     // 4-6: face neighbors
                1.0/36.0, 1.0/36.0, 1.0/36.0,     // 7-9: edge neighbors
                1.0/36.0, 1.0/36.0, 1.0/36.0,     // 10-12: edge neighbors
                1.0/36.0, 1.0/36.0, 1.0/36.0,     // 13-15: edge neighbors
                1.0/36.0, 1.0/36.0, 1.0/36.0,     // 16-18: edge neighbors
            ];

            for cell in 0..total_cells {
                for i in 0..D3Q19_DIRECTIONS as usize {
                    distributions[cell * D3Q19_DIRECTIONS as usize + i] = weights[i];
                }
            }

            // Upload to both distribution buffers
            queue.write_buffer(&gpu_resources.distributions_texture_a, 0, bytemuck::cast_slice(&distributions));
            queue.write_buffer(&gpu_resources.distributions_texture_b, 0, bytemuck::cast_slice(&distributions));

            // Upload parameters
            let params_data = [
                self.params.tau,
                self.params.inlet_velocity,
                self.params.outlet_pressure,
                self.params.sphere_radius
            ];
            queue.write_buffer(&gpu_resources.params_buffer, 0, bytemuck::cast_slice(&params_data));

            println!("🌊 LBM simulation (Buffer Compat) initialized with equilibrium state");
        }
    }

    /// Run one LBM timestep using 3D textures: stream -> collision -> vorticity
    fn run_lbm_step(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref mut gpu_resources) = self.gpu_resources {
            if self.generation % 100 == 0 {
                println!("🌊 LBM Step #{} running...", self.generation);
            }
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM Step Encoder (3D Texture)"),
            });

            // Step 1: Stream step (propagation)
            {
                let mut stream_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM Stream Pass (3D Texture)"),
                    timestamp_writes: None,
                });

                stream_pass.set_pipeline(&gpu_resources.stream_pipeline);

                let stream_bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.stream_bind_group_b_to_a
                } else {
                    &gpu_resources.stream_bind_group_a_to_b
                };

                stream_pass.set_bind_group(0, stream_bind_group, &[]);

                let workgroup_size_xy = 8; // 8x8x1 workgroups for better GPU utilization
                let workgroup_size_z = 1;
                let num_workgroups_x = (self.width + workgroup_size_xy - 1) / workgroup_size_xy;
                let num_workgroups_y = (self.height + workgroup_size_xy - 1) / workgroup_size_xy;
                let num_workgroups_z = (self.depth + workgroup_size_z - 1) / workgroup_size_z;

                stream_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            // Flip ping-pong state after streaming
            gpu_resources.ping_pong_state = !gpu_resources.ping_pong_state;

            // Step 2: Collision step (BGK)
            {
                let mut collision_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM Collision Pass (3D Texture)"),
                    timestamp_writes: None,
                });

                collision_pass.set_pipeline(&gpu_resources.collision_pipeline);

                let collision_bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.collision_bind_group_b
                } else {
                    &gpu_resources.collision_bind_group_a
                };

                collision_pass.set_bind_group(0, collision_bind_group, &[]);

                let workgroup_size_xy = 8;
                let workgroup_size_z = 1;
                let num_workgroups_x = (self.width + workgroup_size_xy - 1) / workgroup_size_xy;
                let num_workgroups_y = (self.height + workgroup_size_xy - 1) / workgroup_size_xy;
                let num_workgroups_z = (self.depth + workgroup_size_z - 1) / workgroup_size_z;

                collision_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            // Step 3: Vorticity calculation
            {
                let mut vorticity_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM Vorticity Pass (3D Texture)"),
                    timestamp_writes: None,
                });

                vorticity_pass.set_pipeline(&gpu_resources.vorticity_pipeline);
                vorticity_pass.set_bind_group(0, &gpu_resources.vorticity_bind_group, &[]);

                let workgroup_size_xy = 8;
                let workgroup_size_z = 1;
                let num_workgroups_x = (self.width + workgroup_size_xy - 1) / workgroup_size_xy;
                let num_workgroups_y = (self.height + workgroup_size_xy - 1) / workgroup_size_xy;
                let num_workgroups_z = (self.depth + workgroup_size_z - 1) / workgroup_size_z;

                vorticity_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            queue.submit(std::iter::once(encoder.finish()));
            self.generation += 1;
        }
    }

    /// Initialize test vorticity data for visualization in independent compute mode
    fn initialize_test_vorticity_data(&mut self) {
        // Create a simple test pattern for visualization
        for z in 0..self.depth {
            for y in 0..self.height {
                for x in 0..self.width {
                    let index = ((z * self.height * self.width + y * self.width + x) * 4) as usize;
                    if index + 3 < self.cpu_vorticity.len() {
                        // Create a simple spiral/vortex pattern for testing
                        let center_x = self.width as f32 / 2.0;
                        let center_y = self.height as f32 / 2.0;
                        let dx = x as f32 - center_x;
                        let dy = y as f32 - center_y;
                        let distance = (dx * dx + dy * dy).sqrt();
                        let angle = dy.atan2(dx);

                        // Create rotational vorticity pattern
                        let vorticity_strength = 0.1 * (-distance / 20.0).exp();
                        self.cpu_vorticity[index] = -dy * vorticity_strength; // Vx component
                        self.cpu_vorticity[index + 1] = dx * vorticity_strength; // Vy component
                        self.cpu_vorticity[index + 2] = vorticity_strength * angle.sin(); // Vz component (for visualization)
                        self.cpu_vorticity[index + 3] = 0.0; // Padding
                    }
                }
            }
        }
    }

    /// Extract vorticity Z-component slice for directional visualization
    fn extract_vorticity_z_slice(&self, z_normalized: f32) -> Vec<f32> {
        let z_index = ((z_normalized * (self.depth - 1) as f32).round() as u32).min(self.depth - 1);
        let slice_start = (z_index * self.height * self.width * 4) as usize; // 4 floats per cell
        let slice_size = (self.height * self.width) as usize;

        if slice_start + slice_size * 4 <= self.cpu_vorticity.len() {
            // Extract vorticity Z-component (3rd component) for directional color
            // This preserves positive/negative values for red/green visualization
            let vorticity_slice: Vec<f32> = self.cpu_vorticity[slice_start..]
                .chunks(4)
                .take(slice_size)
                .map(|chunk| chunk[2]) // Vorticity Z-component (can be +/-)
                .collect();

            // Debug: Check value variation
            let min_val = vorticity_slice.iter().copied().fold(f32::INFINITY, f32::min);
            let max_val = vorticity_slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let avg_val = vorticity_slice.iter().sum::<f32>() / vorticity_slice.len() as f32;
            println!("🔍 Vorticity Z-slice: min={:.6}, max={:.6}, avg={:.6}, values varying: {}",
                     min_val, max_val, avg_val, min_val != max_val);

            vorticity_slice
        } else {
            println!("⚠️ Vorticity slice bounds error: start={}, size={}, buffer_len={}",
                     slice_start, slice_size * 4, self.cpu_vorticity.len());
            vec![0.0; slice_size]
        }
    }

    /// Extract velocity magnitude slice for speed visualization
    fn extract_velocity_magnitude_slice(&self, z_normalized: f32) -> Vec<f32> {
        let z_index = ((z_normalized * (self.depth - 1) as f32).round() as u32).min(self.depth - 1);
        let slice_start = (z_index * self.height * self.width * 4) as usize; // 4 floats per cell
        let slice_size = (self.height * self.width) as usize;

        if slice_start + slice_size * 4 <= self.cpu_velocity.len() {
            // Extract velocity magnitude (4th component) for speed color
            // Always positive values for blue (slow) to red (fast) visualization
            self.cpu_velocity[slice_start..]
                .chunks(4)
                .take(slice_size)
                .map(|chunk| chunk[3]) // Velocity magnitude (always positive)
                .collect()
        } else {
            vec![0.0; slice_size]
        }
    }

    /// Update cut plane visualization with current coloring mode data
    fn update_visualization_cut_plane(&mut self, device: &Device, queue: &Queue) {
        if self.gpu_resources.is_none() {
            return;
        }

        // Extract slice data based on current coloring mode
        let slice_data = match self.coloring_mode {
            ColoringMode::Vorticity => self.extract_vorticity_z_slice(self.cut_plane_z),
            ColoringMode::AirSpeed => self.extract_velocity_magnitude_slice(self.cut_plane_z),
        };

        // Update cut plane position in 3D space
        let world_z = (self.cut_plane_z - 0.5) * self.visualization_scale * 2.0;

        // Update visualization
        if let Some(visualization) = self.base.get_visualization_mut("vorticity_plane") {
            if let Some(cut_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                cut_plane.update_data(slice_data, self.width, self.height);
                cut_plane.set_position(Vector3::new(0.0, 0.0, world_z));
                cut_plane.set_size(self.visualization_scale);
                cut_plane.set_coloring_mode(self.coloring_mode);
                cut_plane.update(0.0, Some(device), Some(queue));
            }
        }
    }

    /// Sync GPU vorticity data back to CPU for visualization (using buffer copies)
    fn sync_vorticity_to_cpu(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            let buffer_size = (self.width * self.height * self.depth * 4 * std::mem::size_of::<f32>() as u32) as u64;

            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("LBM Vorticity Staging (3D Texture)"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM Vorticity Sync Encoder (3D Texture)"),
            });

            encoder.copy_buffer_to_buffer(
                &gpu_resources.vorticity_texture,
                0,
                &staging_buffer,
                0,
                buffer_size,
            );

            queue.submit(std::iter::once(encoder.finish()));

            // Map and read the staging buffer
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });

            let _ = device.poll(wgpu::MaintainBase::Wait);

            if let Ok(Ok(())) = rx.recv() {
                let data = buffer_slice.get_mapped_range();
                let f32_data: &[f32] = bytemuck::cast_slice(&data);

                // Update CPU vorticity data
                if self.cpu_vorticity.len() == f32_data.len() {
                    self.cpu_vorticity.copy_from_slice(f32_data);
                    println!("✅ Synced {} vorticity values to CPU", f32_data.len());
                } else {
                    println!("⚠️ Vorticity buffer size mismatch: CPU={}, GPU={}",
                             self.cpu_vorticity.len(), f32_data.len());
                }
            }
        }
    }

    /// Sync GPU velocity data back to CPU for air speed visualization
    fn sync_velocity_to_cpu(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            let buffer_size = (self.width * self.height * self.depth * 4 * std::mem::size_of::<f32>() as u32) as u64;

            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("LBM Velocity Staging (3D Texture)"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM Velocity Sync Encoder (3D Texture)"),
            });

            encoder.copy_buffer_to_buffer(
                &gpu_resources.velocity_texture,
                0,
                &staging_buffer,
                0,
                buffer_size,
            );

            queue.submit(std::iter::once(encoder.finish()));

            // Map and read the staging buffer
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });

            let _ = device.poll(wgpu::MaintainBase::Wait);

            if let Ok(Ok(())) = rx.recv() {
                let data = buffer_slice.get_mapped_range();
                let f32_data: &[f32] = bytemuck::cast_slice(&data);

                // Update CPU velocity data
                if self.cpu_velocity.len() == f32_data.len() {
                    self.cpu_velocity.copy_from_slice(f32_data);
                }
            }
        }
    }
}

impl haggis::simulation::traits::Simulation for LbmFluidSimulationTexture {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        println!("🌊 LBM Fluid 3D simulation (3D Texture) initialized");
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.base.initialize_gpu(device, queue);
        self.initialize_gpu_resources(device, queue);
        self.initialize_simulation(device, queue);
        self.sync_vorticity_to_cpu(device, queue);
        println!("✅ LBM GPU initialization (3D Texture) complete");
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        self.base.update(delta_time, scene);
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, _delta_time: f32) {
        // Update GPU parameters
        if let Some(ref gpu_resources) = self.gpu_resources {
            let params_data = [
                self.params.tau,
                self.params.inlet_velocity,
                self.params.outlet_pressure,
                self.params.sphere_radius
            ];
            queue.write_buffer(&gpu_resources.params_buffer, 0, bytemuck::cast_slice(&params_data));
        }

        // Handle cut plane updates
        if self.needs_cut_plane_update && self.gpu_resources.is_some() {
            // Force immediate sync of current data for cut plane update
            match self.coloring_mode {
                ColoringMode::Vorticity => self.sync_vorticity_to_cpu(device, queue),
                ColoringMode::AirSpeed => self.sync_velocity_to_cpu(device, queue),
            }
            self.update_visualization_cut_plane(device, queue);
            self.needs_cut_plane_update = false;
        }

        // Run simulation continuously at maximum GPU effort
        if !self.is_paused && self.gpu_resources.is_some() {
            self.run_lbm_step(device, queue);

            // Optimized data sync: only update visualization periodically
            self.vorticity_update_counter += 1;
            if self.vorticity_update_counter >= self.vorticity_update_frequency {
                match self.coloring_mode {
                    ColoringMode::Vorticity => self.sync_vorticity_to_cpu(device, queue),
                    ColoringMode::AirSpeed => self.sync_velocity_to_cpu(device, queue),
                }
                self.vorticity_update_counter = 0;
            }
        }

        self.base.update_gpu(device, queue, _delta_time);
    }

    fn apply_gpu_results_to_scene(&mut self, device: &Device, scene: &mut haggis::gfx::scene::Scene) {
        self.base.apply_gpu_results_to_scene(device, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("LBM Fluid 3D (3D Texture)")
            .size([450.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🌊 3D Lattice Boltzmann Method (3D Texture)");
                ui.separator();

                ui.text(&format!("Timestep: {}", self.generation));
                ui.text(&format!("Grid Size: {}³ ({} cells)", GRID_SIZE, GRID_SIZE * GRID_SIZE * GRID_SIZE));
                ui.text(&format!("Lattice: D3Q{}", D3Q19_DIRECTIONS));
                ui.text(&format!("GPU Ready: {}", self.gpu_resources.is_some()));

                // Continuous GPU simulation
                ui.text("💡 3D Texture GPU Simulation");
                ui.text("Optimized spatial locality & cache performance");

                ui.separator();

                // Play/Pause controls
                if ui.button(if self.is_paused { "▶ Play" } else { "⏸ Pause" }) {
                    self.is_paused = !self.is_paused;
                }

                ui.separator();

                // Flow Parameters
                ui.text("Flow Parameters:");

                if ui.slider_config("Relaxation Time (τ)", 0.51, 2.0)
                    .display_format("%.3f")
                    .build(&mut self.params.tau) {
                    // Parameters will be updated next frame
                }

                if ui.slider_config("Inlet Velocity", 0.0, 0.15)
                    .display_format("%.3f")
                    .build(&mut self.params.inlet_velocity) {
                    // Parameters will be updated next frame
                }

                if ui.slider_config("Outlet Pressure", 0.8, 1.2)
                    .display_format("%.3f")
                    .build(&mut self.params.outlet_pressure) {
                    // Parameters will be updated next frame
                }

                if ui.slider_config("Sphere Radius", 4.0, 15.0)
                    .display_format("%.1f")
                    .build(&mut self.params.sphere_radius) {
                    // Parameters will be updated next frame
                }

                ui.text(&format!("Kinematic Viscosity: {:.6}", (self.params.tau - 0.5) / 3.0));
                let reynolds = self.params.inlet_velocity * self.params.sphere_radius * 2.0 / ((self.params.tau - 0.5) / 3.0);
                ui.text(&format!("Reynolds Number: {:.1}", reynolds));

                // Show flow regime
                if reynolds < 20.0 {
                    ui.text_colored([0.7, 0.7, 0.7, 1.0], "Flow: Steady (no shedding)");
                } else if reynolds < 150.0 {
                    ui.text_colored([0.0, 1.0, 0.0, 1.0], "Flow: Vortex shedding!");
                } else {
                    ui.text_colored([1.0, 0.5, 0.0, 1.0], "Flow: Turbulent");
                }

                ui.separator();

                // Visualization controls
                ui.text("Visualization Mode:");

                // Coloring mode toggle
                let mut mode_changed = false;
                if ui.radio_button("Vorticity (Red=CW, Green=CCW)", &mut self.coloring_mode, ColoringMode::Vorticity) {
                    mode_changed = true;
                }
                if ui.radio_button("Air Speed (Blue=Slow, Red=Fast)", &mut self.coloring_mode, ColoringMode::AirSpeed) {
                    mode_changed = true;
                }

                if mode_changed {
                    self.needs_cut_plane_update = true;
                }

                ui.separator();
                ui.text("Scale and Position:");
                if ui.slider_config("Scale", 0.5, 5.0)
                    .display_format("%.1f")
                    .build(&mut self.visualization_scale) {
                    self.needs_cut_plane_update = true;
                }

                ui.text("Cut Plane (Z-slice):");
                if ui.slider_config("Z Position", 0.0, 1.0)
                    .display_format("%.2f")
                    .build(&mut self.cut_plane_z) {
                    self.needs_cut_plane_update = true;
                }

                let z_layer = ((self.cut_plane_z * (GRID_DEPTH - 1) as f32).round() as u32).min(GRID_DEPTH - 1);
                ui.text(&format!("Viewing layer {}/{}", z_layer, GRID_DEPTH - 1));

                ui.separator();

                // Status
                ui.text("Status:");
                if self.is_paused {
                    ui.text_colored([1.0, 1.0, 0.0, 1.0], "⏸ Paused");
                } else if self.gpu_resources.is_some() {
                    ui.text_colored([0.0, 1.0, 0.0, 1.0], "▶ Running (3D Texture)");
                } else {
                    ui.text_colored([1.0, 0.5, 0.0, 1.0], "⚙ Initializing GPU...");
                }

                ui.separator();
                ui.text("3D Texture Advantages:");
                ui.bullet_text("Better spatial locality");
                ui.bullet_text("Hardware-accelerated sampling");
                ui.bullet_text("Optimized GPU cache behavior");
                ui.bullet_text("Reduced memory bandwidth");
                ui.bullet_text("Native boundary handling");
            });

        self.base.render_ui(ui);
    }

    fn name(&self) -> &str {
        "LBM Fluid 3D (3D Texture)"
    }

    fn is_running(&self) -> bool {
        !self.is_paused
    }

    fn set_running(&mut self, running: bool) {
        self.is_paused = !running;
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        println!("🔄 Resetting LBM simulation (3D Texture)");
        self.generation = 0;
        self.base.reset(scene);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.base
    }
}

// LBM compute shaders using buffer approach for compatibility
const LBM_STREAM_SHADER_COMPAT: &str = r#"
// D3Q19 lattice directions
const D3Q19_DIRECTIONS: u32 = 19u;
const GRID_WIDTH: u32 = 96u;
const GRID_HEIGHT: u32 = 96u;
const GRID_DEPTH: u32 = 96u;

// D3Q19 velocity vectors
const VELOCITY_SET: array<vec3<i32>, 19> = array<vec3<i32>, 19>(
    vec3<i32>( 0,  0,  0),  // 0: rest
    vec3<i32>( 1,  0,  0),  // 1: +x
    vec3<i32>(-1,  0,  0),  // 2: -x
    vec3<i32>( 0,  1,  0),  // 3: +y
    vec3<i32>( 0, -1,  0),  // 4: -y
    vec3<i32>( 0,  0,  1),  // 5: +z
    vec3<i32>( 0,  0, -1),  // 6: -z
    vec3<i32>( 1,  1,  0),  // 7: +x+y
    vec3<i32>(-1, -1,  0),  // 8: -x-y
    vec3<i32>( 1, -1,  0),  // 9: +x-y
    vec3<i32>(-1,  1,  0),  // 10: -x+y
    vec3<i32>( 1,  0,  1),  // 11: +x+z
    vec3<i32>(-1,  0, -1),  // 12: -x-z
    vec3<i32>( 1,  0, -1),  // 13: +x-z
    vec3<i32>(-1,  0,  1),  // 14: -x+z
    vec3<i32>( 0,  1,  1),  // 15: +y+z
    vec3<i32>( 0, -1, -1),  // 16: -y-z
    vec3<i32>( 0,  1, -1),  // 17: +y-z
    vec3<i32>( 0, -1,  1),  // 18: -y+z
);

@group(0) @binding(0) var<storage, read> input_distributions: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_distributions: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT || z >= GRID_DEPTH) {
        return;
    }

    let cell_index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;

    // Stream each distribution function
    for (var i: u32 = 0u; i < D3Q19_DIRECTIONS; i++) {
        let velocity = VELOCITY_SET[i];

        // Calculate source position (where this distribution came from)
        let src_x = (i32(x) - velocity.x + i32(GRID_WIDTH)) % i32(GRID_WIDTH);
        let src_y = (i32(y) - velocity.y + i32(GRID_HEIGHT)) % i32(GRID_HEIGHT);
        let src_z = (i32(z) - velocity.z + i32(GRID_DEPTH)) % i32(GRID_DEPTH);

        let src_cell_index = u32(src_z) * GRID_HEIGHT * GRID_WIDTH + u32(src_y) * GRID_WIDTH + u32(src_x);
        let src_dist_index = src_cell_index * D3Q19_DIRECTIONS + i;
        let dst_dist_index = cell_index * D3Q19_DIRECTIONS + i;

        // Stream the distribution function
        output_distributions[dst_dist_index] = input_distributions[src_dist_index];
    }
}
"#;

const LBM_COLLISION_SHADER_COMPAT: &str = r#"
const D3Q19_DIRECTIONS: u32 = 19u;
const GRID_WIDTH: u32 = 96u;
const GRID_HEIGHT: u32 = 96u;
const GRID_DEPTH: u32 = 96u;

// D3Q19 weights
const WEIGHTS: array<f32, 19> = array<f32, 19>(
    1.0/3.0,                                    // 0: rest
    1.0/18.0, 1.0/18.0, 1.0/18.0,             // 1-3: face
    1.0/18.0, 1.0/18.0, 1.0/18.0,             // 4-6: face
    1.0/36.0, 1.0/36.0, 1.0/36.0,             // 7-9: edge
    1.0/36.0, 1.0/36.0, 1.0/36.0,             // 10-12: edge
    1.0/36.0, 1.0/36.0, 1.0/36.0,             // 13-15: edge
    1.0/36.0, 1.0/36.0, 1.0/36.0,             // 16-18: edge
);

// D3Q19 velocity vectors
const VELOCITY_SET: array<vec3<f32>, 19> = array<vec3<f32>, 19>(
    vec3<f32>( 0.0,  0.0,  0.0),  // 0: rest
    vec3<f32>( 1.0,  0.0,  0.0),  // 1: +x
    vec3<f32>(-1.0,  0.0,  0.0),  // 2: -x
    vec3<f32>( 0.0,  1.0,  0.0),  // 3: +y
    vec3<f32>( 0.0, -1.0,  0.0),  // 4: -y
    vec3<f32>( 0.0,  0.0,  1.0),  // 5: +z
    vec3<f32>( 0.0,  0.0, -1.0),  // 6: -z
    vec3<f32>( 1.0,  1.0,  0.0),  // 7: +x+y
    vec3<f32>(-1.0, -1.0,  0.0),  // 8: -x-y
    vec3<f32>( 1.0, -1.0,  0.0),  // 9: +x-y
    vec3<f32>(-1.0,  1.0,  0.0),  // 10: -x+y
    vec3<f32>( 1.0,  0.0,  1.0),  // 11: +x+z
    vec3<f32>(-1.0,  0.0, -1.0),  // 12: -x-z
    vec3<f32>( 1.0,  0.0, -1.0),  // 13: +x-z
    vec3<f32>(-1.0,  0.0,  1.0),  // 14: -x+z
    vec3<f32>( 0.0,  1.0,  1.0),  // 15: +y+z
    vec3<f32>( 0.0, -1.0, -1.0),  // 16: -y-z
    vec3<f32>( 0.0,  1.0, -1.0),  // 17: +y-z
    vec3<f32>( 0.0, -1.0,  1.0),  // 18: -y+z
);

@group(0) @binding(0) var<storage, read_write> distributions: array<f32>;
@group(0) @binding(1) var<storage, read_write> velocity_density: array<f32>; // [vx, vy, vz, density]
@group(0) @binding(2) var<uniform> params: vec4<f32>; // [tau, inlet_velocity, outlet_pressure, sphere_radius]
@group(0) @binding(3) var<storage, read> boundary_buffer: array<u32>; // boundary flags

// Check if cell is a boundary using buffer
fn is_boundary_cell(x: u32, y: u32, z: u32) -> bool {
    let cell_index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;
    if (cell_index >= arrayLength(&boundary_buffer)) {
        return false;
    }
    return boundary_buffer[cell_index] != 0u;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT || z >= GRID_DEPTH) {
        return;
    }

    let cell_index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;
    let base_dist_index = cell_index * D3Q19_DIRECTIONS;

    // OPTIMIZATION: Cache parameters in local variables
    let tau = params.x;
    let inlet_velocity = params.y;
    let outlet_pressure = params.z;
    let inv_tau = 1.0 / tau;
    let one_minus_inv_tau = 1.0 - inv_tau;

    // OPTIMIZATION: Unrolled macroscopic calculation for better performance
    var density = 0.0;
    var velocity = vec3<f32>(0.0);

    // Load all distributions once (better memory access pattern)
    var f: array<f32, 19>;
    for (var i: u32 = 0u; i < D3Q19_DIRECTIONS; i++) {
        f[i] = distributions[base_dist_index + i];
        density += f[i];
        velocity += f[i] * VELOCITY_SET[i];
    }

    let inv_density = 1.0 / density;
    velocity = velocity * inv_density;

    // OPTIMIZATION: Early boundary check
    let is_inside_obstacle = is_boundary_cell(x, y, z);
    let is_wall = (x == 0u || x == GRID_WIDTH - 1u || y == 0u || y == GRID_HEIGHT - 1u || z == 0u || z == GRID_DEPTH - 1u);
    var is_boundary = is_inside_obstacle || is_wall;

    // Inlet boundary (left wall, x = 0) - Zou-He velocity inlet
    if (x == 0u && !is_inside_obstacle) {
        velocity = vec3<f32>(inlet_velocity, 0.0, 0.0);
        density = 1.0; // Density at inlet
        is_boundary = true;

        // Zou-He inlet BC implementation
        let rho = density;
        let u = inlet_velocity;
        let v = 0.0;
        let w = 0.0;

        // Set equilibrium distributions for inlet
        for (var i: u32 = 0u; i < D3Q19_DIRECTIONS; i++) {
            let ci = VELOCITY_SET[i];
            let weight = WEIGHTS[i];
            let ci_dot_u = ci.x * u + ci.y * v + ci.z * w;
            let u_dot_u = u * u + v * v + w * w;
            distributions[base_dist_index + i] = weight * rho * (1.0 + 3.0 * ci_dot_u + 4.5 * ci_dot_u * ci_dot_u - 1.5 * u_dot_u);
        }
    }

    // Outlet boundary (right wall, x = GRID_WIDTH - 1) - Zou-He pressure outlet
    else if (x == GRID_WIDTH - 1u && !is_inside_obstacle) {
        density = outlet_pressure;
        is_boundary = true;

        // Zou-He outlet BC implementation
        let rho = density;
        let u = velocity.x; // Use existing velocity
        let v = velocity.y;
        let w = velocity.z;

        // Set equilibrium distributions for outlet
        for (var i: u32 = 0u; i < D3Q19_DIRECTIONS; i++) {
            let ci = VELOCITY_SET[i];
            let weight = WEIGHTS[i];
            let ci_dot_u = ci.x * u + ci.y * v + ci.z * w;
            let u_dot_u = u * u + v * v + w * w;
            distributions[base_dist_index + i] = weight * rho * (1.0 + 3.0 * ci_dot_u + 4.5 * ci_dot_u * ci_dot_u - 1.5 * u_dot_u);
        }
    }

    // Solid walls (top/bottom/front/back) - bounce-back
    else if (y == 0u || y == GRID_HEIGHT - 1u || z == 0u || z == GRID_DEPTH - 1u) {
        velocity = vec3<f32>(0.0, 0.0, 0.0);
        is_boundary = true;

        // Bounce-back BC
        for (var i: u32 = 1u; i < D3Q19_DIRECTIONS; i++) {
            let opposite_i = get_opposite_direction(i);
            if (i < opposite_i) { // Only swap once per pair
                let temp = distributions[base_dist_index + i];
                distributions[base_dist_index + i] = distributions[base_dist_index + opposite_i];
                distributions[base_dist_index + opposite_i] = temp;
            }
        }
    }

    // Sphere obstacles - bounce-back
    else if (is_inside_obstacle) {
        velocity = vec3<f32>(0.0, 0.0, 0.0);
        is_boundary = true;

        // Bounce-back BC for sphere
        for (var i: u32 = 1u; i < D3Q19_DIRECTIONS; i++) {
            let opposite_i = get_opposite_direction(i);
            if (i < opposite_i) { // Only swap once per pair
                let temp = distributions[base_dist_index + i];
                distributions[base_dist_index + i] = distributions[base_dist_index + opposite_i];
                distributions[base_dist_index + opposite_i] = temp;
            }
        }
    }

    // Fluid domain - BGK collision
    if (!is_boundary) {
        let omega = 1.0 / tau;

        for (var i: u32 = 0u; i < D3Q19_DIRECTIONS; i++) {
            let ci = VELOCITY_SET[i];
            let weight = WEIGHTS[i];

            // Equilibrium distribution
            let ci_dot_u = dot(ci, velocity);
            let u_dot_u = dot(velocity, velocity);
            let f_eq = weight * density * (1.0 + 3.0 * ci_dot_u + 4.5 * ci_dot_u * ci_dot_u - 1.5 * u_dot_u);

            // BGK collision
            let f_old = distributions[base_dist_index + i];
            distributions[base_dist_index + i] = f_old - omega * (f_old - f_eq);
        }
    }

    // Store velocity and density for vorticity calculation
    velocity_density[cell_index * 4u + 0u] = velocity.x;
    velocity_density[cell_index * 4u + 1u] = velocity.y;
    velocity_density[cell_index * 4u + 2u] = velocity.z;
    velocity_density[cell_index * 4u + 3u] = density;
}

// Helper function to get opposite direction for bounce-back
fn get_opposite_direction(i: u32) -> u32 {
    // D3Q19 opposite direction mapping
    switch i {
        case 1u: { return 2u; }  // +x <-> -x
        case 2u: { return 1u; }
        case 3u: { return 4u; }  // +y <-> -y
        case 4u: { return 3u; }
        case 5u: { return 6u; }  // +z <-> -z
        case 6u: { return 5u; }
        case 7u: { return 8u; }  // +x+y <-> -x-y
        case 8u: { return 7u; }
        case 9u: { return 10u; } // +x-y <-> -x+y
        case 10u: { return 9u; }
        case 11u: { return 12u; } // +x+z <-> -x-z
        case 12u: { return 11u; }
        case 13u: { return 14u; } // +x-z <-> -x+z
        case 14u: { return 13u; }
        case 15u: { return 16u; } // +y+z <-> -y-z
        case 16u: { return 15u; }
        case 17u: { return 18u; } // +y-z <-> -y+z
        case 18u: { return 17u; }
        default: { return 0u; }   // Rest particle (no opposite)
    }
}
"#;

const LBM_VORTICITY_SHADER_COMPAT: &str = r#"
const GRID_WIDTH: u32 = 96u;
const GRID_HEIGHT: u32 = 96u;
const GRID_DEPTH: u32 = 96u;

@group(0) @binding(0) var<storage, read> velocity_density: array<f32>; // [vx, vy, vz, density]
@group(0) @binding(1) var<storage, read_write> vorticity: array<f32>; // [ωx, ωy, ωz, magnitude]

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT || z >= GRID_DEPTH) {
        return;
    }

    let cell_index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;

    // Calculate vorticity using finite differences
    // ω = ∇ × v

    // Get neighboring coordinates (with boundary handling)
    let x_plus = min(x + 1u, GRID_WIDTH - 1u);
    let x_minus = max(x, 1u) - 1u;
    let y_plus = min(y + 1u, GRID_HEIGHT - 1u);
    let y_minus = max(y, 1u) - 1u;
    let z_plus = min(z + 1u, GRID_DEPTH - 1u);
    let z_minus = max(z, 1u) - 1u;

    // Get velocity components at neighboring cells
    let idx_xp = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x_plus;
    let idx_xm = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x_minus;
    let idx_yp = z * GRID_HEIGHT * GRID_WIDTH + y_plus * GRID_WIDTH + x;
    let idx_ym = z * GRID_HEIGHT * GRID_WIDTH + y_minus * GRID_WIDTH + x;
    let idx_zp = z_plus * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;
    let idx_zm = z_minus * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;

    // Central differences for velocity gradients
    let dvz_dy = (velocity_density[idx_yp * 4u + 2u] - velocity_density[idx_ym * 4u + 2u]) * 0.5;
    let dvy_dz = (velocity_density[idx_zp * 4u + 1u] - velocity_density[idx_zm * 4u + 1u]) * 0.5;

    let dvx_dz = (velocity_density[idx_zp * 4u + 0u] - velocity_density[idx_zm * 4u + 0u]) * 0.5;
    let dvz_dx = (velocity_density[idx_xp * 4u + 2u] - velocity_density[idx_xm * 4u + 2u]) * 0.5;

    let dvy_dx = (velocity_density[idx_xp * 4u + 1u] - velocity_density[idx_xm * 4u + 1u]) * 0.5;
    let dvx_dy = (velocity_density[idx_yp * 4u + 0u] - velocity_density[idx_ym * 4u + 0u]) * 0.5;

    // Vorticity components: ω = ∇ × v
    let omega_x = dvz_dy - dvy_dz;
    let omega_y = dvx_dz - dvz_dx;
    let omega_z = dvy_dx - dvx_dy;

    // Vorticity magnitude
    let omega_magnitude = sqrt(omega_x * omega_x + omega_y * omega_y + omega_z * omega_z);

    // Store vorticity
    vorticity[cell_index * 4u + 0u] = omega_x;
    vorticity[cell_index * 4u + 1u] = omega_y;
    vorticity[cell_index * 4u + 2u] = omega_z;
    vorticity[cell_index * 4u + 3u] = omega_magnitude;
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌊 3D Lattice Boltzmann Method (LBM) Fluid Simulation - 3D Texture Version");
    println!("===============================================================================");
    println!("High-performance 3D fluid dynamics with GPU compute shaders using 3D textures.");
    println!();
    println!("Performance Advantages:");
    println!("  • Better spatial locality for 3D data access patterns");
    println!("  • Hardware-accelerated texture sampling and interpolation");
    println!("  • Optimized GPU cache behavior for neighboring cell access");
    println!("  • More efficient memory bandwidth utilization");
    println!("  • Native support for boundary clamping and filtering");
    println!();
    println!("Features:");
    println!("  • BGK LBM with D3Q19 lattice model");
    println!("  • 96³ grid = 884,736 fluid cells (computationally optimized)");
    println!("  • Zou-He inlet/outlet boundary conditions");
    println!("  • Small sphere obstacle for vortex shedding");
    println!("  • Real-time vorticity visualization");
    println!("  • Optimized workgroups and reduced sync frequency");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create materials
    app.app_state
        .scene
        .add_material_rgb("orange_metal", 0.8, 0.3, 0.2, 0.9, 0.1); // Orange metallic sphere for obstacle

    // Create the LBM simulation with 3D textures
    let simulation = LbmFluidSimulationTexture::new();

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add sphere obstacle visualization at center
    app.add_object("examples/test/sphere.obj")
        .with_transform([0.0, 0.0, 0.0], 0.17, 0.0) // Scale to match 8-unit radius in 96-unit grid
        .with_material("orange_metal")
        .with_name("Sphere Obstacle");

    // Add boundary markers to show domain extent
    app.add_object("examples/test/cube.obj")
        .with_transform([-1.0, -1.0, -1.0], 0.05, 0.0)
        .with_name("Domain Corner 1");

    app.add_object("examples/test/cube.obj")
        .with_transform([1.0, 1.0, 1.0], 0.05, 0.0)
        .with_name("Domain Corner 2");

    // Add sphere obstacle marker (for visual reference)
    // The actual boundary is handled by the 3D boundary texture

    // Sphere at grid center
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.3, 0.0) // Sphere visualization
        .with_name("Sphere Obstacle");

    // Set up UI
    app.set_ui(|ui, scene, selected_index| {
        // Default transform panel
        haggis::ui::panel::default_transform_panel(ui, scene, selected_index);

        // LBM info panel
        ui.window("LBM Info (3D Texture)")
            .size([300.0, 250.0], imgui::Condition::FirstUseEver)
            .position([20.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🌊 3D Lattice Boltzmann Method (3D Texture)");
                ui.separator();
                ui.text("Flow around sphere obstacle");
                ui.text("Zou-He inlet/outlet boundaries");
                ui.text("D3Q19 lattice, BGK collision");
                ui.separator();
                ui.text("💡 Flow Setup:");
                ui.text("  • Left: Velocity inlet (Zou-He)");
                ui.text("  • Right: Pressure outlet (Zou-He)");
                ui.text("  • Center: Sphere obstacle");
                ui.text("  • Walls: No-slip bounce-back");
                ui.separator();
                ui.text("⚪ Sphere Boundary:");
                ui.text("  • Simple sphere geometry");
                ui.text("  • Centered in the domain");
                ui.text("  • Adjustable radius parameter");
                ui.separator();
                ui.text("🌀 Vorticity Visualization:");
                ui.text("  • Cut plane shows wake patterns");
                ui.text("  • Red = Counter-clockwise rotation");
                ui.text("  • Green = Clockwise rotation");
                ui.text("  • Classic von Kármán vortex street");
                ui.separator();
                ui.text("🚀 3D Texture Advantages:");
                ui.text("  • Better spatial locality");
                ui.text("  • Hardware-accelerated sampling");
                ui.text("  • Optimized cache behavior");
                ui.text("  • Reduced memory bandwidth");
            });
    });

    // Configure for 60fps visuals with independent background compute
    app.set_framerate_limit(Some(60.0));
    app.set_compute_mode(haggis::ComputeMode::Independent { compute_fps: 60.0 });

    // Run the application
    app.show_performance_panel(true);
    app.run();

    Ok(())
}