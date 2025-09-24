//! # Resource Management System
//!
//! Centralized GPU resource lifecycle management for the Haggis framework.
//! Provides efficient resource pooling, automatic cleanup, and reference counting.
//!
//! ## Architecture
//!
//! The resource system is built around three key components:
//!
//! - **ResourceManager** - Central registry for all GPU resources with lifecycle management
//! - **ResourcePool<T>** - Object pooling for frequently allocated/deallocated resources
//! - **ResourceHandle<T>** - Smart pointer with automatic reference counting and cleanup
//!
//! ## Features
//!
//! - **Automatic Cleanup** - Resources are automatically freed when no longer referenced
//! - **Object Pooling** - Reuse buffers and textures to reduce allocation overhead
//! - **Reference Counting** - Track usage and prevent premature deallocation
//! - **Type Safety** - Strongly-typed handles prevent resource type confusion
//! - **Performance Monitoring** - Track resource usage and allocation patterns
//!
//! ## Usage
//!
//! ```no_run
//! use haggis::resources::{ResourceManager, ResourceHandle};
//! use wgpu::{Device, Buffer};
//!
//! let manager = ResourceManager::new();
//!
//! // Create a managed buffer
//! let buffer_handle: ResourceHandle<Buffer> = manager.create_buffer(
//!     &device,
//!     &wgpu::BufferDescriptor {
//!         label: Some("vertex_buffer"),
//!         size: 1024,
//!         usage: wgpu::BufferUsages::VERTEX,
//!         mapped_at_creation: false,
//!     }
//! ).await?;
//!
//! // Use the buffer (automatically derefs to &Buffer)
//! render_pass.set_vertex_buffer(0, buffer_handle.slice(..));
//!
//! // Buffer is automatically cleaned up when handle is dropped
//! ```

pub mod manager;
pub mod handle;
pub mod pool;

// Re-export main types for convenience
pub use manager::ResourceManager;
pub use handle::{ResourceHandle, WeakResourceHandle};
pub use pool::{ResourcePool, PooledResource};

use crate::error::{HaggisError, HaggisResult};
use std::sync::Arc;
use wgpu::{Device, Queue};

/// Trait for resources that can be managed by the ResourceManager
pub trait ManagedResource: Send + Sync + 'static {
    /// Get a human-readable name for this resource type
    fn resource_type_name() -> &'static str;

    /// Get the estimated memory usage of this resource in bytes
    fn estimated_size(&self) -> u64;

    /// Check if this resource can be pooled (reused)
    fn is_poolable(&self) -> bool {
        false
    }

    /// Reset this resource to a clean state for pooling
    /// Only called if is_poolable() returns true
    fn reset(&mut self) -> HaggisResult<()> {
        Ok(())
    }
}

/// Configuration for resource management behavior
#[derive(Debug, Clone)]
pub struct ResourceConfig {
    /// Enable object pooling for frequently allocated resources
    pub enable_pooling: bool,

    /// Maximum number of resources to keep in each pool
    pub max_pool_size: usize,

    /// Enable performance monitoring and metrics collection
    pub enable_metrics: bool,

    /// Automatically cleanup unused resources after this duration (in seconds)
    pub cleanup_interval: f32,

    /// Maximum total memory usage before forcing cleanup (in bytes)
    pub max_memory_usage: Option<u64>,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            enable_pooling: true,
            max_pool_size: 64,
            enable_metrics: false,
            cleanup_interval: 5.0,
            max_memory_usage: None,
        }
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceMetrics {
    /// Total number of resources currently allocated
    pub total_resources: usize,

    /// Total estimated memory usage in bytes
    pub total_memory_bytes: u64,

    /// Number of resources currently in pools
    pub pooled_resources: usize,

    /// Number of pool hits (reused resources)
    pub pool_hits: u64,

    /// Number of pool misses (new allocations)
    pub pool_misses: u64,

    /// Number of automatic cleanups performed
    pub cleanup_count: u64,
}

impl ResourceMetrics {
    /// Calculate the pool hit rate as a percentage
    pub fn pool_hit_rate(&self) -> f32 {
        if self.pool_hits + self.pool_misses == 0 {
            0.0
        } else {
            (self.pool_hits as f32) / ((self.pool_hits + self.pool_misses) as f32) * 100.0
        }
    }

    /// Format memory usage in a human-readable way
    pub fn format_memory_usage(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = self.total_memory_bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

// Implement ManagedResource for common wgpu types
impl ManagedResource for wgpu::Buffer {
    fn resource_type_name() -> &'static str {
        "Buffer"
    }

    fn estimated_size(&self) -> u64 {
        self.size()
    }

    fn is_poolable(&self) -> bool {
        // Buffers can be pooled if they don't have special usage flags
        let usage = self.usage();
        !usage.contains(wgpu::BufferUsages::MAP_READ) &&
        !usage.contains(wgpu::BufferUsages::MAP_WRITE)
    }
}

impl ManagedResource for wgpu::Texture {
    fn resource_type_name() -> &'static str {
        "Texture"
    }

    fn estimated_size(&self) -> u64 {
        // Rough estimation based on dimensions and format
        let size = self.size();
        let bytes_per_pixel = match self.format() {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::Rg8Unorm => 2,
            wgpu::TextureFormat::R8Unorm => 1,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            wgpu::TextureFormat::Depth32Float => 4,
            _ => 4, // Default estimate
        };

        (size.width as u64) * (size.height as u64) * (size.depth_or_array_layers as u64) * bytes_per_pixel
    }

    fn is_poolable(&self) -> bool {
        // Textures can be pooled if they're render targets or storage
        let usage = self.usage();
        usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) ||
        usage.contains(wgpu::TextureUsages::STORAGE_BINDING)
    }
}

impl ManagedResource for wgpu::BindGroup {
    fn resource_type_name() -> &'static str {
        "BindGroup"
    }

    fn estimated_size(&self) -> u64 {
        // Bind groups are lightweight, estimate 64 bytes
        64
    }
}

impl ManagedResource for wgpu::RenderPipeline {
    fn resource_type_name() -> &'static str {
        "RenderPipeline"
    }

    fn estimated_size(&self) -> u64 {
        // Pipelines can be large due to shader compilation, estimate 4KB
        4096
    }
}

impl ManagedResource for wgpu::ComputePipeline {
    fn resource_type_name() -> &'static str {
        "ComputePipeline"
    }

    fn estimated_size(&self) -> u64 {
        // Similar to render pipelines
        4096
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_config_default() {
        let config = ResourceConfig::default();
        assert!(config.enable_pooling);
        assert_eq!(config.max_pool_size, 64);
        assert!(!config.enable_metrics);
    }

    #[test]
    fn test_resource_metrics() {
        let mut metrics = ResourceMetrics::default();
        metrics.pool_hits = 8;
        metrics.pool_misses = 2;
        metrics.total_memory_bytes = 1024 * 1024; // 1MB

        assert_eq!(metrics.pool_hit_rate(), 80.0);
        assert_eq!(metrics.format_memory_usage(), "1.00 MB");
    }

    #[test]
    fn test_memory_format() {
        let mut metrics = ResourceMetrics::default();

        metrics.total_memory_bytes = 512;
        assert_eq!(metrics.format_memory_usage(), "512.00 B");

        metrics.total_memory_bytes = 1536; // 1.5 KB
        assert_eq!(metrics.format_memory_usage(), "1.50 KB");

        metrics.total_memory_bytes = 2 * 1024 * 1024; // 2 MB
        assert_eq!(metrics.format_memory_usage(), "2.00 MB");
    }
}