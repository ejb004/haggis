//! # Unified Builder Pattern
//!
//! Consistent builder pattern interface across simulation, visualization, and scene building APIs

/// Core builder pattern trait for fluent API construction
pub trait Builder<T> {
    fn build(self) -> T;
}

/// Builder pattern for configurable components with chaining methods  
pub trait ConfigurableBuilder<T>: Builder<T> {
    /// Merge configuration from another builder
    fn merge(self, other: Self) -> Self;
    
    /// Validate configuration before building
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
    
    /// Build with validation
    fn build_validated(self) -> Result<T, String> 
    where
        Self: Sized,
    {
        self.validate()?;
        Ok(self.build())
    }
}

/// GPU/CPU resource hint for auto-selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionHint {
    /// Automatically choose based on data size and complexity
    Auto,
    /// Prefer CPU execution
    PreferCpu,
    /// Prefer GPU execution  
    PreferGpu,
    /// Force CPU execution only
    ForceCpu,
    /// Force GPU execution only
    ForceGpu,
}

impl Default for ExecutionHint {
    fn default() -> Self {
        ExecutionHint::Auto
    }
}

/// Common configuration shared across all builders
#[derive(Debug, Clone)]
pub struct CommonConfig {
    pub name: Option<String>,
    pub enabled: bool,
    pub execution_hint: ExecutionHint,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            name: None,
            enabled: true,
            execution_hint: ExecutionHint::Auto,
        }
    }
}

/// Macro for implementing common builder methods
#[macro_export]
macro_rules! impl_common_builder_methods {
    ($builder:ty) => {
        impl $builder {
            /// Set a name for this component
            pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
                self.common.name = Some(name.into());
                self
            }
            
            /// Enable or disable this component
            pub fn with_enabled(mut self, enabled: bool) -> Self {
                self.common.enabled = enabled;
                self
            }
            
            /// Set execution preference
            pub fn with_execution_hint(mut self, hint: crate::builder::ExecutionHint) -> Self {
                self.common.execution_hint = hint;
                self
            }
        }
    };
}