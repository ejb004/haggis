//! # Mid-Level Simulation API
//!
//! This module provides convenient wrappers around the existing trait system,
//! offering more control than the high-level API while still providing helpful
//! abstractions for common patterns.
//!
//! ## Features
//!
//! - **Trait Extensions**: Additional methods for existing simulation traits
//! - **Resource Helpers**: Simplified GPU buffer and pipeline management
//! - **Performance Utilities**: Profiling and optimization helpers
//! - **Interop Layer**: Bridge between high-level and low-level APIs
//!
//! ## Usage
//!
//! This layer is ideal for developers who need more control than the high-level
//! API provides but don't want to manage raw wgpu resources directly.

use crate::gfx::scene::Scene;
use crate::simulation::traits::Simulation;
use wgpu::{Device, Queue};
use wgpu::util::DeviceExt;
use std::collections::HashMap;
use std::time::Instant;

/// Extension trait for the base Simulation trait
pub trait SimulationExt: Simulation {
    /// Provides timing information for performance monitoring
    fn get_timing_info(&self) -> TimingInfo {
        TimingInfo::default()
    }

    /// Gets memory usage statistics
    fn get_memory_usage(&self) -> MemoryUsage {
        MemoryUsage::default()
    }

    /// Provides debug information
    fn get_debug_info(&self) -> DebugInfo {
        DebugInfo::default()
    }

    /// Called when simulation should pause/resume
    fn on_pause_changed(&mut self, _paused: bool) {
        // Default implementation - can be overridden
    }

    /// Called when simulation parameters change
    fn on_parameters_changed(&mut self, _parameters: &HashMap<String, f32>) {
        // Default implementation - can be overridden
    }
}

/// Timing information for performance monitoring
#[derive(Debug, Clone)]
pub struct TimingInfo {
    pub last_update_time: f32,
    pub average_update_time: f32,
    pub total_updates: u64,
    pub updates_per_second: f32,
}

impl Default for TimingInfo {
    fn default() -> Self {
        Self {
            last_update_time: 0.0,
            average_update_time: 0.0,
            total_updates: 0,
            updates_per_second: 0.0,
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub cpu_memory_bytes: usize,
    pub gpu_memory_bytes: usize,
    pub buffer_count: usize,
    pub texture_count: usize,
}

impl Default for MemoryUsage {
    fn default() -> Self {
        Self {
            cpu_memory_bytes: 0,
            gpu_memory_bytes: 0,
            buffer_count: 0,
            texture_count: 0,
        }
    }
}

/// Debug information for development
#[derive(Debug, Clone)]
pub struct DebugInfo {
    pub simulation_type: String,
    pub active_objects: usize,
    pub performance_warnings: Vec<String>,
    pub resource_warnings: Vec<String>,
}

impl Default for DebugInfo {
    fn default() -> Self {
        Self {
            simulation_type: "Unknown".to_string(),
            active_objects: 0,
            performance_warnings: Vec::new(),
            resource_warnings: Vec::new(),
        }
    }
}

/// Wrapper for simulations with automatic timing and profiling
pub struct ManagedSimulation<T: Simulation> {
    simulation: T,
    timing_info: TimingInfo,
    last_frame_time: Option<Instant>,
    parameters: HashMap<String, f32>,
    debug_mode: bool,
}

impl<T: Simulation> ManagedSimulation<T> {
    /// Creates a new managed simulation wrapper
    pub fn new(simulation: T) -> Self {
        Self {
            simulation,
            timing_info: TimingInfo::default(),
            last_frame_time: None,
            parameters: HashMap::new(),
            debug_mode: false,
        }
    }

    /// Enables debug mode with additional logging and checks
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug_mode = debug;
        self
    }

    /// Sets a parameter value
    pub fn set_parameter(&mut self, name: String, value: f32) {
        self.parameters.insert(name, value);
        // Note: SimulationExt calls would need to be made directly on the concrete type
        // This is a simplified version for now
    }

    /// Gets a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<f32> {
        self.parameters.get(name).copied()
    }

    /// Gets the wrapped simulation
    pub fn inner(&self) -> &T {
        &self.simulation
    }

    /// Gets the wrapped simulation mutably
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.simulation
    }

    /// Gets timing information
    pub fn timing_info(&self) -> &TimingInfo {
        &self.timing_info
    }

    /// Updates timing statistics
    fn update_timing(&mut self, delta_time: f32) {
        self.timing_info.total_updates += 1;
        self.timing_info.last_update_time = delta_time;
        
        // Calculate running average
        let alpha = 0.1; // Smoothing factor
        self.timing_info.average_update_time = 
            alpha * delta_time + (1.0 - alpha) * self.timing_info.average_update_time;
        
        // Calculate updates per second
        if delta_time > 0.0 {
            self.timing_info.updates_per_second = 
                alpha * (1.0 / delta_time) + (1.0 - alpha) * self.timing_info.updates_per_second;
        }
    }
}

// Unfortunately, we can't implement as_any_mut for the base Simulation trait
// without modifying it, so we'll create a helper trait for that
trait SimulationAny {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: Simulation + 'static> SimulationAny for T {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl<T: Simulation + 'static> Simulation for ManagedSimulation<T> {
    fn initialize(&mut self, scene: &mut Scene) {
        self.simulation.initialize(scene);
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        let start_time = Instant::now();
        
        self.simulation.update(delta_time, scene);
        
        let elapsed = start_time.elapsed().as_secs_f32();
        self.update_timing(elapsed);
        
        if self.debug_mode && elapsed > 0.016 {
            println!("Warning: Simulation update took {:.3}ms (>16ms)", elapsed * 1000.0);
        }
    }

    fn render_ui(&mut self, ui: &imgui::Ui) {
        // Render the wrapped simulation's UI
        self.simulation.render_ui(ui);
        
        // Add performance monitoring UI if in debug mode
        if self.debug_mode {
            ui.window(&format!("{} - Performance", self.simulation.name()))
                .size([300.0, 150.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text(format!("Updates: {}", self.timing_info.total_updates));
                    ui.text(format!("Last: {:.3}ms", self.timing_info.last_update_time * 1000.0));
                    ui.text(format!("Avg: {:.3}ms", self.timing_info.average_update_time * 1000.0));
                    ui.text(format!("FPS: {:.1}", self.timing_info.updates_per_second));
                    
                    if self.timing_info.average_update_time > 0.016 {
                        ui.text_colored([1.0, 1.0, 0.0, 1.0], "Performance Warning!");
                    }
                });
        }
    }

    fn name(&self) -> &str {
        self.simulation.name()
    }

    fn is_running(&self) -> bool {
        self.simulation.is_running()
    }

    fn set_running(&mut self, running: bool) {
        self.simulation.set_running(running);
        // Note: SimulationExt calls would need to be made directly on the concrete type
        // This is a simplified version for now
    }

    fn reset(&mut self, scene: &mut Scene) {
        self.simulation.reset(scene);
        self.timing_info = TimingInfo::default();
        self.last_frame_time = None;
    }

    fn cleanup(&mut self, scene: &mut Scene) {
        self.simulation.cleanup(scene);
    }

    fn initialize_gpu(&mut self, device: &Device, queue: &Queue) {
        self.simulation.initialize_gpu(device, queue);
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, delta_time: f32) {
        self.simulation.update_gpu(device, queue, delta_time);
    }

    fn apply_gpu_results_to_scene(&mut self, device: &Device, scene: &mut Scene) {
        self.simulation.apply_gpu_results_to_scene(device, scene);
    }

    fn is_gpu_ready(&self) -> bool {
        self.simulation.is_gpu_ready()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// GPU resource manager for simulations
pub struct GpuResourceManager {
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    buffers: HashMap<String, wgpu::Buffer>,
    textures: HashMap<String, wgpu::Texture>,
    bind_groups: HashMap<String, wgpu::BindGroup>,
    pipelines: HashMap<String, wgpu::ComputePipeline>,
}

impl GpuResourceManager {
    /// Creates a new GPU resource manager
    pub fn new() -> Self {
        Self {
            device: None,
            queue: None,
            buffers: HashMap::new(),
            textures: HashMap::new(),
            bind_groups: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }

    /// Initializes with GPU device and queue
    pub fn initialize(&mut self, device: wgpu::Device, queue: wgpu::Queue) {
        self.device = Some(device);
        self.queue = Some(queue);
    }

    /// Creates a buffer with the given name and data
    pub fn create_buffer<T: bytemuck::Pod>(&mut self, name: &str, data: &[T], usage: wgpu::BufferUsages) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(name),
            contents: bytemuck::cast_slice(data),
            usage,
        });
        
        self.buffers.insert(name.to_string(), buffer);
        Ok(())
    }

    /// Gets a buffer by name
    pub fn get_buffer(&self, name: &str) -> Option<&wgpu::Buffer> {
        self.buffers.get(name)
    }

    /// Updates a buffer with new data
    pub fn update_buffer<T: bytemuck::Pod>(&self, name: &str, data: &[T]) -> Result<(), String> {
        let queue = self.queue.as_ref().ok_or("Queue not initialized")?;
        let buffer = self.buffers.get(name).ok_or("Buffer not found")?;
        
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
        Ok(())
    }

    /// Creates a compute pipeline
    pub fn create_compute_pipeline(&mut self, name: &str, shader_source: &str) -> Result<(), String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{}_shader", name)),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&format!("{}_pipeline", name)),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });
        
        self.pipelines.insert(name.to_string(), pipeline);
        Ok(())
    }

    /// Gets a compute pipeline by name
    pub fn get_pipeline(&self, name: &str) -> Option<&wgpu::ComputePipeline> {
        self.pipelines.get(name)
    }

    /// Cleans up all resources
    pub fn cleanup(&mut self) {
        self.buffers.clear();
        self.textures.clear();
        self.bind_groups.clear();
        self.pipelines.clear();
    }
}

/// Batch processor for efficient simulation updates
pub struct BatchProcessor {
    batch_size: usize,
    current_batch: Vec<usize>,
    total_items: usize,
}

impl BatchProcessor {
    /// Creates a new batch processor
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            current_batch: Vec::with_capacity(batch_size),
            total_items: 0,
        }
    }

    /// Processes items in batches
    pub fn process<T, F>(&mut self, items: &mut [T], mut processor: F)
    where
        F: FnMut(&mut [T]),
    {
        self.total_items = items.len();
        
        for chunk in items.chunks_mut(self.batch_size) {
            processor(chunk);
        }
    }

    /// Gets batch statistics
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let num_batches = (self.total_items + self.batch_size - 1) / self.batch_size;
        (self.total_items, self.batch_size, num_batches)
    }
}

/// Simulation profiler for performance analysis
pub struct SimulationProfiler {
    timings: HashMap<String, Vec<f32>>,
    max_samples: usize,
    enabled: bool,
}

impl SimulationProfiler {
    /// Creates a new profiler
    pub fn new(max_samples: usize) -> Self {
        Self {
            timings: HashMap::new(),
            max_samples,
            enabled: true,
        }
    }

    /// Records a timing sample
    pub fn record(&mut self, name: &str, duration: f32) {
        if !self.enabled {
            return;
        }
        
        let samples = self.timings.entry(name.to_string()).or_insert_with(Vec::new);
        samples.push(duration);
        
        if samples.len() > self.max_samples {
            samples.remove(0);
        }
    }

    /// Gets average timing for a named section
    pub fn get_average(&self, name: &str) -> Option<f32> {
        self.timings.get(name).map(|samples| {
            samples.iter().sum::<f32>() / samples.len() as f32
        })
    }

    /// Gets all timing statistics
    pub fn get_all_stats(&self) -> HashMap<String, (f32, f32, f32)> {
        self.timings.iter().map(|(name, samples)| {
            let avg = samples.iter().sum::<f32>() / samples.len() as f32;
            let min = samples.iter().copied().fold(f32::INFINITY, f32::min);
            let max = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            (name.clone(), (avg, min, max))
        }).collect()
    }

    /// Clears all timing data
    pub fn clear(&mut self) {
        self.timings.clear();
    }

    /// Enables or disables profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Convenience macros for timing simulation code
#[macro_export]
macro_rules! time_section {
    ($profiler:expr, $name:expr, $code:block) => {
        {
            let start = std::time::Instant::now();
            let result = $code;
            let elapsed = start.elapsed().as_secs_f32();
            $profiler.record($name, elapsed);
            result
        }
    };
}

/// Helper for creating common simulation patterns
pub mod patterns {
    use super::*;
    use crate::simulation::high_level::ParticleSystem;

    /// Creates a physics-based particle system with timing
    pub fn timed_particles(name: &str, count: usize) -> ManagedSimulation<crate::simulation::high_level::ParticleSimulation> {
        let particles = ParticleSystem::new()
            .with_count(count)
            .with_gravity([0.0, 0.0, -9.8])
            .with_ground(0.0)
            .build();
        
        let simulation = crate::simulation::high_level::ParticleSimulation::new(name.to_string(), particles);
        ManagedSimulation::new(simulation).with_debug(true)
    }

    /// Creates a wind-affected particle system
    pub fn wind_simulation(name: &str, count: usize, wind_strength: f32) -> ManagedSimulation<crate::simulation::high_level::ParticleSimulation> {
        let particles = ParticleSystem::new()
            .with_count(count)
            .with_force([wind_strength, 0.0, 0.0])
            .with_bounds([-10.0, 10.0], [-10.0, 10.0], [0.0, 20.0])
            .build();
        
        let simulation = crate::simulation::high_level::ParticleSimulation::new(name.to_string(), particles);
        ManagedSimulation::new(simulation).with_debug(true)
    }
}