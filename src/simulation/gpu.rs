//! GPU simulation utilities and base types
//!
//! Provides compute shader infrastructure for GPU-based simulations

use std::marker::PhantomData;
use wgpu::{BindGroup, Buffer, ComputePipeline, Device, Queue};

/// Base struct for GPU simulations with common compute infrastructure
pub struct GpuSimulationBase<T: bytemuck::Pod + bytemuck::Zeroable> {
    pub name: String,
    pub running: bool,
    pub gpu_ready: bool,

    // Compute pipeline and resources
    pub compute_pipeline: Option<ComputePipeline>,
    pub bind_group: Option<BindGroup>,
    pub data_buffer: Option<Buffer>,
    pub uniform_buffer: Option<Buffer>,

    // Simulation parameters
    pub workgroup_size: (u32, u32, u32),
    pub dispatch_size: (u32, u32, u32),

    _phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> GpuSimulationBase<T> {
    pub fn new(name: String, workgroup_size: (u32, u32, u32)) -> Self {
        Self {
            name,
            running: false,
            gpu_ready: false,
            compute_pipeline: None,
            bind_group: None,
            data_buffer: None,
            uniform_buffer: None,
            workgroup_size,
            dispatch_size: (1, 1, 1),
            _phantom: PhantomData,
        }
    }

    /// Helper to create compute pipeline from shader source
    pub fn create_compute_pipeline(
        device: &Device,
        shader_source: &str,
        entry_point: &str,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> ComputePipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    /// Helper to create storage buffer
    pub fn create_storage_buffer(device: &Device, size: u64, label: &str) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Helper to create uniform buffer
    pub fn create_uniform_buffer(device: &Device, size: u64, label: &str) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Execute compute shader
    pub fn dispatch_compute(&self, device: &Device, queue: &Queue) {
        if let Some(pipeline) = &self.compute_pipeline {
            if let Some(bind_group) = &self.bind_group {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Encoder"),
                });

                {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute Pass"),
                            timestamp_writes: None,
                        });

                    compute_pass.set_pipeline(pipeline);
                    compute_pass.set_bind_group(0, bind_group, &[]);
                    compute_pass.dispatch_workgroups(
                        self.dispatch_size.0,
                        self.dispatch_size.1,
                        self.dispatch_size.2,
                    );
                }

                queue.submit(std::iter::once(encoder.finish()));
            }
        }
    }

    /// Update uniform buffer data
    pub fn update_uniforms<U: bytemuck::Pod>(&self, queue: &Queue, data: &U) {
        if let Some(buffer) = &self.uniform_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[*data]));
        }
    }

    /// Set dispatch size based on problem size
    pub fn set_dispatch_size(&mut self, problem_size: (u32, u32, u32)) {
        self.dispatch_size = (
            (problem_size.0 + self.workgroup_size.0 - 1) / self.workgroup_size.0,
            (problem_size.1 + self.workgroup_size.1 - 1) / self.workgroup_size.1,
            (problem_size.2 + self.workgroup_size.2 - 1) / self.workgroup_size.2,
        );
    }
}

/// Uniform data structure for common simulation parameters
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationUniforms {
    pub delta_time: f32,
    pub total_time: f32,
    pub particle_count: u32,
    pub _padding: u32,
}

/// Trait for data that can be uploaded to GPU buffers
pub trait GpuData: bytemuck::Pod + bytemuck::Zeroable + Copy {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    fn as_bytes(&self) -> Vec<u8> {
        bytemuck::cast_slice(&[*self]).to_vec()
    }
}

/// Helper macro to implement GpuData for structs
#[macro_export]
macro_rules! impl_gpu_data {
    ($type:ty) => {
        impl GpuData for $type {}
    };
}

// Example usage in simulation manager update:
pub fn update_with_gpu_support(
    simulation_manager: &mut crate::simulation::manager::SimulationManager,
    delta_time: f32,
    scene: &mut crate::gfx::scene::Scene,
    device: Option<&Device>,
    queue: Option<&Queue>,
) {
    // The simulation manager now handles both CPU and GPU updates internally
    simulation_manager.update(delta_time, scene, device, queue);
}
