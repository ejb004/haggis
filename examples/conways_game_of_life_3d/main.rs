//! # Conway's Game of Life - 3D GPU Implementation
//!
//! This example demonstrates Conway's Game of Life extended to 3D using GPU compute shaders
//! with proper ping-pong buffering for high-performance 3D cellular automaton simulation.
//!
//! ## Features Demonstrated
//!
//! - GPU-accelerated 3D Conway's Game of Life with compute shaders
//! - 3D grid simulation with 26-neighbor rule (3x3x3 neighborhood)
//! - Ping-pong buffer system for efficient GPU computation
//! - Real-time 2D cut plane visualization that slices through 3D data
//! - Interactive cut plane position controls (move through Z-axis)
//! - High-performance simulation of 64³ grids
//!
//! ## 3D Conway's Rules (26-neighbor version)
//!
//! 1. Live cell with 4-7 neighbors survives (balanced for 3D)
//! 2. Dead cell with exactly 6 neighbors becomes alive
//! 3. All other cells die or stay dead
//!
//! ## Usage
//!
//! Run with: `cargo run --example conways_game_of_life_3d`

use haggis::prelude::*;
use haggis::{
    simulation::BaseSimulation,
    visualization::traits::VisualizationComponent,
};

/// Grid size for the 3D Game of Life (64³ = 262,144 cells)
const GRID_SIZE: u32 = 64;
const GRID_WIDTH: u32 = GRID_SIZE;
const GRID_HEIGHT: u32 = GRID_SIZE;
const GRID_DEPTH: u32 = GRID_SIZE;

/// 3D Life patterns
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Life3DPattern {
    Random,
    Glider3D,
    Block3D,
    Oscillator3D,
    Clear,
}

impl Life3DPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            Life3DPattern::Random => "Random",
            Life3DPattern::Glider3D => "3D Glider", 
            Life3DPattern::Block3D => "3D Block",
            Life3DPattern::Oscillator3D => "3D Oscillator",
            Life3DPattern::Clear => "Clear",
        }
    }
}

/// GPU resources for 3D Conway's Game of Life compute shader
struct Gpu3DGameOfLifeResources {
    compute_pipeline: wgpu::ComputePipeline,
    #[allow(dead_code)]
    bind_group_layout: wgpu::BindGroupLayout,
    // Ping-pong buffers for 3D data
    buffer_a: wgpu::Buffer, // Current state
    buffer_b: wgpu::Buffer, // Next state
    // Bind groups for ping-pong
    bind_group_a_to_b: wgpu::BindGroup, // Read from A, write to B
    bind_group_b_to_a: wgpu::BindGroup, // Read from B, write to A
    // State
    ping_pong_state: bool, // false = A is current, true = B is current
}

/// 3D Conway's Game of Life simulation using GPU compute shaders
struct Conways3DGpuSimulation {
    base: BaseSimulation,
    // 3D Grid state
    width: u32,
    height: u32,
    depth: u32,
    generation: u64,
    current_pattern: Life3DPattern,
    // Control
    last_update: Instant,
    speed: f32,
    is_paused: bool,
    // GPU resources
    gpu_resources: Option<Gpu3DGameOfLifeResources>,
    // CPU backup for pattern initialization
    cpu_grid: Vec<bool>,
    // Update flags
    needs_gpu_upload: bool,
    needs_manual_step: bool,
    // Cut plane controls
    cut_plane_z: f32, // Z position of the cut plane (0.0 to 1.0)
    needs_cut_plane_update: bool, // Flag to update cut plane visualization
}

impl Conways3DGpuSimulation {
    fn new() -> Self {
        let mut base = BaseSimulation::new("Conway's Game of Life 3D");

        // Create and configure the cut plane visualization
        let mut cut_plane = CutPlane2D::new();
        cut_plane.set_position(Vector3::new(0.0, 0.0, 0.0)); // Start at center
        cut_plane.set_size(2.0); // 2x2x2 world bounds

        // Initialize with empty data for now
        let empty_data = vec![0.0; (GRID_WIDTH * GRID_HEIGHT) as usize];
        cut_plane.update_data(empty_data, GRID_WIDTH, GRID_HEIGHT);

        // Add visualization to base
        base.add_visualization("cut_plane", cut_plane);

        let mut simulation = Self {
            base,
            width: GRID_WIDTH,
            height: GRID_HEIGHT,
            depth: GRID_DEPTH,
            generation: 0,
            current_pattern: Life3DPattern::Random,
            last_update: Instant::now(),
            speed: 5.0, // Slower for 3D complexity
            is_paused: false,
            gpu_resources: None,
            cpu_grid: vec![false; (GRID_WIDTH * GRID_HEIGHT * GRID_DEPTH) as usize],
            needs_gpu_upload: true,
            needs_manual_step: false,
            cut_plane_z: 0.5, // Start at middle Z slice
            needs_cut_plane_update: true, // Update visualization on startup
        };

        // Initialize with random pattern
        simulation.initialize_pattern(Life3DPattern::Random);

        // Log initial pattern
        let live_count = simulation.cpu_grid.iter().filter(|&&cell| cell).count();
        println!(
            "🔬 Initial 3D {} pattern: {} live cells in {}³ grid",
            simulation.current_pattern.as_str(),
            live_count,
            GRID_SIZE
        );

        simulation
    }

    /// Initialize the 3D grid with a specific pattern
    fn initialize_pattern(&mut self, pattern: Life3DPattern) {
        self.current_pattern = pattern;
        self.generation = 0;

        // Clear the CPU grid first
        self.cpu_grid.fill(false);

        match pattern {
            Life3DPattern::Random => self.initialize_random_3d(),
            Life3DPattern::Glider3D => self.initialize_3d_glider(),
            Life3DPattern::Block3D => self.initialize_3d_block(),
            Life3DPattern::Oscillator3D => self.initialize_3d_oscillator(),
            Life3DPattern::Clear => {} // Already cleared
        }

        // Mark that we need to upload new data to GPU
        self.needs_gpu_upload = true;
    }

    /// Initialize with random 3D pattern (15% alive - lower density for 3D)
    fn initialize_random_3d(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        for cell in self.cpu_grid.iter_mut() {
            *cell = rng.random_bool(0.15); // Lower density for 3D
        }
    }

    /// Initialize with a simple 3D glider-like pattern
    fn initialize_3d_glider(&mut self) {
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let center_z = self.depth / 2;

        // Simple 3D glider pattern (extending 2D glider into 3D)
        let glider_3d_coords = [
            // Base layer (z=0)
            (1, 0, 0), (2, 1, 0), (0, 2, 0), (1, 2, 0), (2, 2, 0),
            // Middle layer (z=1) 
            (1, 1, 1), (2, 1, 1), (1, 2, 1),
            // Top layer (z=2)
            (1, 1, 2),
        ];

        for (dx, dy, dz) in glider_3d_coords.iter() {
            let x = center_x + dx - 1;
            let y = center_y + dy - 1;
            let z = center_z + dz - 1;
            if x < self.width && y < self.height && z < self.depth {
                let index = ((z * self.height + y) * self.width + x) as usize;
                if index < self.cpu_grid.len() {
                    self.cpu_grid[index] = true;
                }
            }
        }
    }

    /// Initialize with a 3D block pattern
    fn initialize_3d_block(&mut self) {
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let center_z = self.depth / 2;

        // 2x2x2 cube
        for dx in 0..2 {
            for dy in 0..2 {
                for dz in 0..2 {
                    let x = center_x + dx - 1;
                    let y = center_y + dy - 1;
                    let z = center_z + dz - 1;
                    if x < self.width && y < self.height && z < self.depth {
                        let index = ((z * self.height + y) * self.width + x) as usize;
                        if index < self.cpu_grid.len() {
                            self.cpu_grid[index] = true;
                        }
                    }
                }
            }
        }
    }

    /// Initialize with a 3D oscillator pattern
    fn initialize_3d_oscillator(&mut self) {
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        let center_z = self.depth / 2;

        // 3D cross pattern
        let oscillator_coords = [
            // X-axis line
            (-1, 0, 0), (0, 0, 0), (1, 0, 0),
            // Y-axis line
            (0, -1, 0), (0, 1, 0),
            // Z-axis line
            (0, 0, -1), (0, 0, 1),
        ];

        for (dx, dy, dz) in oscillator_coords.iter() {
            let x = (center_x as i32 + dx) as u32;
            let y = (center_y as i32 + dy) as u32;
            let z = (center_z as i32 + dz) as u32;
            if x < self.width && y < self.height && z < self.depth {
                let index = ((z * self.height + y) * self.width + x) as usize;
                if index < self.cpu_grid.len() {
                    self.cpu_grid[index] = true;
                }
            }
        }
    }

    /// Extract 2D slice from 3D grid at specified Z position
    fn extract_z_slice(&self, z_normalized: f32) -> Vec<f32> {
        let z_index = ((z_normalized * (self.depth - 1) as f32).round() as u32).min(self.depth - 1);
        let slice_start = (z_index * self.height * self.width) as usize;
        let slice_size = (self.height * self.width) as usize;
        
        if slice_start + slice_size <= self.cpu_grid.len() {
            self.cpu_grid[slice_start..slice_start + slice_size]
                .iter()
                .map(|&b| if b { 1.0 } else { 0.0 })
                .collect()
        } else {
            vec![0.0; slice_size]
        }
    }

    /// Initialize GPU resources for 3D computation
    fn initialize_gpu_resources(&mut self, device: &Device) {
        // 3D Conway's Game of Life compute shader
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Conway's 3D Game of Life Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(CONWAY_3D_COMPUTE_SHADER.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Conway 3D Bind Group Layout"),
            entries: &[
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

        // Create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Conway 3D Compute Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Conway 3D Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &compute_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        // Create ping-pong buffers for 3D data
        let buffer_size = (self.width * self.height * self.depth * std::mem::size_of::<u32>() as u32) as u64;

        let buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Conway 3D Buffer A"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Conway 3D Buffer B"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create bind groups for ping-pong
        let bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Conway 3D Bind Group A->B"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_b.as_entire_binding(),
                },
            ],
        });

        let bind_group_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Conway 3D Bind Group B->A"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_a.as_entire_binding(),
                },
            ],
        });

        self.gpu_resources = Some(Gpu3DGameOfLifeResources {
            compute_pipeline,
            bind_group_layout,
            buffer_a,
            buffer_b,
            bind_group_a_to_b,
            bind_group_b_to_a,
            ping_pong_state: false, // Start with A as current
        });
    }

    /// Upload CPU 3D grid data to GPU buffer
    fn upload_grid_to_gpu(&self, _device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            // Convert bool grid to u32 grid
            let u32_data: Vec<u32> = self
                .cpu_grid
                .iter()
                .map(|&b| if b { 1u32 } else { 0u32 })
                .collect();

            // Log GPU uploads for verification
            if self.generation == 0 {
                let live_count = u32_data.iter().filter(|&&val| val > 0).count();
                println!(
                    "📡 Uploading initial 3D pattern to GPU: {} live cells in {}³ grid",
                    live_count,
                    GRID_SIZE
                );
            }

            // Upload to the current buffer
            let current_buffer = if gpu_resources.ping_pong_state {
                &gpu_resources.buffer_b
            } else {
                &gpu_resources.buffer_a
            };

            queue.write_buffer(current_buffer, 0, bytemuck::cast_slice(&u32_data));
        }
    }

    /// Update cut plane visualization with current Z slice
    fn update_cut_plane_visualization(&mut self, device: &Device, queue: &Queue) {
        if self.gpu_resources.is_none() {
            return;
        }

        // Extract Z slice at current cut plane position
        let slice_data = self.extract_z_slice(self.cut_plane_z);

        // Update cut plane position in 3D space (map 0.0-1.0 to -1.0 to +1.0 in world coords)
        let world_z = (self.cut_plane_z - 0.5) * 2.0;

        // Update visualization
        if let Some(visualization) = self.base.get_visualization_mut("cut_plane") {
            if let Some(cut_plane) = visualization.as_any_mut().downcast_mut::<CutPlane2D>() {
                cut_plane.update_data(slice_data, self.width, self.height);
                cut_plane.set_position(Vector3::new(0.0, 0.0, world_z));
                cut_plane.update(0.0, Some(device), Some(queue));
            }
        }
    }

    /// Run one GPU compute step for 3D simulation
    fn run_gpu_compute_step(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref mut gpu_resources) = self.gpu_resources {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Conway 3D Compute Encoder"),
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Conway 3D Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&gpu_resources.compute_pipeline);

                // Use appropriate bind group based on ping-pong state
                let bind_group = if gpu_resources.ping_pong_state {
                    &gpu_resources.bind_group_b_to_a
                } else {
                    &gpu_resources.bind_group_a_to_b
                };

                compute_pass.set_bind_group(0, bind_group, &[]);

                // Dispatch compute shader for 3D grid
                let workgroup_size = 4; // 4x4x4 workgroups for 3D (64 invocations < 256 limit)
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;
                let num_workgroups_z = (self.depth + workgroup_size - 1) / workgroup_size;

                compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, num_workgroups_z);
            }

            queue.submit(std::iter::once(encoder.finish()));

            // Flip ping-pong state
            gpu_resources.ping_pong_state = !gpu_resources.ping_pong_state;
            self.generation += 1;
        }
    }

    /// Sync GPU results back to CPU and update visualization  
    fn sync_gpu_to_cpu(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            // Create staging buffer to read GPU data
            let buffer_size = (self.width * self.height * self.depth * std::mem::size_of::<u32>() as u32) as u64;
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Conway 3D Staging Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            // Copy from current GPU buffer to staging buffer
            let current_buffer = if gpu_resources.ping_pong_state {
                &gpu_resources.buffer_b
            } else {
                &gpu_resources.buffer_a
            };

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Conway 3D Sync Encoder"),
            });

            encoder.copy_buffer_to_buffer(current_buffer, 0, &staging_buffer, 0, buffer_size);
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
                let u32_data: &[u32] = bytemuck::cast_slice(&data);

                // Update CPU grid with GPU results
                for (i, &gpu_val) in u32_data.iter().enumerate() {
                    if i < self.cpu_grid.len() {
                        self.cpu_grid[i] = gpu_val > 0;
                    }
                }

                // Optional: Log generation progress
                if self.generation % 20 == 0 && self.generation > 0 {
                    let live_count = u32_data.iter().filter(|&&val| val > 0).count();
                    println!(
                        "🔬 3D Generation {}: {} live cells",
                        self.generation, live_count
                    );
                }

                // Update cut plane visualization with current slice
                self.update_cut_plane_visualization(device, queue);
            }
        }
    }
}

impl haggis::simulation::traits::Simulation for Conways3DGpuSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        println!("🚀 Conway's 3D Game of Life GPU simulation initialized");
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.base.initialize_gpu(device, queue);

        // Initialize GPU compute resources
        self.initialize_gpu_resources(device);

        // Upload initial pattern to GPU
        self.upload_grid_to_gpu(device, queue);

        // Sync initial data and update visualization
        self.sync_gpu_to_cpu(device, queue);

        println!("🔧 3D GPU compute resources initialized");
        println!("✅ Initial 3D GPU data uploaded and visualization synced");
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        self.base.update(delta_time, scene);
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, _delta_time: f32) {
        // Check if we need to reupload data to GPU (after pattern change)
        if self.needs_gpu_upload && self.gpu_resources.is_some() {
            println!("🔄 Switching to 3D {} pattern", self.current_pattern.as_str());
            self.upload_grid_to_gpu(device, queue);
            self.sync_gpu_to_cpu(device, queue);
            self.needs_gpu_upload = false;
        }

        // Handle manual step request
        if self.needs_manual_step && self.gpu_resources.is_some() {
            self.run_gpu_compute_step(device, queue);
            self.sync_gpu_to_cpu(device, queue);
            self.needs_manual_step = false;
        }

        // Handle cut plane updates (works even when paused!)
        if self.needs_cut_plane_update && self.gpu_resources.is_some() {
            self.update_cut_plane_visualization(device, queue);
            self.needs_cut_plane_update = false;
        }

        // Auto-evolve based on speed setting (if not paused and GPU ready)
        if !self.is_paused && self.speed > 0.0 && self.gpu_resources.is_some() {
            let time_per_generation = 1.0 / self.speed;
            if self.last_update.elapsed().as_secs_f32() >= time_per_generation {
                self.run_gpu_compute_step(device, queue);
                self.sync_gpu_to_cpu(device, queue);
                self.last_update = Instant::now();
            }
        }

        self.base.update_gpu(device, queue, _delta_time);
    }

    fn apply_gpu_results_to_scene(
        &mut self,
        device: &Device,
        scene: &mut haggis::gfx::scene::Scene,
    ) {
        self.base.apply_gpu_results_to_scene(device, scene);
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        ui.window("Conway's 3D Game of Life GPU")
            .size([450.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🔬 Conway's 3D Game of Life GPU");
                ui.separator();

                ui.text(&format!("Generation: {}", self.generation));
                ui.text(&format!("Grid Size: {}³ ({} cells)", GRID_SIZE, GRID_SIZE * GRID_SIZE * GRID_SIZE));
                ui.text(&format!("GPU Ready: {}", self.gpu_resources.is_some()));

                ui.separator();

                // Play/Pause button
                if ui.button(if self.is_paused {
                    "▶ Play"
                } else {
                    "⏸ Pause"
                }) {
                    self.is_paused = !self.is_paused;
                }
                ui.same_line();

                // Manual step button
                if ui.button("⏭ Step") && self.gpu_resources.is_some() {
                    self.needs_manual_step = true;
                }

                ui.separator();

                // 3D Pattern selection
                ui.text("3D Pattern:");
                let patterns = [
                    Life3DPattern::Random,
                    Life3DPattern::Glider3D,
                    Life3DPattern::Block3D,
                    Life3DPattern::Oscillator3D,
                    Life3DPattern::Clear,
                ];
                for pattern in patterns.iter() {
                    if ui.radio_button_bool(pattern.as_str(), self.current_pattern == *pattern) {
                        self.initialize_pattern(*pattern);
                    }
                }

                ui.separator();

                // Cut plane controls
                ui.text("Cut Plane (Z-slice):");
                if ui
                    .slider_config("Z Position", 0.0, 1.0)
                    .display_format("%.2f")
                    .build(&mut self.cut_plane_z)
                {
                    // Mark that cut plane needs update when slider changes
                    self.needs_cut_plane_update = true;
                }

                // Show which Z layer we're viewing
                let z_layer = ((self.cut_plane_z * (GRID_DEPTH - 1) as f32).round() as u32).min(GRID_DEPTH - 1);
                ui.text(&format!("Viewing layer {}/{}", z_layer, GRID_DEPTH - 1));

                ui.separator();

                // Speed control
                ui.text("Simulation Speed:");
                let mut speed_value = self.speed;
                if ui
                    .slider_config("Generations/sec", 0.1, 20.0)
                    .display_format("%.1f gen/sec")
                    .build(&mut speed_value)
                {
                    self.speed = speed_value;
                }

                ui.separator();

                // Status
                ui.text("Status:");
                if self.is_paused {
                    ui.text_colored([1.0, 1.0, 0.0, 1.0], "⏸ Paused");
                } else if self.gpu_resources.is_some() {
                    ui.text_colored(
                        [0.0, 1.0, 0.0, 1.0],
                        &format!("▶ Running ({:.1} gen/sec)", self.speed),
                    );
                } else {
                    ui.text_colored([1.0, 0.5, 0.0, 1.0], "⚙ Initializing GPU...");
                }

                ui.separator();
                ui.text("3D GPU Features:");
                ui.bullet_text("26-neighbor 3D cellular automaton");
                ui.bullet_text("64³ grid = 262,144 cells");
                ui.bullet_text("Real-time Z-slice visualization");
                ui.bullet_text("GPU compute shaders");
            });

        // Render base simulation UI
        self.base.render_ui(ui);
    }

    fn name(&self) -> &str {
        "Conway's 3D Game of Life GPU"
    }

    fn is_running(&self) -> bool {
        !self.is_paused
    }

    fn set_running(&mut self, running: bool) {
        self.is_paused = !running;
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        println!("🐛 Debug: Resetting 3D simulation");
        self.initialize_pattern(self.current_pattern);
        self.base.reset(scene);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.base
    }
}

// 3D Conway's Game of Life compute shader with 26-neighbor rule
const CONWAY_3D_COMPUTE_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input_buffer: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_buffer: array<u32>;

// 3D Grid dimensions
const GRID_WIDTH: u32 = 64u;
const GRID_HEIGHT: u32 = 64u;
const GRID_DEPTH: u32 = 64u;

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;
    
    // Check bounds
    if (x >= GRID_WIDTH || y >= GRID_HEIGHT || z >= GRID_DEPTH) {
        return;
    }
    
    let index = z * GRID_HEIGHT * GRID_WIDTH + y * GRID_WIDTH + x;
    
    // Count live neighbors in 3x3x3 neighborhood (26 neighbors)
    var live_neighbors = 0u;
    
    for (var dz: i32 = -1; dz <= 1; dz++) {
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                if (dx == 0 && dy == 0 && dz == 0) {
                    continue; // Skip self
                }
                
                // Calculate neighbor position with wrapping
                let nx = (i32(x) + dx + i32(GRID_WIDTH)) % i32(GRID_WIDTH);
                let ny = (i32(y) + dy + i32(GRID_HEIGHT)) % i32(GRID_HEIGHT);
                let nz = (i32(z) + dz + i32(GRID_DEPTH)) % i32(GRID_DEPTH);
                let neighbor_index = u32(nz) * GRID_HEIGHT * GRID_WIDTH + u32(ny) * GRID_WIDTH + u32(nx);
                
                if (input_buffer[neighbor_index] == 1u) {
                    live_neighbors++;
                }
            }
        }
    }
    
    let current_cell = input_buffer[index];
    
    // Apply 3D Conway's rules (adjusted for 26 neighbors)
    var next_state = 0u;
    if (current_cell == 1u) {
        // Live cell: survives with 4-7 neighbors (balanced for 3D)
        if (live_neighbors >= 4u && live_neighbors <= 7u) {
            next_state = 1u; // Survives
        }
        // Otherwise dies
    } else {
        // Dead cell: becomes alive with exactly 6 neighbors
        if (live_neighbors == 6u) {
            next_state = 1u; // Becomes alive
        }
        // Otherwise stays dead
    }
    
    output_buffer[index] = next_state;
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Conway's 3D Game of Life - GPU Implementation");
    println!("===============================================");
    println!("High-performance 3D GPU compute shader implementation with cut plane visualization.");
    println!();
    println!("Features:");
    println!("  • 3D cellular automaton with 26-neighbor rule");
    println!("  • 64³ grid = 262,144 cells computed in parallel");
    println!("  • Real-time Z-slice visualization with moveable cut plane");
    println!("  • 2x2x2 world bounds with 64³ grid resolution");
    println!("  • GPU ping-pong buffers for optimal performance");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create the 3D GPU Conway's Game of Life simulation
    let simulation = Conways3DGpuSimulation::new();

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add reference objects for context (2x2x2 bounds)
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.1, 0.0)
        .with_name("Reference Cube at Origin");

    // Boundary markers for 2x2x2 world
    app.add_object("examples/test/cube.obj")
        .with_transform([-1.0, -1.0, -1.0], 0.05, 0.0)
        .with_name("Bound (-1,-1,-1)");
    
    app.add_object("examples/test/cube.obj")
        .with_transform([1.0, 1.0, 1.0], 0.05, 0.0)
        .with_name("Bound (1,1,1)");

    // Run the application
    app.show_performance_panel(true);
    app.run();

    Ok(())
}