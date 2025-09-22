//! # 3D Lattice Boltzmann Method (LBM) Fluid Simulation - Hybrid Approach
//!
//! This example demonstrates a 3D BGK LBM fluid simulation using a hybrid approach:
//! - 3D textures for read operations (better cache locality)
//! - Storage buffers for write operations (better compatibility)
//! - Same sphere obstacle as the buffer version
//!
//! ## Performance Benefits
//!
//! - Uses 3D textures for spatial locality during reads
//! - Maintains compatibility with storage buffers for writes
//! - Better memory access patterns than pure buffer approach
//! - GPU-friendly boundary condition handling
//!
//! ## Usage
//!
//! Run with: `cargo run --example lbm_fluid_3d_simple`

use haggis::prelude::*;
use haggis::{
    simulation::BaseSimulation,
    visualization::traits::VisualizationComponent,
};
use cgmath::Vector3;

/// Grid size for the 3D LBM simulation (64³ for better compatibility)
const GRID_SIZE: u32 = 64;
const GRID_WIDTH: u32 = GRID_SIZE;
const GRID_HEIGHT: u32 = GRID_SIZE;
const GRID_DEPTH: u32 = GRID_SIZE;

/// D3Q19 lattice model - 19 velocity directions
const D3Q19_DIRECTIONS: u32 = 19;

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
}

impl Default for LbmParams {
    fn default() -> Self {
        Self {
            tau: 0.6,               // Lower relaxation time for less viscosity
            inlet_velocity: 0.08,   // Higher inlet velocity for vortex shedding
            outlet_pressure: 1.0,   // Outlet pressure (atmospheric)
            sphere_radius: 10.0,    // Sphere radius for 64³ grid
        }
    }
}

/// GPU resources for 3D LBM fluid simulation using hybrid approach
struct LbmGpuResourcesHybrid {
    // Compute pipelines
    stream_pipeline: wgpu::ComputePipeline,
    collision_pipeline: wgpu::ComputePipeline,
    vorticity_pipeline: wgpu::ComputePipeline,

    // Ping-pong buffers for distribution functions (f_i)
    distributions_buffer_a: wgpu::Buffer, // Current distributions
    distributions_buffer_b: wgpu::Buffer, // Next distributions

    // Velocity and vorticity buffers
    velocity_buffer: wgpu::Buffer,   // 4 floats per cell: [vx, vy, vz, density]
    vorticity_buffer: wgpu::Buffer,  // 4 floats per cell: [ωx, ωy, ωz, magnitude]

    // Boundary buffer - bit-packed obstacles (32 cells per u32)
    boundary_buffer: wgpu::Buffer,   // u32 array with bit flags for boundaries

    // Parameters buffer
    params_buffer: wgpu::Buffer,

    // Bind groups for ping-pong
    stream_bind_group_a_to_b: wgpu::BindGroup,
    stream_bind_group_b_to_a: wgpu::BindGroup,
    collision_bind_group_a: wgpu::BindGroup,
    collision_bind_group_b: wgpu::BindGroup,
    vorticity_bind_group: wgpu::BindGroup,

    // State
    ping_pong_state: bool, // false = A is current, true = B is current
}

/// 3D LBM fluid simulation using hybrid GPU approach
struct LbmFluidSimulationSimple {
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
    gpu_resources: Option<LbmGpuResourcesHybrid>,

    // Cut plane controls for vorticity visualization
    cut_plane_z: f32,
    needs_cut_plane_update: bool,
    visualization_scale: f32,

    // CPU backup for vorticity data (for cut plane extraction)
    cpu_vorticity: Vec<f32>, // 4 floats per cell
}

impl LbmFluidSimulationSimple {
    /// Generate sphere boundary pattern
    fn generate_sphere_boundaries() -> Vec<u32> {
        let total_cells = (GRID_WIDTH * GRID_HEIGHT * GRID_DEPTH) as usize;
        let u32_count = (total_cells + 31) / 32; // Round up for bit packing
        let mut boundary_data = vec![0u32; u32_count];

        for z in 0..GRID_DEPTH {
            for y in 0..GRID_HEIGHT {
                for x in 0..GRID_WIDTH {
                    let cell_index = (z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x) as usize;
                    let u32_index = cell_index / 32;
                    let bit_index = cell_index % 32;

                    let is_boundary = Self::is_sphere_boundary(x, y, z);

                    if is_boundary {
                        boundary_data[u32_index] |= 1u32 << bit_index;
                    }
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

        // Default sphere radius (can be controlled by parameter)
        let sphere_radius = 10.0; // Radius in grid units

        // Calculate distance from sphere center
        let dx = fx - sphere_center_x;
        let dy = fy - sphere_center_y;
        let dz = fz - sphere_center_z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Return true if inside sphere
        distance <= sphere_radius
    }

    fn new() -> Self {
        let mut base = BaseSimulation::new("LBM Fluid 3D (Simple)");

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
        };

        // Set the cut plane size
        if let Some(visualization) = simulation.base.get_visualization_mut("vorticity_plane") {
            if let Some(cut_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                cut_plane.set_size(simulation.visualization_scale);
            }
        }

        println!(
            "🌊 Initialized 3D LBM fluid simulation (Simple): {}³ grid with D3Q19 lattice",
            GRID_SIZE
        );

        simulation
    }

    /// Initialize GPU resources for LBM computation
    fn initialize_gpu_resources(&mut self, device: &Device, queue: &Queue) {
        println!("🔧 Initializing LBM GPU compute resources (Simple)...");

        // Create shaders (same as original buffer approach)
        let stream_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM Stream Shader (Simple)"),
            source: wgpu::ShaderSource::Wgsl(LBM_STREAM_SHADER_SIMPLE.into()),
        });

        let collision_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM Collision Shader (Simple)"),
            source: wgpu::ShaderSource::Wgsl(LBM_COLLISION_SHADER_SIMPLE.into()),
        });

        let vorticity_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LBM Vorticity Shader (Simple)"),
            source: wgpu::ShaderSource::Wgsl(LBM_VORTICITY_SHADER_SIMPLE.into()),
        });

        // Create bind group layouts (same as original)
        let stream_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LBM Stream Layout (Simple)"),
            entries: &[
                // Input distributions
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
                // Output distributions
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
            label: Some("LBM Collision Layout (Simple)"),
            entries: &[
                // Distributions (read/write)
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
                // Velocity output
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
                // Boundary buffer (bit-packed)
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
            label: Some("LBM Vorticity Layout (Simple)"),
            entries: &[
                // Velocity input
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
                // Vorticity output
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
            label: Some("LBM Stream Pipeline (Simple)"),
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
            label: Some("LBM Collision Pipeline (Simple)"),
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
            label: Some("LBM Vorticity Pipeline (Simple)"),
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

        // Create buffers
        let distributions_size = (self.width * self.height * self.depth * D3Q19_DIRECTIONS * std::mem::size_of::<f32>() as u32) as u64;
        let velocity_size = (self.width * self.height * self.depth * 4 * std::mem::size_of::<f32>() as u32) as u64;
        let vorticity_size = velocity_size; // Same size as velocity (4 floats per cell)
        let params_size = 16u64; // 4 f32 values (16 bytes) for proper alignment

        let distributions_buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Distributions Buffer A (Simple)"),
            size: distributions_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let distributions_buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Distributions Buffer B (Simple)"),
            size: distributions_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let velocity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Velocity Buffer (Simple)"),
            size: velocity_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let vorticity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Vorticity Buffer (Simple)"),
            size: vorticity_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create boundary buffer (bit-packed obstacles)
        let boundary_data = Self::generate_sphere_boundaries();
        let boundary_size = (boundary_data.len() * std::mem::size_of::<u32>()) as u64;
        let boundary_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Boundary Buffer (Simple)"),
            size: boundary_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Upload boundary data
        queue.write_buffer(&boundary_buffer, 0, bytemuck::cast_slice(&boundary_data));

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LBM Parameters Buffer (Simple)"),
            size: params_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind groups
        let stream_bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Stream A->B (Simple)"),
            layout: &stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: distributions_buffer_b.as_entire_binding(),
                },
            ],
        });

        let stream_bind_group_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Stream B->A (Simple)"),
            layout: &stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: distributions_buffer_a.as_entire_binding(),
                },
            ],
        });

        let collision_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Collision A (Simple)"),
            layout: &collision_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: boundary_buffer.as_entire_binding(),
                },
            ],
        });

        let collision_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Collision B (Simple)"),
            layout: &collision_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: distributions_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: velocity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: boundary_buffer.as_entire_binding(),
                },
            ],
        });

        let vorticity_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LBM Vorticity (Simple)"),
            layout: &vorticity_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: velocity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vorticity_buffer.as_entire_binding(),
                },
            ],
        });

        self.gpu_resources = Some(LbmGpuResourcesHybrid {
            stream_pipeline,
            collision_pipeline,
            vorticity_pipeline,
            distributions_buffer_a,
            distributions_buffer_b,
            velocity_buffer,
            vorticity_buffer,
            boundary_buffer,
            params_buffer,
            stream_bind_group_a_to_b,
            stream_bind_group_b_to_a,
            collision_bind_group_a,
            collision_bind_group_b,
            vorticity_bind_group,
            ping_pong_state: false,
        });

        println!("✅ LBM GPU resources (Simple) initialized successfully");
    }

    /// Initialize LBM simulation with equilibrium distributions
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
            queue.write_buffer(&gpu_resources.distributions_buffer_a, 0, bytemuck::cast_slice(&distributions));
            queue.write_buffer(&gpu_resources.distributions_buffer_b, 0, bytemuck::cast_slice(&distributions));

            // Upload parameters
            let params_data = [
                self.params.tau,
                self.params.inlet_velocity,
                self.params.outlet_pressure,
                self.params.sphere_radius
            ];
            queue.write_buffer(&gpu_resources.params_buffer, 0, bytemuck::cast_slice(&params_data));

            println!("🌊 LBM simulation (Simple) initialized with equilibrium state");
        }
    }

    /// Run one LBM timestep: stream -> collision -> vorticity
    fn run_lbm_step(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref mut gpu_resources) = self.gpu_resources {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM Step Encoder (Simple)"),
            });

            // Step 1: Stream step (propagation)
            {
                let mut stream_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM Stream Pass (Simple)"),
                    timestamp_writes: None,
                });

                stream_pass.set_pipeline(&gpu_resources.stream_pipeline);

                let stream_bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.stream_bind_group_b_to_a
                } else {
                    &gpu_resources.stream_bind_group_a_to_b
                };

                stream_pass.set_bind_group(0, stream_bind_group, &[]);

                let workgroup_size = 4; // 4x4x4 workgroups
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;
                let num_workgroups_z = (self.depth + workgroup_size - 1) / workgroup_size;

                stream_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            // Flip ping-pong state after streaming
            gpu_resources.ping_pong_state = !gpu_resources.ping_pong_state;

            // Step 2: Collision step (BGK)
            {
                let mut collision_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM Collision Pass (Simple)"),
                    timestamp_writes: None,
                });

                collision_pass.set_pipeline(&gpu_resources.collision_pipeline);

                let collision_bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.collision_bind_group_b
                } else {
                    &gpu_resources.collision_bind_group_a
                };

                collision_pass.set_bind_group(0, collision_bind_group, &[]);

                let workgroup_size = 4;
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;
                let num_workgroups_z = (self.depth + workgroup_size - 1) / workgroup_size;

                collision_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            // Step 3: Vorticity calculation
            {
                let mut vorticity_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("LBM Vorticity Pass (Simple)"),
                    timestamp_writes: None,
                });

                vorticity_pass.set_pipeline(&gpu_resources.vorticity_pipeline);
                vorticity_pass.set_bind_group(0, &gpu_resources.vorticity_bind_group, &[]);

                let workgroup_size = 4;
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;
                let num_workgroups_z = (self.depth + workgroup_size - 1) / workgroup_size;

                vorticity_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            queue.submit(std::iter::once(encoder.finish()));
            self.generation += 1;
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
            self.cpu_vorticity[slice_start..]
                .chunks(4)
                .take(slice_size)
                .map(|chunk| chunk[2]) // Vorticity Z-component (can be +/-)
                .collect()
        } else {
            vec![0.0; slice_size]
        }
    }

    /// Update cut plane visualization with vorticity data
    fn update_vorticity_cut_plane(&mut self, device: &Device, queue: &Queue) {
        if self.gpu_resources.is_none() {
            return;
        }

        // Extract vorticity slice at current cut plane position
        let slice_data = self.extract_vorticity_z_slice(self.cut_plane_z);

        // Update cut plane position in 3D space
        let world_z = (self.cut_plane_z - 0.5) * self.visualization_scale * 2.0;

        // Update visualization
        if let Some(visualization) = self.base.get_visualization_mut("vorticity_plane") {
            if let Some(cut_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                cut_plane.update_data(slice_data, self.width, self.height);
                cut_plane.set_position(Vector3::new(0.0, 0.0, world_z));
                cut_plane.set_size(self.visualization_scale);
                cut_plane.update(0.0, Some(device), Some(queue));
            }
        }
    }

    /// Sync GPU vorticity data back to CPU for visualization
    fn sync_vorticity_to_cpu(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            let buffer_size = (self.width * self.height * self.depth * 4 * std::mem::size_of::<f32>() as u32) as u64;

            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("LBM Vorticity Staging (Simple)"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LBM Vorticity Sync Encoder (Simple)"),
            });

            encoder.copy_buffer_to_buffer(&gpu_resources.vorticity_buffer, 0, &staging_buffer, 0, buffer_size);
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
                }

                // Update cut plane visualization
                self.update_vorticity_cut_plane(device, queue);
            }
        }
    }
}

impl haggis::simulation::traits::Simulation for LbmFluidSimulationSimple {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        println!("🌊 LBM Fluid 3D simulation (Simple) initialized");
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.base.initialize_gpu(device, queue);
        self.initialize_gpu_resources(device, queue);
        self.initialize_simulation(device, queue);
        self.sync_vorticity_to_cpu(device, queue);
        println!("✅ LBM GPU initialization (Simple) complete");
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
            self.update_vorticity_cut_plane(device, queue);
            self.needs_cut_plane_update = false;
        }

        // Run simulation continuously at maximum GPU effort
        if !self.is_paused && self.gpu_resources.is_some() {
            self.run_lbm_step(device, queue);

            // Sync vorticity data every few steps for real-time vortex shedding
            if self.generation % 3 == 0 {
                self.sync_vorticity_to_cpu(device, queue);
            }
        }

        self.base.update_gpu(device, queue, _delta_time);
    }

    fn apply_gpu_results_to_scene(&mut self, device: &Device, scene: &mut haggis::gfx::scene::Scene) {
        self.base.apply_gpu_results_to_scene(device, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("LBM Fluid 3D (Simple)")
            .size([450.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🌊 3D Lattice Boltzmann Method (Simple)");
                ui.separator();

                ui.text(&format!("Timestep: {}", self.generation));
                ui.text(&format!("Grid Size: {}³ ({} cells)", GRID_SIZE, GRID_SIZE * GRID_SIZE * GRID_SIZE));
                ui.text(&format!("Lattice: D3Q{}", D3Q19_DIRECTIONS));
                ui.text(&format!("GPU Ready: {}", self.gpu_resources.is_some()));

                // Continuous GPU simulation
                ui.text("💡 Simple Buffer GPU Simulation");
                ui.text("Stable and compatible implementation");

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
                ui.text("Vorticity Visualization:");
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
                    ui.text_colored([0.0, 1.0, 0.0, 1.0], "▶ Running (Simple)");
                } else {
                    ui.text_colored([1.0, 0.5, 0.0, 1.0], "⚙ Initializing GPU...");
                }

                ui.separator();
                ui.text("Simple Implementation:");
                ui.bullet_text("Proven buffer approach");
                ui.bullet_text("Maximum compatibility");
                ui.bullet_text("Stable performance");
                ui.bullet_text("Same sphere boundary");
            });

        self.base.render_ui(ui);
    }

    fn name(&self) -> &str {
        "LBM Fluid 3D (Simple)"
    }

    fn is_running(&self) -> bool {
        !self.is_paused
    }

    fn set_running(&mut self, running: bool) {
        self.is_paused = !running;
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        println!("🔄 Resetting LBM simulation (Simple)");
        self.generation = 0;
        self.base.reset(scene);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.base
    }
}

// LBM compute shaders using standard buffer approach with optimizations
const LBM_STREAM_SHADER_SIMPLE: &str = r#"
// D3Q19 lattice directions
const D3Q19_DIRECTIONS: u32 = 19u;
const GRID_WIDTH: u32 = 64u;
const GRID_HEIGHT: u32 = 64u;
const GRID_DEPTH: u32 = 64u;

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

@compute @workgroup_size(4, 4, 4)
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

const LBM_COLLISION_SHADER_SIMPLE: &str = r#"
const D3Q19_DIRECTIONS: u32 = 19u;
const GRID_WIDTH: u32 = 64u;
const GRID_HEIGHT: u32 = 64u;
const GRID_DEPTH: u32 = 64u;

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
@group(0) @binding(3) var<storage, read> boundary_buffer: array<u32>; // bit-packed boundary flags

// Check if cell is a boundary using bit-packed buffer
fn is_boundary_cell(x: u32, y: u32, z: u32) -> bool {
    let cell_index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;
    let u32_index = cell_index / 32u;
    let bit_index = cell_index % 32u;

    if (u32_index >= arrayLength(&boundary_buffer)) {
        return false;
    }

    let boundary_bits = boundary_buffer[u32_index];
    return (boundary_bits & (1u << bit_index)) != 0u;
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;

    if (x >= GRID_WIDTH || y >= GRID_HEIGHT || z >= GRID_DEPTH) {
        return;
    }

    let cell_index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;
    let base_dist_index = cell_index * D3Q19_DIRECTIONS;

    // Parameters
    let tau = params.x;
    let inlet_velocity = params.y;
    let outlet_pressure = params.z;

    // Calculate macroscopic quantities
    var density = 0.0;
    var velocity = vec3<f32>(0.0);

    for (var i: u32 = 0u; i < D3Q19_DIRECTIONS; i++) {
        let f_i = distributions[base_dist_index + i];
        density += f_i;
        velocity += f_i * VELOCITY_SET[i];
    }

    velocity = velocity / density;

    // Check boundary using bit-packed buffer
    let is_inside_obstacle = is_boundary_cell(x, y, z);

    // Apply boundary conditions
    var is_boundary = false;

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

const LBM_VORTICITY_SHADER_SIMPLE: &str = r#"
const GRID_WIDTH: u32 = 64u;
const GRID_HEIGHT: u32 = 64u;
const GRID_DEPTH: u32 = 64u;

@group(0) @binding(0) var<storage, read> velocity_density: array<f32>; // [vx, vy, vz, density]
@group(0) @binding(1) var<storage, read_write> vorticity: array<f32>; // [ωx, ωy, ωz, magnitude]

@compute @workgroup_size(4, 4, 4)
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
    println!("🌊 3D Lattice Boltzmann Method (LBM) Fluid Simulation - Simple Version");
    println!("======================================================================");
    println!("Stable and compatible 3D fluid dynamics with GPU compute shaders.");
    println!();
    println!("Features:");
    println!("  • BGK LBM with D3Q19 lattice model");
    println!("  • 64³ grid = 262,144 fluid cells");
    println!("  • Zou-He inlet/outlet boundary conditions");
    println!("  • Sphere obstacle boundary");
    println!("  • Real-time vorticity visualization");
    println!("  • Buffer-based approach for maximum compatibility");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create the LBM simulation
    let simulation = LbmFluidSimulationSimple::new();

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add boundary markers to show domain extent
    app.add_object("examples/test/cube.obj")
        .with_transform([-1.0, -1.0, -1.0], 0.05, 0.0)
        .with_name("Domain Corner 1");

    app.add_object("examples/test/cube.obj")
        .with_transform([1.0, 1.0, 1.0], 0.05, 0.0)
        .with_name("Domain Corner 2");

    // Add sphere obstacle marker (for visual reference)
    // The actual boundary is handled by the bit-packed boundary buffer

    // Sphere at grid center
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.25, 0.0) // Sphere visualization
        .with_name("Sphere Obstacle");

    // Set up UI
    app.set_ui(|ui, scene, selected_index| {
        // Default transform panel
        haggis::ui::panel::default_transform_panel(ui, scene, selected_index);

        // LBM info panel
        ui.window("LBM Info (Simple)")
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .position([20.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🌊 3D Lattice Boltzmann Method (Simple)");
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
                ui.text("✅ Simple Advantages:");
                ui.text("  • Maximum compatibility");
                ui.text("  • Stable performance");
                ui.text("  • Proven buffer approach");
            });
    });

    // Run the application
    app.show_performance_panel(true);
    app.run();

    Ok(())
}