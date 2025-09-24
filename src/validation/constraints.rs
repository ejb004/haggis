//! # Validation Constraints
//!
//! Device limits and performance constraints for validation.

use wgpu::{Limits, Features};

/// Device-specific constraints for validation
#[derive(Debug, Clone)]
pub struct DeviceConstraints {
    /// Maximum buffer size in bytes
    pub max_buffer_size: u64,

    /// Maximum texture dimensions (width, height)
    pub max_texture_size: (u32, u32),

    /// Maximum number of bind group entries
    pub max_bind_group_entries: u32,

    /// Maximum number of vertex attributes
    pub max_vertex_attributes: u32,

    /// Maximum number of vertex buffers
    pub max_vertex_buffers: u32,

    /// Maximum uniform buffer size
    pub max_uniform_buffer_size: u64,

    /// Maximum storage buffer size
    pub max_storage_buffer_size: u64,

    /// Supported features
    pub features: Features,

    /// Device limits
    pub limits: Limits,
}

impl DeviceConstraints {
    /// Create constraints from a wgpu device
    pub fn from_device(limits: Limits, features: Features) -> Self {
        Self {
            max_buffer_size: limits.max_buffer_size,
            max_texture_size: (limits.max_texture_dimension_2d, limits.max_texture_dimension_2d),
            max_bind_group_entries: limits.max_bindings_per_bind_group,
            max_vertex_attributes: limits.max_vertex_attributes,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_uniform_buffer_size: limits.max_uniform_buffer_binding_size as u64,
            max_storage_buffer_size: limits.max_storage_buffer_binding_size as u64,
            features,
            limits,
        }
    }

    /// Check if a feature is supported
    pub fn supports_feature(&self, feature: Features) -> bool {
        self.features.contains(feature)
    }

    /// Get memory usage estimate for a buffer
    pub fn estimate_buffer_memory(&self, size: u64) -> u64 {
        // Add some overhead for alignment and metadata
        size + 256
    }

    /// Get memory usage estimate for a texture
    pub fn estimate_texture_memory(&self, width: u32, height: u32, format: wgpu::TextureFormat) -> u64 {
        let bytes_per_pixel = match format {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => 4,
            wgpu::TextureFormat::Rg8Unorm => 2,
            wgpu::TextureFormat::R8Unorm => 1,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            wgpu::TextureFormat::Depth32Float => 4,
            _ => 4, // Default estimate
        };

        (width as u64) * (height as u64) * bytes_per_pixel + 512 // Add overhead
    }
}

/// Performance constraints and guidelines
#[derive(Debug, Clone)]
pub struct PerformanceConstraints {
    /// Recommended maximum buffer size for optimal performance
    pub recommended_max_buffer_size: u64,

    /// Recommended maximum texture size for optimal performance
    pub recommended_max_texture_size: (u32, u32),

    /// Warning threshold for large buffer allocations
    pub large_buffer_threshold: u64,

    /// Warning threshold for large texture allocations
    pub large_texture_threshold: u64,

    /// Maximum recommended draw calls per frame
    pub max_draw_calls_per_frame: u32,

    /// Maximum recommended state changes per frame
    pub max_state_changes_per_frame: u32,

    /// Memory usage warning threshold (MB)
    pub memory_warning_threshold_mb: u64,
}

impl Default for PerformanceConstraints {
    fn default() -> Self {
        Self {
            recommended_max_buffer_size: 256 * 1024 * 1024, // 256 MB
            recommended_max_texture_size: (4096, 4096),
            large_buffer_threshold: 100 * 1024 * 1024, // 100 MB
            large_texture_threshold: 16 * 1024 * 1024, // 16 megapixels
            max_draw_calls_per_frame: 1000,
            max_state_changes_per_frame: 500,
            memory_warning_threshold_mb: 1024, // 1 GB
        }
    }
}

impl PerformanceConstraints {
    /// Create conservative constraints for mobile devices
    pub fn mobile() -> Self {
        Self {
            recommended_max_buffer_size: 64 * 1024 * 1024, // 64 MB
            recommended_max_texture_size: (2048, 2048),
            large_buffer_threshold: 32 * 1024 * 1024, // 32 MB
            large_texture_threshold: 4 * 1024 * 1024, // 4 megapixels
            max_draw_calls_per_frame: 200,
            max_state_changes_per_frame: 100,
            memory_warning_threshold_mb: 256, // 256 MB
        }
    }

    /// Create permissive constraints for desktop devices
    pub fn desktop() -> Self {
        Self {
            recommended_max_buffer_size: 1024 * 1024 * 1024, // 1 GB
            recommended_max_texture_size: (8192, 8192),
            large_buffer_threshold: 256 * 1024 * 1024, // 256 MB
            large_texture_threshold: 64 * 1024 * 1024, // 64 megapixels
            max_draw_calls_per_frame: 5000,
            max_state_changes_per_frame: 2000,
            memory_warning_threshold_mb: 4096, // 4 GB
        }
    }

    /// Check if a buffer size is considered large
    pub fn is_large_buffer(&self, size: u64) -> bool {
        size > self.large_buffer_threshold
    }

    /// Check if a texture is considered large
    pub fn is_large_texture(&self, width: u32, height: u32) -> bool {
        (width as u64) * (height as u64) > self.large_texture_threshold
    }

    /// Get performance recommendation for buffer size
    pub fn get_buffer_recommendation(&self, size: u64) -> Option<String> {
        if size > self.recommended_max_buffer_size {
            Some(format!(
                "Buffer size {} MB exceeds recommended maximum {} MB",
                size / (1024 * 1024),
                self.recommended_max_buffer_size / (1024 * 1024)
            ))
        } else if self.is_large_buffer(size) {
            Some(format!(
                "Buffer size {} MB is large and may impact performance",
                size / (1024 * 1024)
            ))
        } else {
            None
        }
    }

    /// Get performance recommendation for texture size
    pub fn get_texture_recommendation(&self, width: u32, height: u32) -> Option<String> {
        let max_size = self.recommended_max_texture_size;
        if width > max_size.0 || height > max_size.1 {
            Some(format!(
                "Texture size {}x{} exceeds recommended maximum {}x{}",
                width, height, max_size.0, max_size.1
            ))
        } else if self.is_large_texture(width, height) {
            Some(format!(
                "Texture size {}x{} is large and may impact performance",
                width, height
            ))
        } else {
            None
        }
    }
}

/// Combined validation constraints
#[derive(Debug, Clone)]
pub struct ValidationConstraints {
    /// Device-specific constraints
    pub device: Option<DeviceConstraints>,

    /// Performance guidelines
    pub performance: PerformanceConstraints,

    /// Whether to enforce strict validation
    pub strict_mode: bool,

    /// Whether to check for deprecated features
    pub check_deprecated: bool,
}

impl ValidationConstraints {
    /// Create new validation constraints
    pub fn new() -> Self {
        Self {
            device: None,
            performance: PerformanceConstraints::default(),
            strict_mode: false,
            check_deprecated: true,
        }
    }

    /// Set device constraints
    pub fn with_device(mut self, constraints: DeviceConstraints) -> Self {
        self.device = Some(constraints);
        self
    }

    /// Set performance constraints
    pub fn with_performance(mut self, constraints: PerformanceConstraints) -> Self {
        self.performance = constraints;
        self
    }

    /// Enable strict mode
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Configure for mobile devices
    pub fn mobile() -> Self {
        Self::new()
            .with_performance(PerformanceConstraints::mobile())
            .strict()
    }

    /// Configure for desktop devices
    pub fn desktop() -> Self {
        Self::new()
            .with_performance(PerformanceConstraints::desktop())
    }

    /// Check if a buffer allocation is valid
    pub fn validate_buffer_allocation(&self, size: u64) -> Result<(), String> {
        // Check device limits
        if let Some(device) = &self.device {
            if size > device.max_buffer_size {
                return Err(format!(
                    "Buffer size {} exceeds device limit {}",
                    size, device.max_buffer_size
                ));
            }
        }

        // Check performance constraints in strict mode
        if self.strict_mode {
            if let Some(recommendation) = self.performance.get_buffer_recommendation(size) {
                return Err(recommendation);
            }
        }

        Ok(())
    }

    /// Check if a texture allocation is valid
    pub fn validate_texture_allocation(&self, width: u32, height: u32) -> Result<(), String> {
        // Check device limits
        if let Some(device) = &self.device {
            let max_size = device.max_texture_size;
            if width > max_size.0 || height > max_size.1 {
                return Err(format!(
                    "Texture size {}x{} exceeds device limit {}x{}",
                    width, height, max_size.0, max_size.1
                ));
            }
        }

        // Check performance constraints in strict mode
        if self.strict_mode {
            if let Some(recommendation) = self.performance.get_texture_recommendation(width, height) {
                return Err(recommendation);
            }
        }

        Ok(())
    }
}

impl Default for ValidationConstraints {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_constraints() {
        let limits = Limits::default();
        let features = Features::empty();
        let constraints = DeviceConstraints::from_device(limits, features);

        assert_eq!(constraints.max_buffer_size, limits.max_buffer_size);
        assert!(!constraints.supports_feature(Features::DEPTH_CLIP_CONTROL));
    }

    #[test]
    fn test_performance_constraints() {
        let constraints = PerformanceConstraints::default();

        assert!(constraints.is_large_buffer(200 * 1024 * 1024));
        assert!(!constraints.is_large_buffer(10 * 1024 * 1024));

        assert!(constraints.is_large_texture(8192, 8192));
        assert!(!constraints.is_large_texture(512, 512));
    }

    #[test]
    fn test_validation_constraints() {
        let constraints = ValidationConstraints::new()
            .with_performance(PerformanceConstraints::mobile())
            .strict();

        // Should fail in strict mode for large buffers
        let result = constraints.validate_buffer_allocation(100 * 1024 * 1024);
        assert!(result.is_err());

        // Should pass for small buffers
        let result = constraints.validate_buffer_allocation(1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_estimates() {
        let limits = Limits::default();
        let features = Features::empty();
        let constraints = DeviceConstraints::from_device(limits, features);

        let buffer_memory = constraints.estimate_buffer_memory(1024);
        assert!(buffer_memory > 1024); // Should include overhead

        let texture_memory = constraints.estimate_texture_memory(512, 512, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(texture_memory, 512 * 512 * 4 + 512); // 4 bytes per pixel + overhead
    }
}