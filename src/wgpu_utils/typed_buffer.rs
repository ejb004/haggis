//! # Typed Buffer System
//!
//! Strongly-typed wgpu buffer abstractions that provide compile-time safety
//! and automatic size calculation with bytemuck integration.

use std::marker::PhantomData;
use std::mem;
use bytemuck::{Pod, cast_slice};
use wgpu::{Device, Queue, Buffer, BufferDescriptor, BufferUsages, util::{BufferInitDescriptor, DeviceExt}};
use crate::error::{HaggisResult, HaggisError};

/// A strongly-typed buffer wrapper that ensures type safety and automatic size calculation
pub struct TypedBuffer<T: Pod> {
    buffer: Buffer,
    capacity: usize,
    usage: BufferUsages,
    label: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T: Pod> TypedBuffer<T> {
    /// Create a new typed buffer with specified capacity
    pub fn new(
        device: &Device,
        capacity: usize,
        usage: BufferUsages,
        label: Option<&str>,
    ) -> HaggisResult<Self> {
        if capacity == 0 {
            return Err(HaggisError::validation_field(
                "Buffer capacity cannot be zero",
                "capacity",
                "> 0",
                "0"
            ));
        }

        let size = (capacity * mem::size_of::<T>()) as u64;

        // Validate that the size doesn't overflow
        if size > u64::MAX / 2 {
            return Err(HaggisError::memory(
                format!("Buffer size too large: {} bytes", size)
            ).with_suggestion("Reduce the capacity or use a smaller data type"));
        }

        let buffer = device.create_buffer(&BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: false,
        });

        Ok(Self {
            buffer,
            capacity,
            usage,
            label: label.map(|s| s.to_string()),
            _phantom: PhantomData,
        })
    }

    /// Create a typed buffer initialized with data
    pub fn new_with_data(
        device: &Device,
        data: &[T],
        usage: BufferUsages,
        label: Option<&str>,
    ) -> HaggisResult<Self> {
        if data.is_empty() {
            return Err(HaggisError::validation("Cannot create buffer with empty data"));
        }

        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: cast_slice(data),
            usage,
        });

        Ok(Self {
            buffer,
            capacity: data.len(),
            usage,
            label: label.map(|s| s.to_string()),
            _phantom: PhantomData,
        })
    }

    /// Write data to the buffer starting at the specified offset
    pub fn write(&self, queue: &Queue, offset: usize, data: &[T]) -> HaggisResult<()> {
        if offset + data.len() > self.capacity {
            return Err(HaggisError::buffer(
                format!(
                    "Write would exceed buffer capacity: offset {} + length {} > capacity {}",
                    offset, data.len(), self.capacity
                ),
                std::any::type_name::<T>(),
            ).with_suggestion("Ensure the write doesn't exceed buffer bounds"));
        }

        let byte_offset = (offset * mem::size_of::<T>()) as u64;
        queue.write_buffer(&self.buffer, byte_offset, cast_slice(data));

        Ok(())
    }

    /// Write a single element to the buffer at the specified index
    pub fn write_element(&self, queue: &Queue, index: usize, element: &T) -> HaggisResult<()> {
        if index >= self.capacity {
            return Err(HaggisError::buffer(
                format!("Index {} exceeds buffer capacity {}", index, self.capacity),
                std::any::type_name::<T>(),
            ));
        }

        self.write(queue, index, std::slice::from_ref(element))
    }

    /// Get the underlying wgpu buffer
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get the capacity of the buffer (number of elements)
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the size of the buffer in bytes
    pub fn size_bytes(&self) -> u64 {
        (self.capacity * mem::size_of::<T>()) as u64
    }

    /// Get the element size in bytes
    pub fn element_size() -> usize {
        mem::size_of::<T>()
    }

    /// Get the buffer usage flags
    pub fn usage(&self) -> BufferUsages {
        self.usage
    }

    /// Get the buffer label
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Check if the buffer supports reading (has MAP_READ usage)
    pub fn can_read(&self) -> bool {
        self.usage.contains(BufferUsages::MAP_READ)
    }

    /// Check if the buffer supports writing (has MAP_WRITE usage)
    pub fn can_write(&self) -> bool {
        self.usage.contains(BufferUsages::MAP_WRITE)
    }

    /// Resize the buffer to a new capacity (creates a new buffer)
    pub fn resize(&mut self, device: &Device, new_capacity: usize) -> HaggisResult<()> {
        if new_capacity == 0 {
            return Err(HaggisError::validation("Cannot resize buffer to zero capacity"));
        }

        let new_size = (new_capacity * mem::size_of::<T>()) as u64;

        let new_buffer = device.create_buffer(&BufferDescriptor {
            label: self.label.as_deref(),
            size: new_size,
            usage: self.usage,
            mapped_at_creation: false,
        });

        self.buffer = new_buffer;
        self.capacity = new_capacity;

        Ok(())
    }

    /// Create a copy command to copy data from another buffer
    pub fn copy_from_buffer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source: &TypedBuffer<T>,
        src_offset: usize,
        dst_offset: usize,
        element_count: usize,
    ) -> HaggisResult<()> {
        // Validation
        if src_offset + element_count > source.capacity {
            return Err(HaggisError::buffer(
                "Source copy range exceeds buffer capacity",
                std::any::type_name::<T>(),
            ));
        }

        if dst_offset + element_count > self.capacity {
            return Err(HaggisError::buffer(
                "Destination copy range exceeds buffer capacity",
                std::any::type_name::<T>(),
            ));
        }

        let element_size = mem::size_of::<T>() as u64;
        let src_byte_offset = (src_offset as u64) * element_size;
        let dst_byte_offset = (dst_offset as u64) * element_size;
        let copy_size = (element_count as u64) * element_size;

        encoder.copy_buffer_to_buffer(
            &source.buffer,
            src_byte_offset,
            &self.buffer,
            dst_byte_offset,
            copy_size,
        );

        Ok(())
    }
}

/// Builder for creating typed buffers with fluent API
pub struct TypedBufferBuilder<T: Pod> {
    capacity: Option<usize>,
    data: Option<Vec<T>>,
    usage: BufferUsages,
    label: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T: Pod> Default for TypedBufferBuilder<T> {
    fn default() -> Self {
        Self {
            capacity: None,
            data: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            label: None,
            _phantom: PhantomData,
        }
    }
}

impl<T: Pod> TypedBufferBuilder<T> {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the capacity (number of elements)
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    /// Initialize with data
    pub fn with_data(mut self, data: Vec<T>) -> Self {
        self.data = Some(data);
        self
    }

    /// Set buffer usage flags
    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    /// Add usage flags to existing usage
    pub fn add_usage(mut self, usage: BufferUsages) -> Self {
        self.usage |= usage;
        self
    }

    /// Set the buffer label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Mark as vertex buffer
    pub fn as_vertex_buffer(self) -> Self {
        self.add_usage(BufferUsages::VERTEX)
    }

    /// Mark as index buffer
    pub fn as_index_buffer(self) -> Self {
        self.add_usage(BufferUsages::INDEX)
    }

    /// Mark as uniform buffer
    pub fn as_uniform_buffer(self) -> Self {
        self.add_usage(BufferUsages::UNIFORM)
    }

    /// Mark as storage buffer
    pub fn as_storage_buffer(self) -> Self {
        self.add_usage(BufferUsages::STORAGE)
    }

    /// Mark as readable from CPU
    pub fn cpu_readable(self) -> Self {
        self.add_usage(BufferUsages::COPY_SRC | BufferUsages::MAP_READ)
    }

    /// Mark as writable from CPU
    pub fn cpu_writable(self) -> Self {
        self.add_usage(BufferUsages::COPY_DST | BufferUsages::MAP_WRITE)
    }

    /// Build the typed buffer
    pub fn build(self, device: &Device) -> HaggisResult<TypedBuffer<T>> {
        let label = self.label.as_deref();

        if let Some(data) = self.data {
            TypedBuffer::new_with_data(device, &data, self.usage, label)
        } else if let Some(capacity) = self.capacity {
            TypedBuffer::new(device, capacity, self.usage, label)
        } else {
            Err(HaggisError::validation(
                "Must specify either capacity or data for buffer creation"
            ).with_suggestion("Use with_capacity() or with_data()"))
        }
    }
}

/// Convenient type aliases for common buffer types
pub type VertexBuffer<T> = TypedBuffer<T>;
pub type IndexBuffer<T> = TypedBuffer<T>;
pub type UniformBuffer<T> = TypedBuffer<T>;
pub type StorageBuffer<T> = TypedBuffer<T>;

/// Helper functions for creating common buffer types
impl<T: Pod> TypedBuffer<T> {
    /// Create a vertex buffer
    pub fn vertex(device: &Device, data: &[T], label: Option<&str>) -> HaggisResult<Self> {
        Self::new_with_data(
            device,
            data,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            label
        )
    }

    /// Create an index buffer
    pub fn index(device: &Device, data: &[T], label: Option<&str>) -> HaggisResult<Self> {
        Self::new_with_data(
            device,
            data,
            BufferUsages::INDEX | BufferUsages::COPY_DST,
            label
        )
    }

    /// Create a uniform buffer
    pub fn uniform(device: &Device, data: &[T], label: Option<&str>) -> HaggisResult<Self> {
        Self::new_with_data(
            device,
            data,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            label
        )
    }

    /// Create a storage buffer
    pub fn storage(device: &Device, capacity: usize, label: Option<&str>) -> HaggisResult<Self> {
        Self::new(
            device,
            capacity,
            BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            label
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct TestVertex {
        position: [f32; 3],
        color: [f32; 3],
    }

    // Note: These tests would require a wgpu device, so they're mostly structure tests
    #[test]
    fn test_buffer_builder() {
        let builder = TypedBufferBuilder::<TestVertex>::new()
            .with_capacity(100)
            .as_vertex_buffer()
            .with_label("Test Buffer");

        assert!(builder.capacity.is_some());
        assert_eq!(builder.capacity.unwrap(), 100);
        assert!(builder.usage.contains(BufferUsages::VERTEX));
    }

    #[test]
    fn test_element_size() {
        assert_eq!(TypedBuffer::<f32>::element_size(), 4);
        assert_eq!(TypedBuffer::<TestVertex>::element_size(), 24); // 6 f32s
    }
}