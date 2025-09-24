//! # Resource Validation
//!
//! Validation logic for GPU resource descriptors and configurations.

use super::types::{ValidationLevel, ValidationRule, ValidationIssue, ValidationSeverity, ValidationCategory};
use super::constraints::DeviceConstraints;
use crate::error::{HaggisError, HaggisResult};
use std::sync::atomic::{AtomicU64, Ordering};

/// Validator for GPU resource descriptors
pub struct ResourceValidator {
    /// Current validation level
    level: ValidationLevel,

    /// Device constraints for validation
    constraints: Option<DeviceConstraints>,

    /// Buffer-specific validator
    buffer_validator: BufferValidator,

    /// Texture-specific validator
    texture_validator: TextureValidator,

    /// Pipeline-specific validator
    pipeline_validator: PipelineValidator,

    /// Validation statistics
    stats: ValidationStats,
}

impl ResourceValidator {
    /// Create a new resource validator
    pub fn new(level: ValidationLevel) -> Self {
        Self {
            level,
            constraints: None,
            buffer_validator: BufferValidator::new(),
            texture_validator: TextureValidator::new(),
            pipeline_validator: PipelineValidator::new(),
            stats: ValidationStats::new(),
        }
    }

    /// Set the validation level
    pub fn set_level(&mut self, level: ValidationLevel) {
        self.level = level;
    }

    /// Set device constraints
    pub fn set_device_constraints(&mut self, constraints: &DeviceConstraints) {
        self.constraints = Some(constraints.clone());
        self.buffer_validator.set_constraints(constraints);
        self.texture_validator.set_constraints(constraints);
        self.pipeline_validator.set_constraints(constraints);
    }

    /// Validate a buffer descriptor
    pub fn validate_buffer_descriptor(
        &self,
        descriptor: &wgpu::BufferDescriptor<'_>,
    ) -> HaggisResult<()> {
        self.stats.buffer_validations.fetch_add(1, Ordering::Relaxed);

        let issues = self.buffer_validator.validate(descriptor, self.level);
        self.process_issues(issues)
    }

    /// Validate a texture descriptor
    pub fn validate_texture_descriptor(
        &self,
        descriptor: &wgpu::TextureDescriptor<'_>,
    ) -> HaggisResult<()> {
        self.stats.texture_validations.fetch_add(1, Ordering::Relaxed);

        let issues = self.texture_validator.validate(descriptor, self.level);
        self.process_issues(issues)
    }

    /// Validate a render pipeline descriptor
    pub fn validate_render_pipeline_descriptor(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor<'_>,
    ) -> HaggisResult<()> {
        self.stats.pipeline_validations.fetch_add(1, Ordering::Relaxed);

        let issues = self.pipeline_validator.validate_render_pipeline(descriptor, self.level);
        self.process_issues(issues)
    }

    /// Validate a compute pipeline descriptor
    pub fn validate_compute_pipeline_descriptor(
        &self,
        descriptor: &wgpu::ComputePipelineDescriptor<'_>,
    ) -> HaggisResult<()> {
        self.stats.pipeline_validations.fetch_add(1, Ordering::Relaxed);

        let issues = self.pipeline_validator.validate_compute_pipeline(descriptor, self.level);
        self.process_issues(issues)
    }

    /// Validate a bind group descriptor
    pub fn validate_bind_group_descriptor(
        &self,
        descriptor: &wgpu::BindGroupDescriptor<'_>,
    ) -> HaggisResult<()> {
        // Basic validation for now
        if descriptor.entries.is_empty() {
            let rule = ValidationRule::warning(
                "empty_bind_group",
                "Empty Bind Group",
                "Bind group has no entries",
                ValidationCategory::Usage,
            );
            let issue = ValidationIssue::new(rule, "Bind group contains no entries");
            self.process_issues(vec![issue])?;
        }

        Ok(())
    }

    /// Get validation statistics
    pub fn get_statistics(&self) -> super::ResourceValidationStats {
        super::ResourceValidationStats {
            buffer_validations: self.stats.buffer_validations.load(Ordering::Relaxed),
            texture_validations: self.stats.texture_validations.load(Ordering::Relaxed),
            pipeline_validations: self.stats.pipeline_validations.load(Ordering::Relaxed),
            error_count: self.stats.error_count.load(Ordering::Relaxed),
            warning_count: self.stats.warning_count.load(Ordering::Relaxed),
        }
    }

    fn process_issues(&self, issues: Vec<ValidationIssue>) -> HaggisResult<()> {
        let mut has_error = false;

        for issue in issues {
            match issue.severity() {
                ValidationSeverity::Error | ValidationSeverity::Critical => {
                    has_error = true;
                    self.stats.error_count.fetch_add(1, Ordering::Relaxed);
                    eprintln!("{}", issue);
                }
                ValidationSeverity::Warning => {
                    self.stats.warning_count.fetch_add(1, Ordering::Relaxed);
                    if self.level >= ValidationLevel::Standard {
                        println!("{}", issue);
                    }
                }
                ValidationSeverity::Info => {
                    if self.level >= ValidationLevel::Debug {
                        println!("{}", issue);
                    }
                }
            }
        }

        if has_error {
            Err(HaggisError::validation("Validation failed"))
        } else {
            Ok(())
        }
    }
}

/// Buffer validation logic
pub struct BufferValidator {
    constraints: Option<DeviceConstraints>,
}

impl BufferValidator {
    fn new() -> Self {
        Self { constraints: None }
    }

    fn set_constraints(&mut self, constraints: &DeviceConstraints) {
        self.constraints = Some(constraints.clone());
    }

    fn validate(
        &self,
        descriptor: &wgpu::BufferDescriptor<'_>,
        level: ValidationLevel,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check size constraints
        if let Some(constraints) = &self.constraints {
            if descriptor.size > constraints.max_buffer_size {
                let rule = ValidationRule::error(
                    "buffer_size_exceeded",
                    "Buffer Size Exceeded",
                    "Buffer size exceeds device limits",
                    ValidationCategory::Safety,
                );
                let issue = ValidationIssue::new(
                    rule,
                    format!(
                        "Buffer size {} exceeds maximum {}",
                        descriptor.size, constraints.max_buffer_size
                    ),
                ).with_suggestion("Use a smaller buffer or split into multiple buffers");
                issues.push(issue);
            }
        }

        // Check for empty buffers
        if descriptor.size == 0 {
            let rule = ValidationRule::warning(
                "empty_buffer",
                "Empty Buffer",
                "Buffer has zero size",
                ValidationCategory::Usage,
            );
            let issue = ValidationIssue::new(rule, "Buffer has zero size");
            issues.push(issue);
        }

        // Check for very large buffers
        if level >= ValidationLevel::Strict && descriptor.size > 100 * 1024 * 1024 {
            let rule = ValidationRule::warning(
                "large_buffer",
                "Large Buffer",
                "Buffer is very large and may impact performance",
                ValidationCategory::Performance,
            );
            let issue = ValidationIssue::new(
                rule,
                format!("Buffer size is {} MB", descriptor.size / 1024 / 1024),
            ).with_suggestion("Consider streaming data or using smaller batches");
            issues.push(issue);
        }

        // Check usage flags
        if descriptor.usage.is_empty() {
            let rule = ValidationRule::error(
                "no_buffer_usage",
                "No Buffer Usage",
                "Buffer has no usage flags set",
                ValidationCategory::Safety,
            );
            let issue = ValidationIssue::new(rule, "Buffer must have at least one usage flag");
            issues.push(issue);
        }

        // Check for conflicting usage
        if descriptor.usage.contains(wgpu::BufferUsages::MAP_READ)
            && descriptor.usage.contains(wgpu::BufferUsages::MAP_WRITE)
        {
            let rule = ValidationRule::warning(
                "conflicting_map_usage",
                "Conflicting Map Usage",
                "Buffer has both MAP_READ and MAP_WRITE flags",
                ValidationCategory::Performance,
            );
            let issue = ValidationIssue::new(rule, "Consider using separate buffers for read and write operations");
            issues.push(issue);
        }

        issues
    }
}

/// Texture validation logic
pub struct TextureValidator {
    constraints: Option<DeviceConstraints>,
}

impl TextureValidator {
    fn new() -> Self {
        Self { constraints: None }
    }

    fn set_constraints(&mut self, constraints: &DeviceConstraints) {
        self.constraints = Some(constraints.clone());
    }

    fn validate(
        &self,
        descriptor: &wgpu::TextureDescriptor<'_>,
        level: ValidationLevel,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check size constraints
        if let Some(constraints) = &self.constraints {
            let max_size = constraints.max_texture_size;
            if descriptor.size.width > max_size.0 || descriptor.size.height > max_size.1 {
                let rule = ValidationRule::error(
                    "texture_size_exceeded",
                    "Texture Size Exceeded",
                    "Texture dimensions exceed device limits",
                    ValidationCategory::Safety,
                );
                let issue = ValidationIssue::new(
                    rule,
                    format!(
                        "Texture size {}x{} exceeds maximum {}x{}",
                        descriptor.size.width,
                        descriptor.size.height,
                        max_size.0,
                        max_size.1
                    ),
                );
                issues.push(issue);
            }
        }

        // Check for zero-sized dimensions
        if descriptor.size.width == 0 || descriptor.size.height == 0 {
            let rule = ValidationRule::error(
                "zero_texture_dimension",
                "Zero Texture Dimension",
                "Texture has zero width or height",
                ValidationCategory::Safety,
            );
            let issue = ValidationIssue::new(rule, "Texture dimensions must be greater than zero");
            issues.push(issue);
        }

        // Check for very large textures
        if level >= ValidationLevel::Strict {
            let pixel_count = descriptor.size.width as u64 * descriptor.size.height as u64;
            if pixel_count > 16 * 1024 * 1024 {
                let rule = ValidationRule::warning(
                    "large_texture",
                    "Large Texture",
                    "Texture is very large and may impact performance",
                    ValidationCategory::Performance,
                );
                let issue = ValidationIssue::new(
                    rule,
                    format!(
                        "Texture has {} megapixels",
                        pixel_count / (1024 * 1024)
                    ),
                ).with_suggestion("Consider using mipmaps or texture compression");
                issues.push(issue);
            }
        }

        // Check usage flags
        if descriptor.usage.is_empty() {
            let rule = ValidationRule::error(
                "no_texture_usage",
                "No Texture Usage",
                "Texture has no usage flags set",
                ValidationCategory::Safety,
            );
            let issue = ValidationIssue::new(rule, "Texture must have at least one usage flag");
            issues.push(issue);
        }

        issues
    }
}

/// Pipeline validation logic
pub struct PipelineValidator {
    constraints: Option<DeviceConstraints>,
}

impl PipelineValidator {
    fn new() -> Self {
        Self { constraints: None }
    }

    fn set_constraints(&mut self, constraints: &DeviceConstraints) {
        self.constraints = Some(constraints.clone());
    }

    fn validate_render_pipeline(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor<'_>,
        _level: ValidationLevel,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for missing fragment shader (common mistake)
        if descriptor.fragment.is_none() {
            let rule = ValidationRule::warning(
                "no_fragment_shader",
                "No Fragment Shader",
                "Render pipeline has no fragment shader",
                ValidationCategory::Usage,
            );
            let issue = ValidationIssue::new(rule, "Consider adding a fragment shader for rendering")
                .with_suggestion("Add a fragment shader or use a depth-only pass");
            issues.push(issue);
        }

        issues
    }

    fn validate_compute_pipeline(
        &self,
        _descriptor: &wgpu::ComputePipelineDescriptor<'_>,
        _level: ValidationLevel,
    ) -> Vec<ValidationIssue> {
        let issues = Vec::new();
        // Compute pipeline validation would go here
        issues
    }
}

/// Validation statistics
struct ValidationStats {
    buffer_validations: AtomicU64,
    texture_validations: AtomicU64,
    pipeline_validations: AtomicU64,
    error_count: AtomicU64,
    warning_count: AtomicU64,
}

impl ValidationStats {
    fn new() -> Self {
        Self {
            buffer_validations: AtomicU64::new(0),
            texture_validations: AtomicU64::new(0),
            pipeline_validations: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            warning_count: AtomicU64::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_validation() {
        let validator = BufferValidator::new();

        // Test empty buffer
        let descriptor = wgpu::BufferDescriptor {
            label: Some("test"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        };

        let issues = validator.validate(&descriptor, ValidationLevel::Standard);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.rule.id == "empty_buffer"));

        // Test no usage flags
        let descriptor = wgpu::BufferDescriptor {
            label: Some("test"),
            size: 1024,
            usage: wgpu::BufferUsages::empty(),
            mapped_at_creation: false,
        };

        let issues = validator.validate(&descriptor, ValidationLevel::Standard);
        assert!(issues.iter().any(|i| i.rule.id == "no_buffer_usage"));
    }

    #[test]
    fn test_texture_validation() {
        let validator = TextureValidator::new();

        // Test zero dimensions
        let descriptor = wgpu::TextureDescriptor {
            label: Some("test"),
            size: wgpu::Extent3d {
                width: 0,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let issues = validator.validate(&descriptor, ValidationLevel::Standard);
        assert!(issues.iter().any(|i| i.rule.id == "zero_texture_dimension"));
    }

    #[test]
    fn test_resource_validator() {
        let mut validator = ResourceValidator::new(ValidationLevel::Standard);

        // Test buffer validation
        let descriptor = wgpu::BufferDescriptor {
            label: Some("test"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        };

        // Should return error due to zero size
        let result = validator.validate_buffer_descriptor(&descriptor);
        assert!(result.is_err() || validator.get_statistics().warning_count > 0);
    }
}