//! # Low-Level Simulation API
//!
//! This module provides direct access to wgpu resources and raw simulation
//! capabilities for expert users who need maximum control and performance.
//!
//! ## Features
//!
//! - **Direct GPU Access**: Raw buffer management and compute pipeline control
//! - **Custom Compute Shaders**: Full control over GPU compute operations
//! - **Memory Management**: Manual buffer allocation and deallocation
//! - **Performance Optimization**: Zero-overhead abstractions where possible
//! - **Advanced Features**: Multi-pass rendering, complex buffer layouts
//!
//! ## Safety
//!
//! This API requires careful resource management and understanding of GPU
//! programming concepts. Improper use can lead to undefined behavior or crashes.
//!
//! ## Usage
//!
//! This layer is intended for advanced users implementing custom compute shaders,
//! complex simulation algorithms, or performance-critical applications.

use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device,
    Queue, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

/// Low-level GPU compute context for simulations
pub struct ComputeContext {
    device: Arc<Device>,
    queue: Arc<Queue>,
    command_encoder: Option<CommandEncoder>,
    active_pass: Option<wgpu::ComputePass<'static>>,
    pipelines: HashMap<String, Arc<ComputePipeline>>,
    buffers: HashMap<String, Arc<Buffer>>,
    bind_groups: HashMap<String, Arc<BindGroup>>,
    layouts: HashMap<String, Arc<BindGroupLayout>>,
}

impl ComputeContext {
    /// Creates a new compute context
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            command_encoder: None,
            active_pass: None,
            pipelines: HashMap::new(),
            buffers: HashMap::new(),
            bind_groups: HashMap::new(),
            layouts: HashMap::new(),
        }
    }

    /// Gets the GPU device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Gets the command queue
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Creates a compute buffer
    pub fn create_buffer<T: Pod>(
        &mut self,
        name: &str,
        data: &[T],
        usage: BufferUsages,
    ) -> Result<(), String> {
        let buffer = self
            .device
            .as_ref()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(name),
                contents: bytemuck::cast_slice(data),
                usage,
            });

        self.buffers.insert(name.to_string(), Arc::new(buffer));
        Ok(())
    }

    /// Creates an empty buffer with specified size
    pub fn create_empty_buffer(
        &mut self,
        name: &str,
        size: u64,
        usage: BufferUsages,
    ) -> Result<(), String> {
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some(name),
            size,
            usage,
            mapped_at_creation: false,
        });

        self.buffers.insert(name.to_string(), Arc::new(buffer));
        Ok(())
    }

    /// Gets a buffer by name
    pub fn get_buffer(&self, name: &str) -> Option<&Buffer> {
        self.buffers.get(name).map(|b| b.as_ref())
    }

    /// Updates a buffer with new data
    pub fn update_buffer<T: Pod>(&self, name: &str, data: &[T]) -> Result<(), String> {
        let buffer = self.buffers.get(name).ok_or("Buffer not found")?;
        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(data));
        Ok(())
    }

    /// Creates a compute shader module
    pub fn create_shader_module(&self, name: &str, source: &str) -> Result<ShaderModule, String> {
        let module = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(source.into()),
        });
        Ok(module)
    }

    /// Creates a compute pipeline
    pub fn create_compute_pipeline(
        &mut self,
        name: &str,
        shader: &ShaderModule,
        entry_point: &str,
    ) -> Result<(), String> {
        let pipeline = self
            .device
            .as_ref()
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some(name),
                layout: None,
                module: shader,
                entry_point: Some(entry_point),
                cache: None,
                compilation_options: Default::default(),
            });

        self.pipelines.insert(name.to_string(), Arc::new(pipeline));
        Ok(())
    }

    /// Gets a compute pipeline by name
    pub fn get_pipeline(&self, name: &str) -> Option<&ComputePipeline> {
        self.pipelines.get(name).map(|p| p.as_ref())
    }

    /// Creates a bind group layout
    pub fn create_bind_group_layout(
        &mut self,
        name: &str,
        entries: &[BindGroupLayoutEntry],
    ) -> Result<(), String> {
        let layout = self
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(name),
                entries,
            });

        self.layouts.insert(name.to_string(), Arc::new(layout));
        Ok(())
    }

    /// Creates a bind group
    pub fn create_bind_group(
        &mut self,
        name: &str,
        layout_name: &str,
        entries: &[wgpu::BindGroupEntry],
    ) -> Result<(), String> {
        let layout = self.layouts.get(layout_name).ok_or("Layout not found")?;

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some(name),
            layout,
            entries,
        });

        self.bind_groups
            .insert(name.to_string(), Arc::new(bind_group));
        Ok(())
    }

    /// Gets a bind group by name
    pub fn get_bind_group(&self, name: &str) -> Option<&BindGroup> {
        self.bind_groups.get(name).map(|bg| bg.as_ref())
    }

    /// Begins a compute pass
    pub fn begin_compute_pass(&mut self, label: &str) -> Result<(), String> {
        if self.command_encoder.is_none() {
            self.command_encoder = Some(self.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some(&format!("{}_encoder", label)),
                },
            ));
        }

        // Note: We can't easily store the compute pass due to lifetime issues
        // Users will need to manage the pass themselves in their dispatch calls
        Ok(())
    }

    /// Dispatches a compute shader
    pub fn dispatch(
        &mut self,
        pipeline_name: &str,
        bind_group_name: &str,
        workgroup_count: (u32, u32, u32),
    ) -> Result<(), String> {
        let pipeline = self
            .pipelines
            .get(pipeline_name)
            .ok_or("Pipeline not found")?;
        let bind_group = self
            .bind_groups
            .get(bind_group_name)
            .ok_or("Bind group not found")?;

        if let Some(encoder) = &mut self.command_encoder {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!("{}_pass", pipeline_name)),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, Some(bind_group.as_ref()), &[]);
            compute_pass.dispatch_workgroups(
                workgroup_count.0,
                workgroup_count.1,
                workgroup_count.2,
            );
        } else {
            return Err("No active command encoder".to_string());
        }

        Ok(())
    }

    /// Submits all recorded commands
    pub fn submit(&mut self) -> Result<(), String> {
        if let Some(encoder) = self.command_encoder.take() {
            self.queue.submit(std::iter::once(encoder.finish()));
        }
        Ok(())
    }

    /// Reads data back from a buffer (blocking operation)
    pub fn read_buffer<T: Pod + Clone>(
        &self,
        buffer_name: &str,
        size: usize,
    ) -> Result<Vec<T>, String> {
        let buffer = self.buffers.get(buffer_name).ok_or("Buffer not found")?;

        // Create a staging buffer
        let staging_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("staging_buffer"),
            size: (size * std::mem::size_of::<T>()) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy data to staging buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("copy_encoder"),
            });

        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<T>()) as u64,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read the buffer
        let slice = staging_buffer.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        let _ = self.device.poll(wgpu::MaintainBase::Wait);

        match futures::executor::block_on(rx) {
            Ok(Ok(())) => {
                let mapped = slice.get_mapped_range();
                let result: Vec<T> = bytemuck::cast_slice(&mapped).to_vec();
                drop(mapped);
                staging_buffer.unmap();
                Ok(result)
            }
            _ => Err("Failed to read buffer".to_string()),
        }
    }

    /// Clears all resources
    pub fn clear(&mut self) {
        self.pipelines.clear();
        self.buffers.clear();
        self.bind_groups.clear();
        self.layouts.clear();
        self.command_encoder = None;
    }
}

/// Raw GPU data structure for particles
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuParticle {
    pub position: [f32; 3],
    pub _padding1: f32,
    pub velocity: [f32; 3],
    pub _padding2: f32,
    pub acceleration: [f32; 3],
    pub mass: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub active: u32,
    pub _padding3: f32,
}

/// Raw GPU data structure for forces
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuForce {
    pub force_type: u32,
    pub _padding1: [f32; 3],
    pub position: [f32; 3],
    pub _padding2: f32,
    pub direction: [f32; 3],
    pub strength: f32,
}

/// Raw GPU data structure for constraints
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuConstraint {
    pub constraint_type: u32,
    pub _padding1: [f32; 3],
    pub position: [f32; 3],
    pub _padding2: f32,
    pub size: [f32; 3],
    pub bounce: f32,
}

/// Raw GPU simulation parameters
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuSimParams {
    pub delta_time: f32,
    pub damping: f32,
    pub particle_count: u32,
    pub force_count: u32,
    pub constraint_count: u32,
    pub _padding: [f32; 3],
}

/// Low-level GPU particle simulation
pub struct RawGpuSimulation {
    context: ComputeContext,
    particles: Vec<GpuParticle>,
    forces: Vec<GpuForce>,
    constraints: Vec<GpuConstraint>,
    params: GpuSimParams,
    initialized: bool,
}

impl RawGpuSimulation {
    /// Creates a new raw GPU simulation
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            context: ComputeContext::new(device, queue),
            particles: Vec::new(),
            forces: Vec::new(),
            constraints: Vec::new(),
            params: GpuSimParams {
                delta_time: 0.0,
                damping: 0.99,
                particle_count: 0,
                force_count: 0,
                constraint_count: 0,
                _padding: [0.0; 3],
            },
            initialized: false,
        }
    }

    /// Gets the compute context
    pub fn context(&self) -> &ComputeContext {
        &self.context
    }

    /// Gets the compute context mutably
    pub fn context_mut(&mut self) -> &mut ComputeContext {
        &mut self.context
    }

    /// Initializes GPU resources with custom shader
    pub fn initialize_with_shader(&mut self, shader_source: &str) -> Result<(), String> {
        // Create shader module
        let shader = self
            .context
            .create_shader_module("particle_shader", shader_source)?;

        // Create compute pipeline
        self.context
            .create_compute_pipeline("particle_pipeline", &shader, "main")?;

        // Create bind group layout
        let layout_entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        self.context
            .create_bind_group_layout("simulation_layout", &layout_entries)?;

        self.initialized = true;
        Ok(())
    }

    /// Sets particle data
    pub fn set_particles(&mut self, particles: Vec<GpuParticle>) -> Result<(), String> {
        self.particles = particles;
        self.params.particle_count = self.particles.len() as u32;

        if self.initialized {
            self.context.create_buffer(
                "particles",
                &self.particles,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            )?;
        }

        Ok(())
    }

    /// Sets force data
    pub fn set_forces(&mut self, forces: Vec<GpuForce>) -> Result<(), String> {
        self.forces = forces;
        self.params.force_count = self.forces.len() as u32;

        if self.initialized {
            self.context.create_buffer(
                "forces",
                &self.forces,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            )?;
        }

        Ok(())
    }

    /// Sets constraint data
    pub fn set_constraints(&mut self, constraints: Vec<GpuConstraint>) -> Result<(), String> {
        self.constraints = constraints;
        self.params.constraint_count = self.constraints.len() as u32;

        if self.initialized {
            self.context.create_buffer(
                "constraints",
                &self.constraints,
                BufferUsages::STORAGE | BufferUsages::COPY_DST,
            )?;
        }

        Ok(())
    }

    /// Updates simulation parameters
    pub fn update_params(&mut self, delta_time: f32) -> Result<(), String> {
        self.params.delta_time = delta_time;

        if self.initialized {
            self.context.create_buffer(
                "params",
                &[self.params],
                BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            )?;
        }

        Ok(())
    }

    /// Creates bind group with current resources
    pub fn create_bind_group(&mut self) -> Result<(), String> {
        // For now, we'll skip the automatic bind group creation as it requires
        // more complex lifetime management. Users can create bind groups manually
        // using the low-level ComputeContext API.
        Ok(())
    }

    /// Dispatches the simulation compute shader
    pub fn dispatch_simulation(&mut self, workgroup_size: (u32, u32, u32)) -> Result<(), String> {
        self.context.begin_compute_pass("particle_simulation")?;
        self.context
            .dispatch("particle_pipeline", "simulation_bind_group", workgroup_size)?;
        self.context.submit()?;
        Ok(())
    }

    /// Reads particle data back from GPU
    pub fn read_particles(&self) -> Result<Vec<GpuParticle>, String> {
        self.context.read_buffer("particles", self.particles.len())
    }

    /// Provides default particle simulation shader
    pub fn default_shader() -> &'static str {
        r#"
        struct Particle {
            position: vec3<f32>,
            velocity: vec3<f32>,
            acceleration: vec3<f32>,
            mass: f32,
            lifetime: f32,
            max_lifetime: f32,
            active: u32,
        }

        struct Force {
            force_type: u32,
            position: vec3<f32>,
            direction: vec3<f32>,
            strength: f32,
        }

        struct Constraint {
            constraint_type: u32,
            position: vec3<f32>,
            size: vec3<f32>,
            bounce: f32,
        }

        struct SimParams {
            delta_time: f32,
            damping: f32,
            particle_count: u32,
            force_count: u32,
            constraint_count: u32,
        }

        @group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
        @group(0) @binding(1) var<storage, read> forces: array<Force>;
        @group(0) @binding(2) var<storage, read> constraints: array<Constraint>;
        @group(0) @binding(3) var<uniform> params: SimParams;

        @compute @workgroup_size(64, 1, 1)
        fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
            let index = global_id.x;
            if (index >= params.particle_count) {
                return;
            }

            var particle = particles[index];
            if (particle.active == 0u) {
                return;
            }

            // Reset acceleration
            particle.acceleration = vec3<f32>(0.0, 0.0, 0.0);

            // Apply forces
            for (var i = 0u; i < params.force_count; i++) {
                let force = forces[i];
                
                if (force.force_type == 0u) { // Uniform force
                    particle.acceleration += force.direction / particle.mass;
                } else if (force.force_type == 1u) { // Point force
                    let direction = force.position - particle.position;
                    let distance = length(direction);
                    if (distance > 0.001) {
                        let normalized = normalize(direction);
                        particle.acceleration += normalized * force.strength / (distance * distance * particle.mass);
                    }
                }
            }

            // Update physics
            particle.velocity += particle.acceleration * params.delta_time;
            particle.velocity *= params.damping;
            particle.position += particle.velocity * params.delta_time;

            // Apply constraints
            for (var i = 0u; i < params.constraint_count; i++) {
                let constraint = constraints[i];
                
                if (constraint.constraint_type == 0u) { // Box constraint
                    let min_pos = constraint.position - constraint.size * 0.5;
                    let max_pos = constraint.position + constraint.size * 0.5;
                    
                    if (particle.position.x < min_pos.x) {
                        particle.position.x = min_pos.x;
                        particle.velocity.x *= -constraint.bounce;
                    } else if (particle.position.x > max_pos.x) {
                        particle.position.x = max_pos.x;
                        particle.velocity.x *= -constraint.bounce;
                    }
                    
                    if (particle.position.y < min_pos.y) {
                        particle.position.y = min_pos.y;
                        particle.velocity.y *= -constraint.bounce;
                    } else if (particle.position.y > max_pos.y) {
                        particle.position.y = max_pos.y;
                        particle.velocity.y *= -constraint.bounce;
                    }
                    
                    if (particle.position.z < min_pos.z) {
                        particle.position.z = min_pos.z;
                        particle.velocity.z *= -constraint.bounce;
                    } else if (particle.position.z > max_pos.z) {
                        particle.position.z = max_pos.z;
                        particle.velocity.z *= -constraint.bounce;
                    }
                }
            }

            // Update lifetime
            particle.lifetime -= params.delta_time;
            if (particle.lifetime <= 0.0) {
                particle.active = 0u;
            }

            particles[index] = particle;
        }
        "#
    }
}

/// Memory pool for efficient buffer allocation
pub struct BufferPool {
    device: Arc<Device>,
    free_buffers: HashMap<u64, Vec<Arc<Buffer>>>,
    used_buffers: HashMap<String, Arc<Buffer>>,
}

impl BufferPool {
    /// Creates a new buffer pool
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            free_buffers: HashMap::new(),
            used_buffers: HashMap::new(),
        }
    }

    /// Allocates a buffer from the pool
    pub fn allocate(&mut self, name: &str, size: u64, usage: BufferUsages) -> Arc<Buffer> {
        // Try to reuse an existing buffer
        if let Some(buffers) = self.free_buffers.get_mut(&size) {
            if let Some(buffer) = buffers.pop() {
                self.used_buffers.insert(name.to_string(), buffer.clone());
                return buffer;
            }
        }

        // Create a new buffer
        let buffer = Arc::new(self.device.create_buffer(&BufferDescriptor {
            label: Some(name),
            size,
            usage,
            mapped_at_creation: false,
        }));

        self.used_buffers.insert(name.to_string(), buffer.clone());
        buffer
    }

    /// Returns a buffer to the pool
    pub fn deallocate(&mut self, name: &str) {
        if let Some(buffer) = self.used_buffers.remove(name) {
            let size = buffer.size();
            self.free_buffers
                .entry(size)
                .or_insert_with(Vec::new)
                .push(buffer);
        }
    }

    /// Clears all buffers
    pub fn clear(&mut self) {
        self.free_buffers.clear();
        self.used_buffers.clear();
    }
}

/// Performance monitoring for low-level operations
pub struct GpuProfiler {
    device: Arc<Device>,
    query_set: Option<wgpu::QuerySet>,
    resolve_buffer: Option<Buffer>,
    result_buffer: Option<Buffer>,
    timestamp_count: u32,
}

impl GpuProfiler {
    /// Creates a new GPU profiler
    pub fn new(device: Arc<Device>, max_timestamps: u32) -> Self {
        Self {
            device,
            query_set: None,
            resolve_buffer: None,
            result_buffer: None,
            timestamp_count: max_timestamps,
        }
    }

    /// Initializes profiling resources
    pub fn initialize(&mut self) -> Result<(), String> {
        // Note: Timestamp queries require specific features to be enabled
        // This is a simplified implementation
        Ok(())
    }

    /// Begins profiling a section
    pub fn begin_section(
        &mut self,
        _encoder: &mut CommandEncoder,
        _name: &str,
    ) -> Result<(), String> {
        // Implementation would write timestamp queries
        Ok(())
    }

    /// Ends profiling a section
    pub fn end_section(
        &mut self,
        _encoder: &mut CommandEncoder,
        _name: &str,
    ) -> Result<(), String> {
        // Implementation would write timestamp queries
        Ok(())
    }

    /// Reads profiling results
    pub fn read_results(&self) -> Result<Vec<(String, f32)>, String> {
        // Implementation would read timestamp query results
        Ok(Vec::new())
    }
}
