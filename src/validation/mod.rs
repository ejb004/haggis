//! # GPU Validation Framework
//!
//! Comprehensive validation system for GPU operations in the Haggis framework.
//! Provides both compile-time and runtime validation for resource creation,
//! operation parameters, and performance constraints.
//!
//! ## Features
//!
//! - **Resource Validation** - Ensure buffer sizes, texture dimensions, and usage flags are valid
//! - **Performance Constraints** - Check against device limits and performance guidelines
//! - **Type Safety** - Compile-time validation for resource compatibility
//! - **Runtime Checks** - Dynamic validation with detailed error reporting
//! - **Configuration** - Configurable validation levels for different build types
//!
//! ## Validation Levels
//!
//! - **None** - No validation (release builds)
//! - **Basic** - Essential safety checks only
//! - **Standard** - Recommended validation for development
//! - **Strict** - Comprehensive validation with performance warnings
//! - **Debug** - Full validation with detailed diagnostics
//!
//! ## Usage
//!
//! ```no_run
//! use haggis::validation::{Validator, ValidationLevel};
//! use wgpu::{BufferDescriptor, BufferUsages};
//!
//! let validator = Validator::new(ValidationLevel::Standard);
//!
//! // Validate buffer descriptor before creation
//! let descriptor = BufferDescriptor {
//!     label: Some("vertex_buffer"),
//!     size: 1024,
//!     usage: BufferUsages::VERTEX,
//!     mapped_at_creation: false,
//! };
//!
//! validator.validate_buffer_descriptor(&descriptor, &device.limits())?;
//! ```

pub mod constraints;
pub mod resource;
pub mod runtime;
pub mod types;

// Re-export main types for convenience
pub use constraints::{DeviceConstraints, PerformanceConstraints, ValidationConstraints};
pub use resource::{ResourceValidator, BufferValidator, TextureValidator, PipelineValidator};
pub use runtime::{RuntimeValidator, ValidationContext, ValidationResult};
pub use types::{ValidationLevel, ValidationRule, ValidationSeverity};

use crate::error::{HaggisError, HaggisResult};
use wgpu::{Device, Limits, Features};
use std::sync::Arc;

/// Main validation entry point for the Haggis framework
///
/// The Validator provides a unified interface for all GPU validation operations.
/// It can be configured with different validation levels and constraints to
/// match the requirements of different build types and deployment scenarios.
pub struct Validator {
    /// Current validation level
    level: ValidationLevel,

    /// Resource-specific validators
    resource_validator: ResourceValidator,

    /// Runtime validation system
    runtime_validator: RuntimeValidator,

    /// Device constraints and limits
    constraints: Option<DeviceConstraints>,

    /// Performance guidelines
    performance: PerformanceConstraints,
}

impl Validator {
    /// Create a new validator with the specified level
    pub fn new(level: ValidationLevel) -> Self {
        Self {
            level,
            resource_validator: ResourceValidator::new(level),
            runtime_validator: RuntimeValidator::new(level),
            constraints: None,
            performance: PerformanceConstraints::default(),
        }
    }

    /// Create a validator with device-specific constraints
    pub fn with_device(level: ValidationLevel, device: &Device) -> Self {
        let mut validator = Self::new(level);
        validator.set_device_constraints(device);
        validator
    }

    /// Set device constraints based on actual device capabilities
    pub fn set_device_constraints(&mut self, device: &Device) {
        let limits = device.limits();
        let features = device.features();
        self.constraints = Some(DeviceConstraints::from_device(limits, features));
        self.resource_validator.set_device_constraints(&self.constraints.as_ref().unwrap());
    }

    /// Get the current validation level
    pub fn level(&self) -> ValidationLevel {
        self.level
    }

    /// Set the validation level
    pub fn set_level(&mut self, level: ValidationLevel) {
        self.level = level;
        self.resource_validator.set_level(level);
        self.runtime_validator.set_level(level);
    }

    /// Check if validation is enabled for the given severity
    pub fn is_enabled(&self, severity: ValidationSeverity) -> bool {
        self.level.includes_severity(severity)
    }

    /// Validate a buffer descriptor
    pub fn validate_buffer_descriptor(
        &self,
        descriptor: &wgpu::BufferDescriptor<'_>,
    ) -> HaggisResult<()> {
        if !self.is_enabled(ValidationSeverity::Error) {
            return Ok(());
        }

        self.resource_validator.validate_buffer_descriptor(descriptor)
    }

    /// Validate a texture descriptor
    pub fn validate_texture_descriptor(
        &self,
        descriptor: &wgpu::TextureDescriptor<'_>,
    ) -> HaggisResult<()> {
        if !self.is_enabled(ValidationSeverity::Error) {
            return Ok(());
        }

        self.resource_validator.validate_texture_descriptor(descriptor)
    }

    /// Validate a render pipeline descriptor
    pub fn validate_render_pipeline_descriptor(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor<'_>,
    ) -> HaggisResult<()> {
        if !self.is_enabled(ValidationSeverity::Error) {
            return Ok(());
        }

        self.resource_validator.validate_render_pipeline_descriptor(descriptor)
    }

    /// Validate a compute pipeline descriptor
    pub fn validate_compute_pipeline_descriptor(
        &self,
        descriptor: &wgpu::ComputePipelineDescriptor<'_>,
    ) -> HaggisResult<()> {
        if !self.is_enabled(ValidationSeverity::Error) {
            return Ok(());
        }

        self.resource_validator.validate_compute_pipeline_descriptor(descriptor)
    }

    /// Validate a bind group descriptor
    pub fn validate_bind_group_descriptor(
        &self,
        descriptor: &wgpu::BindGroupDescriptor<'_>,
    ) -> HaggisResult<()> {
        if !self.is_enabled(ValidationSeverity::Error) {
            return Ok(());
        }

        self.resource_validator.validate_bind_group_descriptor(descriptor)
    }

    /// Perform runtime validation of an operation
    pub fn validate_runtime_operation<T>(
        &mut self,
        context: &ValidationContext,
        operation: T,
    ) -> HaggisResult<T>
    where
        T: RuntimeValidatable,
    {
        if !self.is_enabled(ValidationSeverity::Warning) {
            return Ok(operation);
        }

        self.runtime_validator.validate_operation(context, operation)
    }

    /// Generate a validation report for debugging
    pub fn generate_report(&self) -> ValidationReport {
        ValidationReport {
            level: self.level,
            constraints: self.constraints.clone(),
            performance: self.performance.clone(),
            resource_stats: self.resource_validator.get_statistics(),
            runtime_stats: self.runtime_validator.get_statistics(),
        }
    }

    /// Create a validator optimized for development
    pub fn for_development() -> Self {
        Self::new(ValidationLevel::Standard)
    }

    /// Create a validator optimized for production
    pub fn for_production() -> Self {
        Self::new(ValidationLevel::Basic)
    }

    /// Create a validator with maximum validation for debugging
    pub fn for_debugging() -> Self {
        Self::new(ValidationLevel::Debug)
    }
}

impl Default for Validator {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        {
            Self::for_development()
        }
        #[cfg(not(debug_assertions))]
        {
            Self::for_production()
        }
    }
}

/// Trait for types that can be validated at runtime
pub trait RuntimeValidatable {
    /// Validate this operation in the given context
    fn validate_runtime(&self, context: &ValidationContext) -> HaggisResult<()>;
}

/// Comprehensive validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Current validation level
    pub level: ValidationLevel,

    /// Device constraints if available
    pub constraints: Option<DeviceConstraints>,

    /// Performance constraints
    pub performance: PerformanceConstraints,

    /// Resource validation statistics
    pub resource_stats: ResourceValidationStats,

    /// Runtime validation statistics
    pub runtime_stats: RuntimeValidationStats,
}

/// Statistics from resource validation
#[derive(Debug, Clone, Default)]
pub struct ResourceValidationStats {
    /// Number of buffer validations performed
    pub buffer_validations: u64,

    /// Number of texture validations performed
    pub texture_validations: u64,

    /// Number of pipeline validations performed
    pub pipeline_validations: u64,

    /// Number of validation errors found
    pub error_count: u64,

    /// Number of validation warnings issued
    pub warning_count: u64,
}

/// Statistics from runtime validation
#[derive(Debug, Clone, Default)]
pub struct RuntimeValidationStats {
    /// Number of runtime operations validated
    pub operation_validations: u64,

    /// Number of performance warnings issued
    pub performance_warnings: u64,

    /// Total validation time in microseconds
    pub total_validation_time_us: u64,

    /// Average validation time per operation
    pub average_validation_time_us: f64,
}

impl ValidationReport {
    /// Format the report as a human-readable string
    pub fn format(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!("Validation Report\n"));
        report.push_str(&format!("================\n"));
        report.push_str(&format!("Level: {:?}\n", self.level));
        report.push_str(&format!("\n"));

        if let Some(constraints) = &self.constraints {
            report.push_str(&format!("Device Constraints:\n"));
            report.push_str(&format!("  Max Buffer Size: {} MB\n", constraints.max_buffer_size / 1024 / 1024));
            report.push_str(&format!("  Max Texture Size: {}x{}\n", constraints.max_texture_size.0, constraints.max_texture_size.1));
            report.push_str(&format!("\n"));
        }

        report.push_str(&format!("Resource Validation:\n"));
        report.push_str(&format!("  Buffers: {}\n", self.resource_stats.buffer_validations));
        report.push_str(&format!("  Textures: {}\n", self.resource_stats.texture_validations));
        report.push_str(&format!("  Pipelines: {}\n", self.resource_stats.pipeline_validations));
        report.push_str(&format!("  Errors: {}\n", self.resource_stats.error_count));
        report.push_str(&format!("  Warnings: {}\n", self.resource_stats.warning_count));
        report.push_str(&format!("\n"));

        report.push_str(&format!("Runtime Validation:\n"));
        report.push_str(&format!("  Operations: {}\n", self.runtime_stats.operation_validations));
        report.push_str(&format!("  Performance Warnings: {}\n", self.runtime_stats.performance_warnings));
        report.push_str(&format!("  Avg Time: {:.2} μs\n", self.runtime_stats.average_validation_time_us));

        report
    }

    /// Check if the validation found any errors
    pub fn has_errors(&self) -> bool {
        self.resource_stats.error_count > 0
    }

    /// Check if the validation issued any warnings
    pub fn has_warnings(&self) -> bool {
        self.resource_stats.warning_count > 0 || self.runtime_stats.performance_warnings > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = Validator::new(ValidationLevel::Standard);
        assert_eq!(validator.level(), ValidationLevel::Standard);
        assert!(validator.is_enabled(ValidationSeverity::Error));
        assert!(validator.is_enabled(ValidationSeverity::Warning));
    }

    #[test]
    fn test_validation_levels() {
        let mut validator = Validator::new(ValidationLevel::None);
        assert!(!validator.is_enabled(ValidationSeverity::Error));

        validator.set_level(ValidationLevel::Debug);
        assert!(validator.is_enabled(ValidationSeverity::Error));
        assert!(validator.is_enabled(ValidationSeverity::Warning));
        assert!(validator.is_enabled(ValidationSeverity::Info));
    }

    #[test]
    fn test_default_validator() {
        let validator = Validator::default();

        // Should use development settings in debug mode, production in release
        #[cfg(debug_assertions)]
        assert_eq!(validator.level(), ValidationLevel::Standard);

        #[cfg(not(debug_assertions))]
        assert_eq!(validator.level(), ValidationLevel::Basic);
    }

    #[test]
    fn test_validation_report() {
        let validator = Validator::for_debugging();
        let report = validator.generate_report();

        assert_eq!(report.level, ValidationLevel::Debug);
        assert!(!report.format().is_empty());
        assert!(!report.has_errors()); // No validations performed yet
    }
}