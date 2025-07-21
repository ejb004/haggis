//! # Conway's Game of Life - GPU Implementation
//!
//! This example demonstrates Conway's Game of Life using GPU compute shaders
//! with proper ping-pong buffering for high-performance cellular automaton simulation.
//!
//! ## Features Demonstrated
//!
//! - GPU-accelerated Conway's Game of Life with compute shaders
//! - Ping-pong buffer system for efficient GPU computation
//! - Real-time 2D visualization of game state
//! - Interactive speed controls and pattern selection
//! - High-performance simulation of large grids
//!
//! ## Conway's Rules (implemented in GPU shader)
//!
//! 1. Live cell with 2-3 neighbors survives
//! 2. Dead cell with exactly 3 neighbors becomes alive  
//! 3. All other cells die or stay dead
//!
//! ## Usage
//!
//! Run with: `cargo run --example conways_game_of_life`

use cgmath::Vector3;
use haggis::{
    simulation::BaseSimulation, visualization::traits::VisualizationComponent, CutPlane2D,
};
use std::time::Instant;
use wgpu::{Device, Queue};

/// Grid size for the Game of Life
const GRID_WIDTH: u32 = 128;
const GRID_HEIGHT: u32 = 128;

/// Classic Game of Life patterns
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LifePattern {
    Random,
    Glider,
    Blinker,
    GosperGun,
    Clear,
}

impl LifePattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            LifePattern::Random => "Random",
            LifePattern::Glider => "Glider",
            LifePattern::Blinker => "Blinker",
            LifePattern::GosperGun => "Gosper Gun",
            LifePattern::Clear => "Clear",
        }
    }
}

/// GPU resources for Conway's Game of Life compute shader
struct GpuGameOfLifeResources {
    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    // Ping-pong buffers
    buffer_a: wgpu::Buffer, // Current state
    buffer_b: wgpu::Buffer, // Next state
    // Bind groups for ping-pong
    bind_group_a_to_b: wgpu::BindGroup, // Read from A, write to B
    bind_group_b_to_a: wgpu::BindGroup, // Read from B, write to A
    // State
    ping_pong_state: bool, // false = A is current, true = B is current
}

/// Conway's Game of Life simulation using GPU compute shaders with BaseSimulation
struct ConwaysGpuSimulation {
    base: BaseSimulation,
    // Game state
    width: u32,
    height: u32,
    generation: u64,
    current_pattern: LifePattern,
    // Control
    last_update: Instant,
    speed: f32,
    is_paused: bool,
    // GPU resources
    gpu_resources: Option<GpuGameOfLifeResources>,
    // CPU backup for pattern initialization
    cpu_grid: Vec<bool>,
    // Flag to indicate we need to reupload data to GPU
    needs_gpu_upload: bool,
    // Flag to indicate we need to run a manual step
    needs_manual_step: bool,
}

impl ConwaysGpuSimulation {
    fn new() -> Self {
        let mut base = BaseSimulation::new("Conway's Game of Life GPU");

        // Create and configure the data plane visualization
        let mut data_plane = CutPlane2D::new();
        data_plane.set_position(Vector3::new(0.0, 2.0, 0.0));
        data_plane.set_size(2.0);

        // Initialize with empty data for now
        let empty_data = vec![0.0; (GRID_WIDTH * GRID_HEIGHT) as usize];
        data_plane.update_data(empty_data, GRID_WIDTH, GRID_HEIGHT);

        // Add visualization to base
        base.add_visualization("data_plane", data_plane);

        let mut simulation = Self {
            base,
            width: GRID_WIDTH,
            height: GRID_HEIGHT,
            generation: 0,
            current_pattern: LifePattern::Glider,
            last_update: Instant::now(),
            speed: 10.0,
            is_paused: false, // Start unpaused
            gpu_resources: None,
            cpu_grid: vec![false; (GRID_WIDTH * GRID_HEIGHT) as usize],
            needs_gpu_upload: true,
            needs_manual_step: false,
        };

        // Initialize with glider pattern
        simulation.initialize_pattern(LifePattern::Glider);

        // Log initial pattern
        let live_count = simulation.cpu_grid.iter().filter(|&&cell| cell).count();
        println!(
            "🔬 Initial {} pattern: {} live cells",
            simulation.current_pattern.as_str(),
            live_count
        );

        simulation
    }

    /// Initialize the grid with a specific pattern
    fn initialize_pattern(&mut self, pattern: LifePattern) {
        self.current_pattern = pattern;
        self.generation = 0;

        // Clear the CPU grid first
        self.cpu_grid.fill(false);

        match pattern {
            LifePattern::Random => self.initialize_random(),
            LifePattern::Glider => self.initialize_glider(),
            LifePattern::Blinker => self.initialize_blinker(),
            LifePattern::GosperGun => self.initialize_gosper_gun(),
            LifePattern::Clear => {} // Already cleared
        }

        // Mark that we need to upload new data to GPU
        self.needs_gpu_upload = true;
    }

    /// Initialize with random pattern (30% alive)
    fn initialize_random(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        for cell in self.cpu_grid.iter_mut() {
            *cell = rng.random_bool(0.3);
        }
    }

    /// Initialize with a glider pattern
    fn initialize_glider(&mut self) {
        let center_x = self.width / 2;
        let center_y = self.height / 2;

        // Glider pattern
        let glider_coords = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];

        for (dx, dy) in glider_coords.iter() {
            let x = center_x + dx - 1;
            let y = center_y + dy - 1;
            if x < self.width && y < self.height {
                let index = (y * self.width + x) as usize;
                if index < self.cpu_grid.len() {
                    self.cpu_grid[index] = true;
                }
            }
        }
    }

    /// Initialize with blinker pattern
    fn initialize_blinker(&mut self) {
        let center_x = self.width / 2;
        let center_y = self.height / 2;

        // Vertical blinker
        for dy in 0..3 {
            let y = center_y + dy - 1;
            if y < self.height {
                let index = (y * self.width + center_x) as usize;
                if index < self.cpu_grid.len() {
                    self.cpu_grid[index] = true;
                }
            }
        }
    }

    /// Initialize with Gosper Glider Gun pattern
    fn initialize_gosper_gun(&mut self) {
        let start_x = 10;
        let start_y = self.height / 2 - 5;

        // Gosper Glider Gun coordinates
        let gun_coords = [
            (24, 0),
            (22, 1),
            (24, 1),
            (12, 2),
            (13, 2),
            (20, 2),
            (21, 2),
            (34, 2),
            (35, 2),
            (11, 3),
            (15, 3),
            (20, 3),
            (21, 3),
            (34, 3),
            (35, 3),
            (0, 4),
            (1, 4),
            (10, 4),
            (16, 4),
            (20, 4),
            (21, 4),
            (0, 5),
            (1, 5),
            (10, 5),
            (14, 5),
            (16, 5),
            (17, 5),
            (22, 5),
            (24, 5),
            (10, 6),
            (16, 6),
            (24, 6),
            (11, 7),
            (15, 7),
            (12, 8),
            (13, 8),
        ];

        for (x, y) in gun_coords.iter() {
            let grid_x = start_x + *x;
            let grid_y = start_y + *y;
            if grid_x < self.width && grid_y < self.height {
                let index = (grid_y * self.width + grid_x) as usize;
                if index < self.cpu_grid.len() {
                    self.cpu_grid[index] = true;
                }
            }
        }
    }

    /// Initialize GPU resources and buffers
    fn initialize_gpu_resources(&mut self, device: &Device) {
        // Conway's Game of Life compute shader
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Conway's Game of Life Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(CONWAY_COMPUTE_SHADER.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Conway Bind Group Layout"),
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
            label: Some("Conway Compute Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Conway Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &compute_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        // Create ping-pong buffers
        let buffer_size = (self.width * self.height * std::mem::size_of::<u32>() as u32) as u64;

        let buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Conway Buffer A"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Conway Buffer B"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create bind groups for ping-pong
        let bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Conway Bind Group A->B"),
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
            label: Some("Conway Bind Group B->A"),
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

        self.gpu_resources = Some(GpuGameOfLifeResources {
            compute_pipeline,
            bind_group_layout,
            buffer_a,
            buffer_b,
            bind_group_a_to_b,
            bind_group_b_to_a,
            ping_pong_state: false, // Start with A as current
        });
    }

    /// Upload CPU grid data to GPU buffer
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
                    "📡 Uploading initial pattern to GPU: {} live cells",
                    live_count
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

    /// Download GPU buffer data and update CPU grid and visualization
    fn sync_gpu_to_cpu_and_viz(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref gpu_resources) = self.gpu_resources {
            // Create staging buffer to read GPU data
            let buffer_size = (self.width * self.height * std::mem::size_of::<u32>() as u32) as u64;
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Conway Staging Buffer"),
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
                label: Some("Conway Sync Encoder"),
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

                // Convert to f32 for visualization
                let viz_data: Vec<f32> = u32_data
                    .iter()
                    .map(|&val| if val > 0 { 1.0 } else { 0.0 })
                    .collect();

                // Optional: Log generation progress
                if self.generation % 50 == 0 && self.generation > 0 {
                    let live_count = viz_data.iter().filter(|&&val| val > 0.0).count();
                    println!(
                        "🔬 Generation {}: {} live cells",
                        self.generation, live_count
                    );
                }

                // Update existing visualization properly
                self.update_visualization_with_data(viz_data, device, queue);
            }
        }
    }

    /// Update the existing visualization with new data (matching CPU version approach)
    fn update_visualization_with_data(&mut self, data: Vec<f32>, device: &Device, queue: &Queue) {
        let live_count = data.iter().filter(|&&val| val > 0.0).count();

        // Create a new CutPlane2D with proper GPU initialization
        let mut data_plane = CutPlane2D::new();

        // Set data and geometry
        data_plane.update_data(data, self.width, self.height);
        data_plane.set_position(Vector3::new(0.0, 2.0, 0.0));
        data_plane.set_size(2.0);

        // CRITICAL: Initialize the material with GPU resources (was missing!)
        data_plane.initialize(Some(device), Some(queue));
        data_plane.update(0.0, Some(device), Some(queue));

        // Replace the visualization
        self.base.remove_visualization("data_plane");
        self.base.add_visualization("data_plane", data_plane);

        // Debug output for initial generations
        if self.generation == 0 {
            println!("📊 Initial visualization: {} live cells", live_count);
        }
    }

    /// Run one GPU compute step
    fn run_gpu_compute_step(&mut self, device: &Device, queue: &Queue) {
        if let Some(ref mut gpu_resources) = self.gpu_resources {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Conway Compute Encoder"),
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Conway Compute Pass"),
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

                // Dispatch compute shader
                let workgroup_size = 8;
                let num_workgroups_x = (self.width + workgroup_size - 1) / workgroup_size;
                let num_workgroups_y = (self.height + workgroup_size - 1) / workgroup_size;

                compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            queue.submit(std::iter::once(encoder.finish()));

            // Flip ping-pong state
            gpu_resources.ping_pong_state = !gpu_resources.ping_pong_state;
            self.generation += 1;
        }
    }
}

impl haggis::simulation::traits::Simulation for ConwaysGpuSimulation {
    fn initialize(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        self.base.initialize(scene);
        println!("🚀 Conway's Game of Life GPU simulation initialized");
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.base.initialize_gpu(device, queue);

        // Initialize GPU compute resources
        self.initialize_gpu_resources(device);

        // Upload initial pattern to GPU
        self.upload_grid_to_gpu(device, queue);

        // Sync initial data for visualization
        self.sync_gpu_to_cpu_and_viz(device, queue);

        println!("🔧 GPU compute resources initialized");
        println!("✅ Initial GPU data uploaded and visualization synced");
    }

    fn update(&mut self, delta_time: f32, scene: &mut haggis::gfx::scene::Scene) {
        self.base.update(delta_time, scene);
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, _delta_time: f32) {
        // Check if we need to reupload data to GPU (after pattern change)
        if self.needs_gpu_upload && self.gpu_resources.is_some() {
            println!("🔄 Switching to {} pattern", self.current_pattern.as_str());
            self.upload_grid_to_gpu(device, queue);
            self.sync_gpu_to_cpu_and_viz(device, queue);
            self.needs_gpu_upload = false;
        }

        // Handle manual step request
        if self.needs_manual_step && self.gpu_resources.is_some() {
            self.run_gpu_compute_step(device, queue);
            self.sync_gpu_to_cpu_and_viz(device, queue);
            self.needs_manual_step = false;
        }

        // Auto-evolve based on speed setting (if not paused and GPU ready)
        if !self.is_paused && self.speed > 0.0 && self.gpu_resources.is_some() {
            let time_per_generation = 1.0 / self.speed;
            if self.last_update.elapsed().as_secs_f32() >= time_per_generation {
                self.run_gpu_compute_step(device, queue);
                self.sync_gpu_to_cpu_and_viz(device, queue);
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
        ui.window("Conway's Game of Life GPU")
            .size([400.0, 320.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🔬 Conway's Game of Life GPU");
                ui.separator();

                ui.text(&format!("Generation: {}", self.generation));
                ui.text(&format!("Grid Size: {}x{}", self.width, self.height));
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

                // Manual step button (only works if GPU is ready)
                if ui.button("⏭ Step") && self.gpu_resources.is_some() {
                    self.needs_manual_step = true;
                }

                ui.separator();

                // Pattern selection
                ui.text("Pattern:");
                let patterns = [
                    LifePattern::Random,
                    LifePattern::Glider,
                    LifePattern::Blinker,
                    LifePattern::GosperGun,
                    LifePattern::Clear,
                ];
                for pattern in patterns.iter() {
                    if ui.radio_button_bool(pattern.as_str(), self.current_pattern == *pattern) {
                        self.initialize_pattern(*pattern);
                        // Note: This will upload to GPU in the next update_gpu call
                    }
                }

                ui.separator();

                // Speed control
                ui.text("Simulation Speed:");
                let mut speed_value = self.speed;
                if ui
                    .slider_config("Generations/sec", 0.1, 60.0)
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
                ui.text("GPU Compute Features:");
                ui.bullet_text("High-performance cellular automaton");
                ui.bullet_text("Ping-pong buffer optimization");
                ui.bullet_text("Parallel computation on GPU");
            });

        // Render base simulation UI
        self.base.render_ui(ui);
    }

    fn name(&self) -> &str {
        "Conway's Game of Life GPU"
    }

    fn is_running(&self) -> bool {
        !self.is_paused
    }

    fn set_running(&mut self, running: bool) {
        self.is_paused = !running;
    }

    fn reset(&mut self, scene: &mut haggis::gfx::scene::Scene) {
        println!("🐛 Debug: Resetting simulation");
        self.initialize_pattern(self.current_pattern);
        self.base.reset(scene);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.base
    }
}

// Conway's Game of Life compute shader
const CONWAY_COMPUTE_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input_buffer: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_buffer: array<u32>;

// Grid dimensions - these should match the Rust constants
const GRID_WIDTH: u32 = 128u;
const GRID_HEIGHT: u32 = 128u;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    // Check bounds
    if (x >= GRID_WIDTH || y >= GRID_HEIGHT) {
        return;
    }
    
    let index = y * GRID_WIDTH + x;
    
    // Count live neighbors (with wrapping)
    var live_neighbors = 0u;
    
    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue; // Skip self
            }
            
            // Calculate neighbor position with wrapping
            let nx = (i32(x) + dx + i32(GRID_WIDTH)) % i32(GRID_WIDTH);
            let ny = (i32(y) + dy + i32(GRID_HEIGHT)) % i32(GRID_HEIGHT);
            let neighbor_index = u32(ny) * GRID_WIDTH + u32(nx);
            
            if (input_buffer[neighbor_index] == 1u) {
                live_neighbors++;
            }
        }
    }
    
    let current_cell = input_buffer[index];
    
    // Apply Conway's rules
    var next_state = 0u;
    if (current_cell == 1u) {
        // Live cell
        if (live_neighbors == 2u || live_neighbors == 3u) {
            next_state = 1u; // Survives
        }
        // Otherwise dies (stays 0)
    } else {
        // Dead cell
        if (live_neighbors == 3u) {
            next_state = 1u; // Becomes alive
        }
        // Otherwise stays dead (stays 0)
    }
    
    output_buffer[index] = next_state;
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Conway's Game of Life - GPU Implementation");
    println!("============================================");
    println!("High-performance GPU compute shader implementation with ping-pong buffers.");
    println!();
    println!("Features:");
    println!("  • GPU-accelerated cellular automaton simulation");
    println!("  • Ping-pong buffer system for optimal performance");
    println!("  • Real-time visualization of cellular automaton (128x128)");
    println!("  • Speed control: 0.1 to 60.0 generations per second");
    println!("  • Classic Game of Life patterns");
    println!();

    // Create the main application
    let mut app = haggis::default();

    // Create the GPU Conway's Game of Life simulation
    let simulation = ConwaysGpuSimulation::new();

    // Attach the simulation to the app
    app.attach_simulation(simulation);

    // Add reference objects for context
    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 0.0, 0.0], 0.5, 0.0)
        .with_name("Reference Cube at Origin");

    app.add_object("examples/test/cube.obj")
        .with_transform([0.0, 2.0, 0.0], 0.3, 0.0)
        .with_name("Reference Cube at Plane Position");

    // Run the application
    app.run();

    Ok(())
}
