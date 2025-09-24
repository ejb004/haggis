//! # Runtime Validation
//!
//! Runtime validation system for GPU operations and performance monitoring.

use super::types::{ValidationLevel, ValidationRule, ValidationIssue, ValidationSeverity, ValidationCategory};
use crate::error::{HaggisError, HaggisResult};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, Duration};
use std::collections::HashMap;

/// Runtime validator for GPU operations
pub struct RuntimeValidator {
    /// Current validation level
    level: ValidationLevel,

    /// Operation performance tracking
    performance_tracker: PerformanceTracker,

    /// Validation statistics
    stats: RuntimeValidationStats,
}

impl RuntimeValidator {
    /// Create a new runtime validator
    pub fn new(level: ValidationLevel) -> Self {
        Self {
            level,
            performance_tracker: PerformanceTracker::new(),
            stats: RuntimeValidationStats::new(),
        }
    }

    /// Set the validation level
    pub fn set_level(&mut self, level: ValidationLevel) {
        self.level = level;
    }

    /// Validate a runtime operation
    pub fn validate_operation<T>(
        &mut self,
        context: &ValidationContext,
        operation: T,
    ) -> HaggisResult<T>
    where
        T: super::RuntimeValidatable,
    {
        let start_time = Instant::now();
        self.stats.operation_validations.fetch_add(1, Ordering::Relaxed);

        // Perform validation
        let result = operation.validate_runtime(context);

        // Track performance
        let validation_time = start_time.elapsed();
        self.performance_tracker.record_validation_time(validation_time);

        // Update statistics
        let total_time = self.stats.total_validation_time_us.fetch_add(
            validation_time.as_micros() as u64,
            Ordering::Relaxed,
        ) + validation_time.as_micros() as u64;

        let operation_count = self.stats.operation_validations.load(Ordering::Relaxed);
        self.stats.average_validation_time_us.store(
            (total_time as f64 / operation_count as f64) as u64,
            Ordering::Relaxed,
        );

        match result {
            Ok(()) => Ok(operation),
            Err(e) => {
                self.stats.performance_warnings.fetch_add(1, Ordering::Relaxed);
                if self.level >= ValidationLevel::Standard {
                    eprintln!("Runtime validation warning: {}", e);
                }
                Ok(operation) // Don't fail on runtime warnings
            }
        }
    }

    /// Check for performance issues in the current frame
    pub fn check_frame_performance(&self, context: &ValidationContext) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if self.level < ValidationLevel::Standard {
            return issues;
        }

        // Check draw call count
        if context.draw_calls_this_frame > 1000 {
            let rule = ValidationRule::warning(
                "high_draw_calls",
                "High Draw Call Count",
                "Frame has excessive draw calls",
                ValidationCategory::Performance,
            );
            let issue = ValidationIssue::new(
                rule,
                format!("Frame has {} draw calls", context.draw_calls_this_frame),
            ).with_suggestion("Consider batching draws or using instancing");
            issues.push(issue);
        }

        // Check state change count
        if context.state_changes_this_frame > 500 {
            let rule = ValidationRule::warning(
                "high_state_changes",
                "High State Change Count",
                "Frame has excessive state changes",
                ValidationCategory::Performance,
            );
            let issue = ValidationIssue::new(
                rule,
                format!("Frame has {} state changes", context.state_changes_this_frame),
            ).with_suggestion("Minimize pipeline and bind group changes");
            issues.push(issue);
        }

        // Check memory usage
        if let Some(memory_usage) = context.current_memory_usage_mb {
            if memory_usage > 1024 { // 1GB
                let rule = ValidationRule::warning(
                    "high_memory_usage",
                    "High Memory Usage",
                    "GPU memory usage is high",
                    ValidationCategory::Performance,
                );
                let issue = ValidationIssue::new(
                    rule,
                    format!("GPU memory usage is {} MB", memory_usage),
                ).with_suggestion("Consider releasing unused resources or using compression");
                issues.push(issue);
            }
        }

        issues
    }

    /// Get runtime validation statistics
    pub fn get_statistics(&self) -> super::RuntimeValidationStats {
        super::RuntimeValidationStats {
            operation_validations: self.stats.operation_validations.load(Ordering::Relaxed),
            performance_warnings: self.stats.performance_warnings.load(Ordering::Relaxed),
            total_validation_time_us: self.stats.total_validation_time_us.load(Ordering::Relaxed),
            average_validation_time_us: self.stats.average_validation_time_us.load(Ordering::Relaxed) as f64,
        }
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        self.stats.operation_validations.store(0, Ordering::Relaxed);
        self.stats.performance_warnings.store(0, Ordering::Relaxed);
        self.stats.total_validation_time_us.store(0, Ordering::Relaxed);
        self.stats.average_validation_time_us.store(0, Ordering::Relaxed);
    }
}

/// Context information for runtime validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Current frame number
    pub frame_number: u64,

    /// Number of draw calls in current frame
    pub draw_calls_this_frame: u32,

    /// Number of state changes in current frame
    pub state_changes_this_frame: u32,

    /// Current GPU memory usage in MB
    pub current_memory_usage_mb: Option<u64>,

    /// Time since last frame in seconds
    pub delta_time: f32,

    /// Current render pass name
    pub current_pass: Option<String>,

    /// Additional context data
    pub metadata: HashMap<String, String>,
}

impl ValidationContext {
    /// Create a new validation context
    pub fn new(frame_number: u64) -> Self {
        Self {
            frame_number,
            draw_calls_this_frame: 0,
            state_changes_this_frame: 0,
            current_memory_usage_mb: None,
            delta_time: 0.0,
            current_pass: None,
            metadata: HashMap::new(),
        }
    }

    /// Record a draw call
    pub fn record_draw_call(&mut self) {
        self.draw_calls_this_frame += 1;
    }

    /// Record a state change
    pub fn record_state_change(&mut self) {
        self.state_changes_this_frame += 1;
    }

    /// Set memory usage
    pub fn set_memory_usage(&mut self, usage_mb: u64) {
        self.current_memory_usage_mb = Some(usage_mb);
    }

    /// Set the current render pass
    pub fn set_current_pass(&mut self, pass_name: impl Into<String>) {
        self.current_pass = Some(pass_name.into());
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Check if this is a performance-critical frame
    pub fn is_performance_critical(&self) -> bool {
        self.draw_calls_this_frame > 100 || self.state_changes_this_frame > 50
    }
}

/// Result of a validation operation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub success: bool,

    /// Issues found during validation
    pub issues: Vec<ValidationIssue>,

    /// Time taken for validation
    pub validation_time: Duration,

    /// Context information
    pub context: ValidationContext,
}

impl ValidationResult {
    /// Create a successful result
    pub fn success(context: ValidationContext, validation_time: Duration) -> Self {
        Self {
            success: true,
            issues: Vec::new(),
            validation_time,
            context,
        }
    }

    /// Create a result with issues
    pub fn with_issues(
        context: ValidationContext,
        validation_time: Duration,
        issues: Vec<ValidationIssue>,
    ) -> Self {
        let success = !issues.iter().any(|i| {
            matches!(i.severity(), ValidationSeverity::Error | ValidationSeverity::Critical)
        });

        Self {
            success,
            issues,
            validation_time,
            context,
        }
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| {
            matches!(i.severity(), ValidationSeverity::Error | ValidationSeverity::Critical)
        })
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| {
            matches!(i.severity(), ValidationSeverity::Warning)
        })
    }

    /// Get the number of issues by severity
    pub fn count_by_severity(&self, severity: ValidationSeverity) -> usize {
        self.issues.iter().filter(|i| i.severity() == severity).count()
    }
}

/// Performance tracking for validation operations
struct PerformanceTracker {
    /// Recent validation times (circular buffer)
    recent_times: Vec<Duration>,

    /// Current index in the circular buffer
    current_index: usize,

    /// Whether the buffer is full
    buffer_full: bool,
}

impl PerformanceTracker {
    const BUFFER_SIZE: usize = 100;

    fn new() -> Self {
        Self {
            recent_times: vec![Duration::default(); Self::BUFFER_SIZE],
            current_index: 0,
            buffer_full: false,
        }
    }

    fn record_validation_time(&mut self, time: Duration) {
        self.recent_times[self.current_index] = time;
        self.current_index = (self.current_index + 1) % Self::BUFFER_SIZE;

        if self.current_index == 0 {
            self.buffer_full = true;
        }
    }

    fn average_time(&self) -> Duration {
        let count = if self.buffer_full {
            Self::BUFFER_SIZE
        } else {
            self.current_index
        };

        if count == 0 {
            return Duration::default();
        }

        let total: Duration = self.recent_times[..count].iter().sum();
        total / count as u32
    }

    fn max_time(&self) -> Duration {
        let count = if self.buffer_full {
            Self::BUFFER_SIZE
        } else {
            self.current_index
        };

        if count == 0 {
            return Duration::default();
        }

        self.recent_times[..count].iter().max().copied().unwrap_or_default()
    }
}

/// Runtime validation statistics
struct RuntimeValidationStats {
    operation_validations: AtomicU64,
    performance_warnings: AtomicU64,
    total_validation_time_us: AtomicU64,
    average_validation_time_us: AtomicU64,
}

impl RuntimeValidationStats {
    fn new() -> Self {
        Self {
            operation_validations: AtomicU64::new(0),
            performance_warnings: AtomicU64::new(0),
            total_validation_time_us: AtomicU64::new(0),
            average_validation_time_us: AtomicU64::new(0),
        }
    }
}

// Example implementations of RuntimeValidatable
impl super::RuntimeValidatable for wgpu::RenderPass<'_> {
    fn validate_runtime(&self, context: &ValidationContext) -> HaggisResult<()> {
        // Example validation for render passes
        if context.draw_calls_this_frame > 1000 {
            return Err(HaggisError::validation("Too many draw calls in frame"));
        }
        Ok(())
    }
}

impl super::RuntimeValidatable for u32 {
    fn validate_runtime(&self, _context: &ValidationContext) -> HaggisResult<()> {
        // Example validation for buffer operations
        if *self > 1000000 {
            return Err(HaggisError::validation("Operation parameter too large"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_context() {
        let mut context = ValidationContext::new(1);
        assert_eq!(context.frame_number, 1);
        assert_eq!(context.draw_calls_this_frame, 0);

        context.record_draw_call();
        assert_eq!(context.draw_calls_this_frame, 1);

        context.record_state_change();
        assert_eq!(context.state_changes_this_frame, 1);

        context.set_memory_usage(512);
        assert_eq!(context.current_memory_usage_mb, Some(512));
    }

    #[test]
    fn test_performance_tracker() {
        let mut tracker = PerformanceTracker::new();

        tracker.record_validation_time(Duration::from_micros(100));
        tracker.record_validation_time(Duration::from_micros(200));

        assert!(tracker.average_time() > Duration::from_micros(100));
        assert_eq!(tracker.max_time(), Duration::from_micros(200));
    }

    #[test]
    fn test_runtime_validator() {
        let validator = RuntimeValidator::new(ValidationLevel::Standard);
        let context = ValidationContext::new(1);

        // Test with a simple operation
        let result = validator.validate_operation(&context, 42u32);
        assert!(result.is_ok());

        let stats = validator.get_statistics();
        assert_eq!(stats.operation_validations, 1);
    }

    #[test]
    fn test_validation_result() {
        let context = ValidationContext::new(1);
        let time = Duration::from_micros(100);

        let result = ValidationResult::success(context.clone(), time);
        assert!(result.success);
        assert!(!result.has_errors());
        assert!(!result.has_warnings());

        let rule = ValidationRule::warning(
            "test",
            "Test Warning",
            "Test description",
            ValidationCategory::Performance,
        );
        let issue = ValidationIssue::new(rule, "Test message");
        let result = ValidationResult::with_issues(context, time, vec![issue]);

        assert!(result.success); // Warnings don't make validation fail
        assert!(!result.has_errors());
        assert!(result.has_warnings());
        assert_eq!(result.count_by_severity(ValidationSeverity::Warning), 1);
    }

    #[test]
    fn test_frame_performance_check() {
        let validator = RuntimeValidator::new(ValidationLevel::Standard);
        let mut context = ValidationContext::new(1);

        // Set high draw call count
        context.draw_calls_this_frame = 1500;
        let issues = validator.check_frame_performance(&context);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.rule.id == "high_draw_calls"));
    }
}