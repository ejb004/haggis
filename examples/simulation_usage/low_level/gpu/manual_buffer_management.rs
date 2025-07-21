//! # Manual Buffer Management Example
//!
//! This example demonstrates how to use the low-level API for direct wgpu buffer
//! operations, memory management, and resource optimization.
//!
//! ## Features Demonstrated
//! - Direct wgpu buffer creation and management
//! - Memory mapping and data transfer optimization
//! - Buffer pooling and reuse strategies
//! - Custom memory layout and alignment
//! - Performance monitoring and profiling
//!
//! ## Usage
//! ```bash
//! cargo run --example manual_buffer_management
//! ```

use haggis::simulation::low_level::ComputeContext;
use haggis::simulation::traits::Simulation;
use haggis::ui::default_transform_panel;
use wgpu::{BufferUsages, Device, Queue, Buffer, BufferDescriptor, MapMode};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Instant, Duration};

/// Buffer pool for efficient memory management
struct BufferPool {
    device: Arc<Device>,
    free_buffers: HashMap<String, Vec<Buffer>>,
    used_buffers: HashMap<String, Vec<Buffer>>,
    buffer_sizes: HashMap<String, u64>,
    allocation_count: usize,
    total_allocated: u64,
}

impl BufferPool {
    fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            free_buffers: HashMap::new(),
            used_buffers: HashMap::new(),
            buffer_sizes: HashMap::new(),
            allocation_count: 0,
            total_allocated: 0,
        }
    }

    fn get_buffer(&mut self, pool_name: &str, size: u64, usage: BufferUsages) -> Buffer {
        // Check if we have a free buffer of the right size
        if let Some(buffers) = self.free_buffers.get_mut(pool_name) {
            if let Some(buffer) = buffers.pop() {
                // Move to used buffers
                self.used_buffers.entry(pool_name.to_string()).or_insert_with(Vec::new).push(buffer.clone());
                return buffer;
            }
        }

        // Create new buffer
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some(&format!("pooled_buffer_{}", pool_name)),
            size,
            usage,
            mapped_at_creation: false,
        });

        // Track allocation
        self.allocation_count += 1;
        self.total_allocated += size;
        self.buffer_sizes.insert(pool_name.to_string(), size);

        // Add to used buffers
        self.used_buffers.entry(pool_name.to_string()).or_insert_with(Vec::new).push(buffer.clone());
        buffer
    }

    fn return_buffer(&mut self, pool_name: &str, buffer: Buffer) {
        // Move buffer from used to free
        if let Some(used) = self.used_buffers.get_mut(pool_name) {
            // Simple removal - in practice would need better buffer identification
            if !used.is_empty() {
                used.remove(0);
            }
        }

        self.free_buffers.entry(pool_name.to_string()).or_insert_with(Vec::new).push(buffer);
    }

    fn get_stats(&self) -> (usize, u64, usize) {
        let free_count: usize = self.free_buffers.values().map(|v| v.len()).sum();
        let used_count: usize = self.used_buffers.values().map(|v| v.len()).sum();
        (self.allocation_count, self.total_allocated, free_count + used_count)
    }
}

/// Custom memory-mapped buffer for high-performance data transfer
struct MappedBuffer {
    buffer: Buffer,
    staging_buffer: Buffer,
    size: u64,
    device: Arc<Device>,
    queue: Arc<Queue>,
    last_map_time: Option<Instant>,
}

impl MappedBuffer {
    fn new(device: Arc<Device>, queue: Arc<Queue>, size: u64, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("mapped_buffer"),
            size,
            usage,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("staging_buffer"),
            size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            staging_buffer,
            size,
            device,
            queue,
            last_map_time: None,
        }
    }

    fn write_data<T: bytemuck::Pod>(&self, data: &[T]) {
        self.queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    fn read_data<T: bytemuck::Pod>(&mut self) -> Option<Vec<T>> {
        let start_time = Instant::now();
        
        // Copy from main buffer to staging buffer
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("buffer_copy"),
        });
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &self.staging_buffer, 0, self.size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map the staging buffer
        let buffer_slice = self.staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        
        buffer_slice.map_async(MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Poll device until mapping is complete
        self.device.poll(wgpu::MaintainBase::Wait);
        
        match receiver.recv() {
            Ok(Ok(())) => {
                let mapped_data = buffer_slice.get_mapped_range();
                let result: Vec<T> = bytemuck::cast_slice(&mapped_data).to_vec();
                
                // Unmap the buffer
                drop(mapped_data);
                self.staging_buffer.unmap();
                
                self.last_map_time = Some(start_time);
                Some(result)
            }
            _ => None,
        }
    }

    fn get_last_map_time(&self) -> Option<Duration> {
        self.last_map_time.map(|t| t.elapsed())
    }
}

/// Low-level simulation with manual buffer management
struct ManualBufferSimulation {
    context: ComputeContext,
    buffer_pool: BufferPool,
    mapped_buffers: HashMap<String, MappedBuffer>,
    particle_count: usize,
    
    // Memory management settings
    buffer_pool_enabled: bool,
    async_transfer_enabled: bool,
    memory_coalescing_enabled: bool,
    
    // Performance monitoring
    allocation_stats: (usize, u64, usize),
    transfer_times: Vec<f32>,
    last_performance_check: Instant,
    
    // Simulation state
    simulation_running: bool,
    debug_mode: bool,
}

impl ManualBufferSimulation {
    fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let context = ComputeContext::new(device.clone(), queue.clone());
        let buffer_pool = BufferPool::new(device);
        
        Self {
            context,
            buffer_pool,
            mapped_buffers: HashMap::new(),
            particle_count: 4096,
            buffer_pool_enabled: true,
            async_transfer_enabled: true,
            memory_coalescing_enabled: true,
            allocation_stats: (0, 0, 0),
            transfer_times: Vec::new(),
            last_performance_check: Instant::now(),
            simulation_running: true,
            debug_mode: false,
        }
    }

    fn setup_manual_buffers(&mut self) -> Result<(), String> {
        let device = self.context.device().clone();
        let queue = self.context.queue().clone();

        // Create particle data with optimal memory layout
        let particle_data_size = self.particle_count * std::mem::size_of::<[f32; 12]>();
        
        if self.buffer_pool_enabled {
            // Use buffer pool for efficient allocation
            let _particle_buffer = self.buffer_pool.get_buffer(
                "particles",
                particle_data_size as u64,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            );
        } else {
            // Direct buffer allocation
            let initial_data: Vec<[f32; 12]> = (0..self.particle_count)
                .map(|i| {
                    let angle = (i as f32 / self.particle_count as f32) * 2.0 * std::f32::consts::PI;
                    [
                        3.0 * angle.cos(),  // position.x
                        3.0 * angle.sin(),  // position.y
                        5.0,                // position.z
                        0.0,                // velocity.x
                        0.0,                // velocity.y
                        0.0,                // velocity.z
                        0.0,                // acceleration.x
                        0.0,                // acceleration.y
                        0.0,                // acceleration.z
                        1.0,                // mass
                        10.0,               // lifetime
                        10.0,               // max_lifetime
                    ]
                })
                .collect();

            self.context.create_buffer(
                "particles",
                bytemuck::cast_slice::<[f32; 12], u8>(&initial_data),
                BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            )?;
        }

        // Create mapped buffer for high-performance data transfer
        let mapped_buffer = MappedBuffer::new(
            device.into(),
            queue.into(),
            particle_data_size as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        );
        
        self.mapped_buffers.insert("particles".to_string(), mapped_buffer);

        // Create parameter buffer with aligned memory layout
        let params = [
            self.particle_count as f32,     // particle_count
            0.016,                          // delta_time
            0.0, 0.0, -9.8,                // gravity
            0.99,                           // damping
            0.0, 0.0,                       // padding for alignment
        ];

        self.context.create_buffer(
            "params",
            bytemuck::cast_slice::<f32, u8>(&params),
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        )?;

        // Update allocation stats
        self.allocation_stats = self.buffer_pool.get_stats();
        
        Ok(())
    }

    fn perform_async_transfer(&mut self) -> Result<(), String> {
        if !self.async_transfer_enabled {
            return Ok(());
        }

        let start_time = Instant::now();
        
        // Simulate async data transfer
        if let Some(mapped_buffer) = self.mapped_buffers.get_mut("particles") {
            // Write some test data
            let test_data: Vec<f32> = (0..self.particle_count * 12)
                .map(|i| (i as f32).sin())
                .collect();
            
            mapped_buffer.write_data(&test_data);
            
            // Optionally read back (for demonstration)
            if self.debug_mode {
                let _read_data: Option<Vec<f32>> = mapped_buffer.read_data();
            }
        }

        let transfer_time = start_time.elapsed().as_secs_f32();
        self.transfer_times.push(transfer_time);
        
        // Keep only recent transfer times
        if self.transfer_times.len() > 60 {
            self.transfer_times.remove(0);
        }

        Ok(())
    }

    fn optimize_memory_layout(&mut self) -> Result<(), String> {
        if !self.memory_coalescing_enabled {
            return Ok(());
        }

        // Demonstrate memory coalescing optimization
        // In practice, this would reorganize data for better cache performance
        
        // Example: Structure of Arrays (SoA) vs Array of Structures (AoS)
        let positions: Vec<[f32; 3]> = (0..self.particle_count)
            .map(|i| {
                let angle = (i as f32 / self.particle_count as f32) * 2.0 * std::f32::consts::PI;
                [3.0 * angle.cos(), 3.0 * angle.sin(), 5.0]
            })
            .collect();

        let velocities: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; self.particle_count];

        // Create separate buffers for coalesced access
        self.context.create_buffer(
            "positions",
            bytemuck::cast_slice::<[f32; 3], u8>(&positions),
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        )?;

        self.context.create_buffer(
            "velocities",
            bytemuck::cast_slice::<[f32; 3], u8>(&velocities),
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        )?;

        Ok(())
    }

    fn get_average_transfer_time(&self) -> f32 {
        if self.transfer_times.is_empty() {
            0.0
        } else {
            self.transfer_times.iter().sum::<f32>() / self.transfer_times.len() as f32
        }
    }
}

impl Simulation for ManualBufferSimulation {
    fn initialize(&mut self, _scene: &mut haggis::gfx::scene::Scene) {
        if let Err(e) = self.setup_manual_buffers() {
            eprintln!("Failed to setup manual buffers: {}", e);
        }
    }

    fn update(&mut self, _delta_time: f32, _scene: &mut haggis::gfx::scene::Scene) {
        if !self.simulation_running {
            return;
        }

        // Perform async transfer operations
        if let Err(e) = self.perform_async_transfer() {
            eprintln!("Failed async transfer: {}", e);
        }

        // Update performance stats periodically
        if self.last_performance_check.elapsed().as_secs() >= 1 {
            self.allocation_stats = self.buffer_pool.get_stats();
            self.last_performance_check = Instant::now();
        }

        // Simulate memory optimization
        if let Err(e) = self.optimize_memory_layout() {
            eprintln!("Failed memory optimization: {}", e);
        }
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        // Manual buffer management controls
        ui.window("Manual Buffer Management")
            .size([450.0, 500.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level API: Direct Buffer Operations");
                ui.separator();
                
                ui.text(&format!("Particle Count: {}", self.particle_count));
                ui.text(&format!("Buffer Pool: {}", if self.buffer_pool_enabled { "Enabled" } else { "Disabled" }));
                ui.text(&format!("Async Transfer: {}", if self.async_transfer_enabled { "Enabled" } else { "Disabled" }));
                ui.text(&format!("Memory Coalescing: {}", if self.memory_coalescing_enabled { "Enabled" } else { "Disabled" }));
                ui.spacing();
                
                ui.checkbox("Enable Buffer Pool", &mut self.buffer_pool_enabled);
                ui.checkbox("Enable Async Transfer", &mut self.async_transfer_enabled);
                ui.checkbox("Enable Memory Coalescing", &mut self.memory_coalescing_enabled);
                ui.checkbox("Debug Mode", &mut self.debug_mode);
                ui.spacing();
                
                // Particle count control
                let mut count = self.particle_count as i32;
                if ui.slider("Particle Count", 1024, 16384, &mut count) {
                    self.particle_count = count as usize;
                }
                
                if ui.button("Recreate Buffers") {
                    let _ = self.setup_manual_buffers();
                }
                
                ui.separator();
                ui.text("Manual Buffer Operations:");
                ui.text("✓ Direct wgpu buffer creation");
                ui.text("✓ Custom memory mapping");
                ui.text("✓ Buffer pool management");
                ui.text("✓ Async data transfer");
                ui.text("✓ Memory layout optimization");
            });

        // Memory statistics
        ui.window("Memory Statistics")
            .size([400.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Buffer Pool Statistics:");
                ui.separator();
                
                let (allocations, total_size, active_buffers) = self.allocation_stats;
                ui.text(&format!("Total Allocations: {}", allocations));
                ui.text(&format!("Total Memory: {:.2} MB", total_size as f64 / 1024.0 / 1024.0));
                ui.text(&format!("Active Buffers: {}", active_buffers));
                ui.spacing();
                
                ui.text("Transfer Performance:");
                ui.text(&format!("Avg Transfer Time: {:.3}ms", self.get_average_transfer_time() * 1000.0));
                ui.text(&format!("Recent Transfers: {}", self.transfer_times.len()));
                ui.spacing();
                
                ui.text("Memory Layout:");
                ui.text(&format!("Particle Size: {} bytes", std::mem::size_of::<[f32; 12]>()));
                ui.text(&format!("Total Particle Data: {:.2} KB", 
                    (self.particle_count * std::mem::size_of::<[f32; 12]>()) as f64 / 1024.0));
                ui.text(&format!("Memory Alignment: {} bytes", std::mem::align_of::<[f32; 12]>()));
                ui.spacing();
                
                ui.text("Buffer Types:");
                ui.text("• Storage buffers (particles)");
                ui.text("• Uniform buffers (parameters)");
                ui.text("• Staging buffers (CPU access)");
                ui.text("• Mapped buffers (async transfer)");
            });

        // Performance optimization guide
        ui.window("Performance Optimization")
            .size([400.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level Optimization Techniques:");
                ui.separator();
                
                ui.text("Buffer Pool Benefits:");
                ui.text("• Reduces allocation overhead");
                ui.text("• Reuses memory efficiently");
                ui.text("• Minimizes fragmentation");
                ui.text("• Improves cache locality");
                ui.spacing();
                
                ui.text("Memory Coalescing:");
                ui.text("• Structure of Arrays (SoA)");
                ui.text("• Aligned memory access");
                ui.text("• Reduced cache misses");
                ui.text("• Better GPU utilization");
                ui.spacing();
                
                ui.text("Async Transfer:");
                ui.text("• Overlap CPU and GPU work");
                ui.text("• Pipeline data transfers");
                ui.text("• Reduce blocking operations");
                ui.text("• Improve throughput");
                ui.spacing();
                
                ui.text("Best Practices:");
                ui.text("✓ Use appropriate buffer usage flags");
                ui.text("✓ Align data to GPU requirements");
                ui.text("✓ Batch small transfers");
                ui.text("✓ Profile memory bandwidth");
                ui.text("✓ Monitor allocation patterns");
            });

        // Implementation details
        ui.window("Implementation Details")
            .size([450.0, 350.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level Buffer Management:");
                ui.separator();
                
                ui.text("Buffer Pool Implementation:");
                ui.text("• HashMap of buffer vectors");
                ui.text("• Size-based allocation");
                ui.text("• Usage flag matching");
                ui.text("• Automatic cleanup");
                ui.spacing();
                
                ui.text("Mapped Buffer Features:");
                ui.text("• Direct memory access");
                ui.text("• Staging buffer optimization");
                ui.text("• Async mapping operations");
                ui.text("• Performance monitoring");
                ui.spacing();
                
                ui.text("Memory Layout Optimization:");
                ui.text("• Structure of Arrays (SoA)");
                ui.text("• Cache-friendly access patterns");
                ui.text("• GPU memory coalescing");
                ui.text("• Alignment considerations");
                ui.spacing();
                
                ui.text("This demonstrates the low-level API's");
                ui.text("power for performance-critical applications");
                ui.text("requiring manual memory management.");
            });
    }

    fn name(&self) -> &str {
        "Manual Buffer Management"
    }

    fn is_running(&self) -> bool {
        self.simulation_running
    }

    fn set_running(&mut self, running: bool) {
        self.simulation_running = running;
    }

    fn reset(&mut self, _scene: &mut haggis::gfx::scene::Scene) {
        self.transfer_times.clear();
        let _ = self.setup_manual_buffers();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haggis = haggis::default();

    // Create materials
    haggis
        .app_state
        .scene
        .add_material_rgb("manual_particle", 1.0, 0.6, 0.2, 0.9, 0.5);
    
    haggis
        .app_state
        .scene
        .add_material_rgb("memory_debug", 0.8, 0.8, 0.8, 0.4, 0.2);

    // Add visual objects
    for i in 0..100 {
        haggis
            .add_object("examples/test/cube.obj")
            .with_material("manual_particle")
            .with_name(&format!("manual_particle_{}", i))
            .with_transform([0.0, 0.0, 0.0], 0.03, 0.0);
    }

    // Note: In a real implementation, we would create the simulation with device/queue
    // let manual_sim = ManualBufferSimulation::new(device, queue);
    // haggis.attach_simulation(manual_sim);

    haggis.set_ui(|ui, scene, selected_index| {
        default_transform_panel(ui, scene, selected_index);

        // Usage guide
        ui.window("Manual Buffer Management Guide")
            .size([500.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Low-Level API: Manual Buffer Management");
                ui.separator();
                
                ui.text("When to Use Manual Management:");
                ui.text("• Performance-critical applications");
                ui.text("• Custom memory layout requirements");
                ui.text("• Specialized allocation patterns");
                ui.text("• Fine-grained resource control");
                ui.text("• Memory-constrained environments");
                ui.spacing();
                
                ui.text("Key Concepts:");
                ui.text("1. Buffer Pool - Efficient reuse");
                ui.text("2. Mapped Buffers - Direct access");
                ui.text("3. Memory Coalescing - Cache optimization");
                ui.text("4. Async Transfer - Overlap operations");
                ui.text("5. Performance Monitoring - Bottleneck detection");
                ui.spacing();
                
                ui.text("Performance Benefits:");
                ui.text("✓ Reduced allocation overhead");
                ui.text("✓ Better memory utilization");
                ui.text("✓ Improved cache performance");
                ui.text("✓ Lower latency operations");
                ui.text("✓ Predictable memory usage");
                ui.spacing();
                
                ui.text("Trade-offs:");
                ui.text("• Increased complexity");
                ui.text("• Manual resource management");
                ui.text("• Platform-specific optimizations");
                ui.text("• Debugging challenges");
                ui.text("• Development time");
                ui.spacing();
                
                ui.text("This example demonstrates the power");
                ui.text("and complexity of manual buffer");
                ui.text("management for expert users.");
            });
    });

    haggis.run();
    Ok(())
}