//! # Haggis Error Handling
//!
//! Comprehensive error handling system for the Haggis graphics framework.
//! Provides structured error reporting with context and recovery suggestions.

use thiserror::Error;

/// Result type alias for Haggis operations
pub type HaggisResult<T> = Result<T, HaggisError>;

/// Comprehensive error type for all Haggis operations
#[derive(Error, Debug, Clone)]
pub enum HaggisError {
    /// Graphics/Rendering related errors
    #[error("Graphics error: {message}")]
    Graphics {
        message: String,
        context: Option<String>,
        suggestion: Option<String>,
    },

    /// GPU resource management errors
    #[error("Resource error: {message}")]
    Resource {
        message: String,
        resource_type: String,
        resource_id: Option<String>,
        suggestion: Option<String>,
    },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        expected: Option<String>,
        actual: Option<String>,
    },

    /// Shader compilation/loading errors
    #[error("Shader error: {message}")]
    Shader {
        message: String,
        shader_name: Option<String>,
        line: Option<u32>,
        suggestion: Option<String>,
    },

    /// Buffer operation errors
    #[error("Buffer error: {message}")]
    Buffer {
        message: String,
        buffer_type: String,
        size: Option<u64>,
        suggestion: Option<String>,
    },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        config_key: Option<String>,
        suggestion: Option<String>,
    },

    /// IO/File related errors
    #[error("IO error: {message}")]
    Io {
        message: String,
        file_path: Option<String>,
        suggestion: Option<String>,
    },

    /// Memory allocation errors
    #[error("Memory error: {message}")]
    Memory {
        message: String,
        requested_size: Option<u64>,
        available_size: Option<u64>,
        suggestion: Option<String>,
    },

    /// Simulation errors
    #[error("Simulation error: {message}")]
    Simulation {
        message: String,
        simulation_name: Option<String>,
        suggestion: Option<String>,
    },

    /// Generic errors with context
    #[error("Error: {message}")]
    Generic {
        message: String,
        context: Option<String>,
        suggestion: Option<String>,
    },
}

impl HaggisError {
    /// Create a graphics error with optional context and suggestion
    pub fn graphics(message: impl Into<String>) -> Self {
        Self::Graphics {
            message: message.into(),
            context: None,
            suggestion: None,
        }
    }

    /// Create a graphics error with context
    pub fn graphics_with_context(
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::Graphics {
            message: message.into(),
            context: Some(context.into()),
            suggestion: None,
        }
    }

    /// Create a resource error
    pub fn resource(message: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self::Resource {
            message: message.into(),
            resource_type: resource_type.into(),
            resource_id: None,
            suggestion: None,
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
            expected: None,
            actual: None,
        }
    }

    /// Create a validation error with field details
    pub fn validation_field(
        message: impl Into<String>,
        field: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
            expected: Some(expected.into()),
            actual: Some(actual.into()),
        }
    }

    /// Create a shader error
    pub fn shader(message: impl Into<String>) -> Self {
        Self::Shader {
            message: message.into(),
            shader_name: None,
            line: None,
            suggestion: None,
        }
    }

    /// Create a buffer error
    pub fn buffer(message: impl Into<String>, buffer_type: impl Into<String>) -> Self {
        Self::Buffer {
            message: message.into(),
            buffer_type: buffer_type.into(),
            size: None,
            suggestion: None,
        }
    }

    /// Create a memory error
    pub fn memory(message: impl Into<String>) -> Self {
        Self::Memory {
            message: message.into(),
            requested_size: None,
            available_size: None,
            suggestion: None,
        }
    }

    /// Create a simulation error
    pub fn simulation(message: impl Into<String>) -> Self {
        Self::Simulation {
            message: message.into(),
            simulation_name: None,
            suggestion: None,
        }
    }

    /// Add a suggestion to any error type
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        match &mut self {
            Self::Graphics { suggestion: s, .. }
            | Self::Resource { suggestion: s, .. }
            | Self::Shader { suggestion: s, .. }
            | Self::Buffer { suggestion: s, .. }
            | Self::Configuration { suggestion: s, .. }
            | Self::Io { suggestion: s, .. }
            | Self::Memory { suggestion: s, .. }
            | Self::Simulation { suggestion: s, .. }
            | Self::Generic { suggestion: s, .. } => {
                *s = Some(suggestion.into());
            }
            Self::Validation { .. } => {
                // Validation errors don't have suggestions field
            }
        }
        self
    }

    /// Add context to any error type that supports it
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        match &mut self {
            Self::Graphics { context: c, .. } | Self::Generic { context: c, .. } => {
                *c = Some(context.into());
            }
            _ => {
                // Other error types don't have context field or use different fields
            }
        }
        self
    }

    /// Get the error's suggestion if available
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::Graphics { suggestion, .. }
            | Self::Resource { suggestion, .. }
            | Self::Shader { suggestion, .. }
            | Self::Buffer { suggestion, .. }
            | Self::Configuration { suggestion, .. }
            | Self::Io { suggestion, .. }
            | Self::Memory { suggestion, .. }
            | Self::Simulation { suggestion, .. }
            | Self::Generic { suggestion, .. } => suggestion.as_deref(),
            Self::Validation { .. } => None,
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Graphics { .. } => true,     // Often recoverable by retrying or fallback
            Self::Resource { .. } => true,     // Resource can often be reallocated
            Self::Validation { .. } => false,  // User input errors need correction
            Self::Shader { .. } => false,      // Shader errors need code fixes
            Self::Buffer { .. } => true,       // Buffer errors often recoverable
            Self::Configuration { .. } => false, // Config errors need user correction
            Self::Io { .. } => true,           // IO errors often transient
            Self::Memory { .. } => true,       // Memory issues might be recoverable
            Self::Simulation { .. } => true,   // Simulation errors often recoverable
            Self::Generic { .. } => false,     // Unknown generic errors are not recoverable
        }
    }

    /// Get a user-friendly error message with context and suggestions
    pub fn user_message(&self) -> String {
        let base_message = self.to_string();
        let mut message = base_message;

        // Add context if available
        match self {
            Self::Graphics { context: Some(context), .. } => {
                message = format!("{}\nContext: {}", message, context);
            }
            Self::Validation { field: Some(field), expected: Some(expected), actual: Some(actual), .. } => {
                message = format!("{}\nField: {}\nExpected: {}\nActual: {}", message, field, expected, actual);
            }
            _ => {}
        }

        // Add suggestion if available
        if let Some(suggestion) = self.suggestion() {
            message = format!("{}\n\nSuggestion: {}", message, suggestion);
        }

        message
    }
}

/// Convert from wgpu errors
impl From<wgpu::SurfaceError> for HaggisError {
    fn from(err: wgpu::SurfaceError) -> Self {
        match err {
            wgpu::SurfaceError::Lost => Self::graphics("Surface was lost")
                .with_suggestion("Try recreating the surface or window"),
            wgpu::SurfaceError::OutOfMemory => Self::memory("GPU ran out of memory")
                .with_suggestion("Reduce texture sizes or use lower quality settings"),
            wgpu::SurfaceError::Outdated => Self::graphics("Surface is outdated")
                .with_suggestion("Recreate the surface"),
            wgpu::SurfaceError::Timeout => Self::graphics("Surface operation timed out")
                .with_suggestion("Try again or check GPU driver"),
            wgpu::SurfaceError::Other => Self::graphics("Unknown surface error")
                .with_suggestion("Check GPU driver and try again"),
        }
    }
}

/// Convert from wgpu request device errors
impl From<wgpu::RequestDeviceError> for HaggisError {
    fn from(err: wgpu::RequestDeviceError) -> Self {
        Self::graphics(format!("Failed to request GPU device: {:?}", err))
            .with_suggestion("Check that your GPU supports the required features")
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for HaggisError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            file_path: None,
            suggestion: Some("Check file permissions and path".to_string()),
        }
    }
}

/// Builder for creating detailed errors
pub struct ErrorBuilder {
    error: HaggisError,
}

impl ErrorBuilder {
    pub fn graphics(message: impl Into<String>) -> Self {
        Self {
            error: HaggisError::graphics(message),
        }
    }

    pub fn resource(message: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self {
            error: HaggisError::resource(message, resource_type),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            error: HaggisError::validation(message),
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.error = self.error.with_suggestion(suggestion);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.error = self.error.with_context(context);
        self
    }

    pub fn build(self) -> HaggisError {
        self.error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = HaggisError::graphics("Test error");
        assert!(matches!(err, HaggisError::Graphics { .. }));
    }

    #[test]
    fn test_error_with_suggestion() {
        let err = HaggisError::graphics("Test error")
            .with_suggestion("Try this fix");
        assert_eq!(err.suggestion(), Some("Try this fix"));
    }

    #[test]
    fn test_error_builder() {
        let err = ErrorBuilder::graphics("Test error")
            .with_suggestion("Try this")
            .with_context("In test function")
            .build();

        assert!(matches!(err, HaggisError::Graphics { .. }));
        assert_eq!(err.suggestion(), Some("Try this"));
    }

    #[test]
    fn test_recoverable_errors() {
        let graphics_err = HaggisError::graphics("Graphics error");
        let validation_err = HaggisError::validation("Invalid input");

        assert!(graphics_err.is_recoverable());
        assert!(!validation_err.is_recoverable());
    }

    #[test]
    fn test_user_message() {
        let err = HaggisError::validation_field(
            "Invalid size",
            "width",
            "positive number",
            "negative number"
        ).with_suggestion("Use a positive value");

        let message = err.user_message();
        assert!(message.contains("Field: width"));
        assert!(message.contains("Suggestion: Use a positive value"));
    }
}