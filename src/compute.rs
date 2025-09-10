//! # GPU Compute Abstraction Layer
//!
//! Simplified interface for GPU compute operations, hiding wgpu complexity while 
//! providing automatic buffer management and common compute patterns.

use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor,
    BufferUsages, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device,
    PipelineLayoutDescriptor, Queue, ShaderModule, ShaderModuleDescriptor, ShaderSource, 
    ShaderStages, CommandEncoder,
};

use crate::builder::{Builder, CommonConfig, ConfigurableBuilder, ExecutionHint};

/// High-level GPU compute abstraction
pub struct ComputeEngine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipelines: HashMap<String, Arc<ComputePipeline>>,
    buffers: HashMap<String, ComputeBuffer>,
    bind_groups: HashMap<String, BindGroup>,
}

/// Managed compute buffer with automatic sizing
pub struct ComputeBuffer {
    buffer: Buffer,
    size: wgpu::BufferAddress,
    usage: BufferUsages,
    element_count: u32,
}

/// Buffer configuration for automatic management
#[derive(Debug, Clone)]
pub struct BufferConfig {
    pub name: String,
    pub element_size: u32,
    pub element_count: u32,
    pub usage: BufferUsages,
    pub initial_data: Option<Vec<u8>>,
}

/// Buffer access mode for compute pipeline bindings
#[derive(Debug, Clone)]
pub enum BufferAccessMode {
    ReadOnly,
    ReadWrite,
}

/// Buffer binding configuration
#[derive(Debug, Clone)]
pub struct BufferBinding {
    pub name: String,
    pub access: BufferAccessMode,
}

/// Compute pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub name: String,
    pub shader_source: String,
    pub entry_point: String,
    pub workgroup_size: [u32; 3],
    pub buffer_bindings: Vec<BufferBinding>,
}

/// Builder for compute operations
pub struct ComputeBuilder {
    common: CommonConfig,
    buffers: Vec<BufferConfig>,
    pipeline: Option<PipelineConfig>,
    dispatch_size: Option<[u32; 3]>,
}

impl ComputeEngine {
    /// Create new compute engine
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            pipelines: HashMap::new(),
            buffers: HashMap::new(),
            bind_groups: HashMap::new(),
        }
    }

    /// Create buffer with automatic management
    pub fn create_buffer(&mut self, config: BufferConfig) -> Result<(), String> {
        let size = (config.element_size * config.element_count) as wgpu::BufferAddress;
        
        let buffer = if let Some(data) = &config.initial_data {
            if data.len() != size as usize {
                return Err(format!(
                    "Initial data size {} doesn't match buffer size {}",
                    data.len(),
                    size
                ));
            }
            self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&config.name),
                contents: data,
                usage: config.usage,
            })
        } else {
            self.device.create_buffer(&BufferDescriptor {
                label: Some(&config.name),
                size,
                usage: config.usage,
                mapped_at_creation: false,
            })
        };

        let compute_buffer = ComputeBuffer {
            buffer,
            size,
            usage: config.usage,
            element_count: config.element_count,
        };

        self.buffers.insert(config.name.clone(), compute_buffer);
        Ok(())
    }

    /// Create compute pipeline from shader
    pub fn create_pipeline(&mut self, config: PipelineConfig) -> Result<(), String> {
        let shader_label = format!("{}_shader", config.name);
        let shader = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&shader_label),
            source: ShaderSource::Wgsl(config.shader_source.into()),
        });

        // Create bind group layout based on buffer bindings
        let mut entries = Vec::new();
        for (i, buffer_binding) in config.buffer_bindings.iter().enumerate() {
            if !self.buffers.contains_key(&buffer_binding.name) {
                return Err(format!("Buffer '{}' not found", buffer_binding.name));
            }
            
            let read_only = matches!(buffer_binding.access, BufferAccessMode::ReadOnly);
            
            entries.push(BindGroupLayoutEntry {
                binding: i as u32,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }

        let bind_group_layout_label = format!("{}_bind_group_layout", config.name);
        let bind_group_layout = self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&bind_group_layout_label),
            entries: &entries,
        });

        let pipeline_layout_label = format!("{}_pipeline_layout", config.name);
        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&pipeline_layout_label),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = self.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(&config.name),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(&config.entry_point),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create bind group for this pipeline
        let mut bind_entries = Vec::new();
        for (i, buffer_binding) in config.buffer_bindings.iter().enumerate() {
            let buffer = &self.buffers[&buffer_binding.name];
            bind_entries.push(BindGroupEntry {
                binding: i as u32,
                resource: buffer.buffer.as_entire_binding(),
            });
        }

        let bind_group_label = format!("{}_bind_group", config.name);
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some(&bind_group_label),
            layout: &bind_group_layout,
            entries: &bind_entries,
        });

        self.pipelines.insert(config.name.clone(), Arc::new(pipeline));
        self.bind_groups.insert(config.name.clone(), bind_group);

        Ok(())
    }

    /// Dispatch compute operation
    pub fn dispatch(&self, pipeline_name: &str, workgroups: [u32; 3]) -> Result<(), String> {
        let pipeline = self.pipelines.get(pipeline_name)
            .ok_or_else(|| format!("Pipeline '{}' not found", pipeline_name))?;
        
        let bind_group = self.bind_groups.get(pipeline_name)
            .ok_or_else(|| format!("Bind group for '{}' not found", pipeline_name))?;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute_encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute_pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
        }

        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Get buffer for reading results
    pub fn get_buffer(&self, name: &str) -> Option<&Buffer> {
        self.buffers.get(name).map(|cb| &cb.buffer)
    }

    /// Update buffer data
    pub fn update_buffer<T: Pod>(&self, name: &str, data: &[T]) -> Result<(), String> {
        let buffer = self.buffers.get(name)
            .ok_or_else(|| format!("Buffer '{}' not found", name))?;
        
        let byte_data = bytemuck::cast_slice(data);
        if byte_data.len() > buffer.size as usize {
            return Err(format!(
                "Data size {} exceeds buffer size {}",
                byte_data.len(),
                buffer.size
            ));
        }

        self.queue.write_buffer(&buffer.buffer, 0, byte_data);
        Ok(())
    }
}

impl ComputeBuilder {
    /// Create new compute builder
    pub fn new() -> Self {
        Self {
            common: CommonConfig::default(),
            buffers: Vec::new(),
            pipeline: None,
            dispatch_size: None,
        }
    }

    /// Add buffer to compute operation
    pub fn with_buffer(mut self, config: BufferConfig) -> Self {
        self.buffers.push(config);
        self
    }

    /// Set compute pipeline configuration
    pub fn with_pipeline(mut self, config: PipelineConfig) -> Self {
        self.pipeline = Some(config);
        self
    }

    /// Set dispatch workgroup count
    pub fn with_dispatch(mut self, workgroups: [u32; 3]) -> Self {
        self.dispatch_size = Some(workgroups);
        self
    }

    /// Add storage buffer (most common case)
    pub fn with_storage_buffer<T: Pod>(
        mut self,
        name: impl Into<String>,
        data: &[T],
    ) -> Self {
        let name = name.into();
        let element_size = std::mem::size_of::<T>() as u32;
        let element_count = data.len() as u32;
        let initial_data = Some(bytemuck::cast_slice(data).to_vec());

        let config = BufferConfig {
            name,
            element_size,
            element_count,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            initial_data,
        };

        self.buffers.push(config);
        self
    }

    /// Add empty storage buffer
    pub fn with_empty_buffer<T: Pod>(
        mut self,
        name: impl Into<String>,
        count: u32,
    ) -> Self {
        let name = name.into();
        let element_size = std::mem::size_of::<T>() as u32;

        let config = BufferConfig {
            name,
            element_size,
            element_count: count,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            initial_data: None,
        };

        self.buffers.push(config);
        self
    }
}

impl Builder<ComputeOperation> for ComputeBuilder {
    fn build(self) -> ComputeOperation {
        ComputeOperation {
            common: self.common,
            buffers: self.buffers,
            pipeline: self.pipeline.expect("Pipeline configuration required"),
            dispatch_size: self.dispatch_size.unwrap_or([1, 1, 1]),
        }
    }
}

impl ConfigurableBuilder<ComputeOperation> for ComputeBuilder {
    fn merge(mut self, other: Self) -> Self {
        self.buffers.extend(other.buffers);
        if other.pipeline.is_some() {
            self.pipeline = other.pipeline;
        }
        if other.dispatch_size.is_some() {
            self.dispatch_size = other.dispatch_size;
        }
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.pipeline.is_none() {
            return Err("Pipeline configuration is required".to_string());
        }
        if self.buffers.is_empty() {
            return Err("At least one buffer is required".to_string());
        }
        Ok(())
    }
}

/// Built compute operation ready for execution
pub struct ComputeOperation {
    pub common: CommonConfig,
    pub buffers: Vec<BufferConfig>,
    pub pipeline: PipelineConfig,
    pub dispatch_size: [u32; 3],
}

impl ComputeOperation {
    /// Execute this compute operation on the given engine
    pub fn execute(&self, engine: &mut ComputeEngine) -> Result<(), String> {
        // Create buffers
        for buffer_config in &self.buffers {
            engine.create_buffer(buffer_config.clone())?;
        }

        // Create pipeline
        engine.create_pipeline(self.pipeline.clone())?;

        // Dispatch
        engine.dispatch(&self.pipeline.name, self.dispatch_size)?;

        Ok(())
    }
}

// Implement common builder methods
crate::impl_common_builder_methods!(ComputeBuilder);

/// Convenience macro for creating compute shaders
#[macro_export]
macro_rules! compute_shader {
    ($name:expr, $source:expr, $entry:expr, $workgroup:expr) => {
        crate::compute::PipelineConfig {
            name: $name.to_string(),
            shader_source: $source.to_string(),
            entry_point: $entry.to_string(),
            workgroup_size: $workgroup,
            buffer_bindings: Vec::new(),
        }
    };
    ($name:expr, $source:expr, $entry:expr, $workgroup:expr, $buffers:expr) => {
        crate::compute::PipelineConfig {
            name: $name.to_string(),
            shader_source: $source.to_string(),
            entry_point: $entry.to_string(),
            workgroup_size: $workgroup,
            buffer_bindings: $buffers.iter().map(|(name, access)| crate::compute::BufferBinding {
                name: name.to_string(),
                access: *access,
            }).collect(),
        }
    };
}