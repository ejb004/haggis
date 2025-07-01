// src/wgpu_utils/uniform_buffer.rs - Enhanced with storage buffer support
use std::marker::PhantomData;

/// Generic buffer wrapper for uniform and storage buffers
pub struct UniformBuffer<Content> {
    buffer: wgpu::Buffer,
    content_type: PhantomData<Content>,
    previous_content: Vec<u8>,
}

impl<Content: bytemuck::Pod> UniformBuffer<Content> {
    fn name() -> &'static str {
        let type_name = std::any::type_name::<Content>();
        let pos = type_name.rfind(':').unwrap_or(0);
        if pos > 0 {
            &type_name[(pos + 1)..]
        } else {
            type_name
        }
    }

    /// Create a new uniform buffer
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<Content>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        UniformBuffer {
            buffer,
            content_type: PhantomData,
            previous_content: Vec::new(),
        }
    }

    /// Create a new storage buffer (for compute shaders)
    pub fn new_storage(device: &wgpu::Device, read_only: bool) -> Self {
        let usage = if read_only {
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        } else {
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
        };

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("StorageBuffer: {}", Self::name())),
            size: std::mem::size_of::<Content>() as u64,
            usage,
            mapped_at_creation: false,
        });

        UniformBuffer {
            buffer,
            content_type: PhantomData,
            previous_content: Vec::new(),
        }
    }

    /// Create buffer with initial data
    pub fn new_with_data(device: &wgpu::Device, initial_content: &Content) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<Content>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory
            .get_mapped_range_mut()
            .clone_from_slice(bytemuck::bytes_of(initial_content));
        buffer.unmap();

        UniformBuffer {
            buffer,
            content_type: PhantomData,
            previous_content: bytemuck::bytes_of(initial_content).to_vec(),
        }
    }

    /// Create storage buffer with initial data
    pub fn new_storage_with_data(
        device: &wgpu::Device,
        initial_content: &Content,
        read_only: bool,
    ) -> Self {
        let usage = if read_only {
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        } else {
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
        };

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("StorageBuffer: {}", Self::name())),
            size: std::mem::size_of::<Content>() as u64,
            usage,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory
            .get_mapped_range_mut()
            .clone_from_slice(bytemuck::bytes_of(initial_content));
        buffer.unmap();

        UniformBuffer {
            buffer,
            content_type: PhantomData,
            previous_content: bytemuck::bytes_of(initial_content).to_vec(),
        }
    }

    /// Update buffer content (optimized to skip unnecessary writes)
    pub fn update_content(&mut self, queue: &wgpu::Queue, content: Content) {
        let new_content = bytemuck::bytes_of(&content);
        if self.previous_content == new_content {
            return;
        }
        queue.write_buffer(&self.buffer, 0, new_content);
        self.previous_content = new_content.to_vec();
    }

    /// Force update buffer content (skips optimization check)
    pub fn force_update_content(&mut self, queue: &wgpu::Queue, content: Content) {
        let new_content = bytemuck::bytes_of(&content);
        queue.write_buffer(&self.buffer, 0, new_content);
        self.previous_content = new_content.to_vec();
    }

    /// Get binding resource
    pub fn binding_resource(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }

    /// Get the underlying buffer (useful for copying operations)
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get buffer size
    pub fn size(&self) -> u64 {
        self.buffer.size()
    }
}

/// Array buffer for handling multiple instances of the same type
pub struct ArrayBuffer<Content> {
    buffer: wgpu::Buffer,
    content_type: PhantomData<Content>,
    capacity: usize,
    current_size: usize,
}

impl<Content: bytemuck::Pod + Clone> ArrayBuffer<Content> {
    fn name() -> &'static str {
        let type_name = std::any::type_name::<Content>();
        let pos = type_name.rfind(':').unwrap_or(0);
        if pos > 0 {
            &type_name[(pos + 1)..]
        } else {
            type_name
        }
    }

    /// Create new array buffer with given capacity
    pub fn new(device: &wgpu::Device, capacity: usize, read_only: bool) -> Self {
        let usage = if read_only {
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        } else {
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
        };

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("ArrayBuffer<{}>", Self::name())),
            size: (capacity * std::mem::size_of::<Content>()) as u64,
            usage,
            mapped_at_creation: false,
        });

        ArrayBuffer {
            buffer,
            content_type: PhantomData,
            capacity,
            current_size: 0,
        }
    }

    /// Create new staging buffer for reading back GPU data
    pub fn new_staging(device: &wgpu::Device, capacity: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("StagingBuffer<{}>", Self::name())),
            size: (capacity * std::mem::size_of::<Content>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        ArrayBuffer {
            buffer,
            content_type: PhantomData,
            capacity,
            current_size: capacity,
        }
    }

    /// Create array buffer with initial data
    pub fn new_with_data(device: &wgpu::Device, data: &[Content], read_only: bool) -> Self {
        let usage = if read_only {
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        } else {
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC
        };

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("ArrayBuffer<{}>", Self::name())),
            size: (data.len() * std::mem::size_of::<Content>()) as u64,
            usage,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory
            .get_mapped_range_mut()
            .clone_from_slice(bytemuck::cast_slice(data));
        buffer.unmap();

        ArrayBuffer {
            buffer,
            content_type: PhantomData,
            capacity: data.len(),
            current_size: data.len(),
        }
    }

    /// Update array data
    pub fn update_data(&mut self, queue: &wgpu::Queue, data: &[Content]) {
        assert!(data.len() <= self.capacity, "Data exceeds buffer capacity");
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        self.current_size = data.len();
    }

    /// Get binding resource
    pub fn binding_resource(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }

    /// Get the underlying buffer
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get current number of elements
    pub fn len(&self) -> usize {
        self.current_size
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
