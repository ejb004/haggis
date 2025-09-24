//! # Validation Types and Enums
//!
//! Core types and enumerations for the validation system.

use std::fmt;

/// Validation levels controlling the strictness of validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationLevel {
    /// No validation performed (for maximum performance)
    None = 0,

    /// Only critical safety checks
    Basic = 1,

    /// Standard validation suitable for development
    Standard = 2,

    /// Strict validation with performance warnings
    Strict = 3,

    /// Full validation with detailed diagnostics
    Debug = 4,
}

impl ValidationLevel {
    /// Check if this level includes the given severity
    pub fn includes_severity(self, severity: ValidationSeverity) -> bool {
        match (self, severity) {
            (ValidationLevel::None, _) => false,
            (ValidationLevel::Basic, ValidationSeverity::Error) => true,
            (ValidationLevel::Basic, _) => false,
            (ValidationLevel::Standard, ValidationSeverity::Info) => false,
            (ValidationLevel::Standard, _) => true,
            (ValidationLevel::Strict, _) => true,
            (ValidationLevel::Debug, _) => true,
        }
    }

    /// Get the human-readable name
    pub fn name(self) -> &'static str {
        match self {
            ValidationLevel::None => "None",
            ValidationLevel::Basic => "Basic",
            ValidationLevel::Standard => "Standard",
            ValidationLevel::Strict => "Strict",
            ValidationLevel::Debug => "Debug",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(ValidationLevel::None),
            "basic" => Some(ValidationLevel::Basic),
            "standard" => Some(ValidationLevel::Standard),
            "strict" => Some(ValidationLevel::Strict),
            "debug" => Some(ValidationLevel::Debug),
            _ => None,
        }
    }
}

impl Default for ValidationLevel {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        {
            ValidationLevel::Standard
        }
        #[cfg(not(debug_assertions))]
        {
            ValidationLevel::Basic
        }
    }
}

impl fmt::Display for ValidationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Severity levels for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    /// Informational messages
    Info = 0,

    /// Performance warnings that don't affect correctness
    Warning = 1,

    /// Errors that could cause incorrect behavior
    Error = 2,

    /// Critical errors that will cause failures
    Critical = 3,
}

impl ValidationSeverity {
    /// Get the human-readable name
    pub fn name(self) -> &'static str {
        match self {
            ValidationSeverity::Info => "Info",
            ValidationSeverity::Warning => "Warning",
            ValidationSeverity::Error => "Error",
            ValidationSeverity::Critical => "Critical",
        }
    }

    /// Get ANSI color code for terminal output
    pub fn color_code(self) -> &'static str {
        match self {
            ValidationSeverity::Info => "\x1b[36m",      // Cyan
            ValidationSeverity::Warning => "\x1b[33m",   // Yellow
            ValidationSeverity::Error => "\x1b[31m",     // Red
            ValidationSeverity::Critical => "\x1b[91m",  // Bright Red
        }
    }

    /// Reset ANSI color
    pub fn reset_color() -> &'static str {
        "\x1b[0m"
    }
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A validation rule that can be applied to operations
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Unique identifier for this rule
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this rule checks
    pub description: String,

    /// Severity level for violations
    pub severity: ValidationSeverity,

    /// Category of validation (performance, safety, etc.)
    pub category: ValidationCategory,

    /// Whether this rule is enabled
    pub enabled: bool,
}

impl ValidationRule {
    /// Create a new validation rule
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        severity: ValidationSeverity,
        category: ValidationCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            severity,
            category,
            enabled: true,
        }
    }

    /// Create an error rule
    pub fn error(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: ValidationCategory,
    ) -> Self {
        Self::new(id, name, description, ValidationSeverity::Error, category)
    }

    /// Create a warning rule
    pub fn warning(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: ValidationCategory,
    ) -> Self {
        Self::new(id, name, description, ValidationSeverity::Warning, category)
    }

    /// Create an info rule
    pub fn info(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: ValidationCategory,
    ) -> Self {
        Self::new(id, name, description, ValidationSeverity::Info, category)
    }

    /// Enable or disable this rule
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if this rule should be applied at the given level
    pub fn applies_at_level(&self, level: ValidationLevel) -> bool {
        self.enabled && level.includes_severity(self.severity)
    }
}

/// Categories of validation rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCategory {
    /// Memory safety and correctness
    Safety,

    /// Performance optimization
    Performance,

    /// API usage best practices
    Usage,

    /// Compatibility with different devices
    Compatibility,

    /// Debug and development assistance
    Debug,
}

impl ValidationCategory {
    /// Get the human-readable name
    pub fn name(self) -> &'static str {
        match self {
            ValidationCategory::Safety => "Safety",
            ValidationCategory::Performance => "Performance",
            ValidationCategory::Usage => "Usage",
            ValidationCategory::Compatibility => "Compatibility",
            ValidationCategory::Debug => "Debug",
        }
    }

    /// Get the short code for this category
    pub fn code(self) -> &'static str {
        match self {
            ValidationCategory::Safety => "SAF",
            ValidationCategory::Performance => "PERF",
            ValidationCategory::Usage => "USE",
            ValidationCategory::Compatibility => "COMPAT",
            ValidationCategory::Debug => "DBG",
        }
    }
}

impl fmt::Display for ValidationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Result of a validation check
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// The rule that was violated
    pub rule: ValidationRule,

    /// Detailed message about the issue
    pub message: String,

    /// Optional context information
    pub context: Option<String>,

    /// Suggested fix for the issue
    pub suggestion: Option<String>,

    /// Source location if available
    pub location: Option<SourceLocation>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(rule: ValidationRule, message: impl Into<String>) -> Self {
        Self {
            rule,
            message: message.into(),
            context: None,
            suggestion: None,
            location: None,
        }
    }

    /// Add context to this issue
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add a suggestion to this issue
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add source location to this issue
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Format this issue for display
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Severity and category
        output.push_str(&format!(
            "{}[{}:{}]{} ",
            self.rule.severity.color_code(),
            self.rule.category.code(),
            self.rule.severity.name(),
            ValidationSeverity::reset_color()
        ));

        // Rule name and message
        output.push_str(&format!("{}: {}", self.rule.name, self.message));

        // Location if available
        if let Some(location) = &self.location {
            output.push_str(&format!(" at {}", location));
        }

        // Context if available
        if let Some(context) = &self.context {
            output.push_str(&format!("\n  Context: {}", context));
        }

        // Suggestion if available
        if let Some(suggestion) = &self.suggestion {
            output.push_str(&format!("\n  Suggestion: {}", suggestion));
        }

        output
    }

    /// Get the severity of this issue
    pub fn severity(&self) -> ValidationSeverity {
        self.rule.severity
    }

    /// Get the category of this issue
    pub fn category(&self) -> ValidationCategory {
        self.rule.category
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Source location information for validation issues
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// File path or identifier
    pub file: String,

    /// Line number (1-based)
    pub line: Option<u32>,

    /// Column number (1-based)
    pub column: Option<u32>,

    /// Function or context name
    pub function: Option<String>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
            column: None,
            function: None,
        }
    }

    /// Create with line number
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Create with column number
    pub fn with_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    /// Create with function name
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.function = Some(function.into());
        self
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file)?;

        if let Some(line) = self.line {
            write!(f, ":{}", line)?;

            if let Some(column) = self.column {
                write!(f, ":{}", column)?;
            }
        }

        if let Some(function) = &self.function {
            write!(f, " in {}", function)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_levels() {
        assert!(ValidationLevel::Debug > ValidationLevel::Standard);
        assert!(ValidationLevel::Standard > ValidationLevel::Basic);
        assert!(ValidationLevel::Basic > ValidationLevel::None);

        assert!(ValidationLevel::Debug.includes_severity(ValidationSeverity::Info));
        assert!(ValidationLevel::Standard.includes_severity(ValidationSeverity::Warning));
        assert!(!ValidationLevel::Standard.includes_severity(ValidationSeverity::Info));
        assert!(!ValidationLevel::None.includes_severity(ValidationSeverity::Error));
    }

    #[test]
    fn test_validation_severity() {
        assert!(ValidationSeverity::Critical > ValidationSeverity::Error);
        assert!(ValidationSeverity::Error > ValidationSeverity::Warning);
        assert!(ValidationSeverity::Warning > ValidationSeverity::Info);

        assert_eq!(ValidationSeverity::Error.name(), "Error");
        assert!(!ValidationSeverity::Warning.color_code().is_empty());
    }

    #[test]
    fn test_validation_rule() {
        let rule = ValidationRule::error(
            "test_rule",
            "Test Rule",
            "This is a test rule",
            ValidationCategory::Safety,
        );

        assert_eq!(rule.id, "test_rule");
        assert_eq!(rule.severity, ValidationSeverity::Error);
        assert!(rule.enabled);
        assert!(rule.applies_at_level(ValidationLevel::Standard));
        assert!(!rule.applies_at_level(ValidationLevel::None));
    }

    #[test]
    fn test_validation_issue() {
        let rule = ValidationRule::warning(
            "perf_warning",
            "Performance Warning",
            "This operation may be slow",
            ValidationCategory::Performance,
        );

        let issue = ValidationIssue::new(rule, "Buffer size is large")
            .with_context("Creating vertex buffer")
            .with_suggestion("Consider using smaller batches");

        assert!(issue.format().contains("Performance Warning"));
        assert!(issue.format().contains("Buffer size is large"));
        assert!(issue.format().contains("Context: Creating vertex buffer"));
    }

    #[test]
    fn test_source_location() {
        let location = SourceLocation::new("main.rs")
            .with_line(42)
            .with_column(10)
            .with_function("create_buffer");

        assert_eq!(location.to_string(), "main.rs:42:10 in create_buffer");
    }

    #[test]
    fn test_validation_level_from_string() {
        assert_eq!(ValidationLevel::from_str("debug"), Some(ValidationLevel::Debug));
        assert_eq!(ValidationLevel::from_str("STANDARD"), Some(ValidationLevel::Standard));
        assert_eq!(ValidationLevel::from_str("invalid"), None);
    }
}