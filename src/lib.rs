//! # Haggis Engine
//!
//! A modern 3D rendering engine with simulation capabilities built on wgpu and winit.
//!
//! Haggis provides a simple, batteries-included API for creating 3D graphics applications
//! and simulations with minimal boilerplate.
//!
//! ## Features
//!
//! - **Easy to use**: Simple API that gets you rendering in just a few lines
//! - **Modern graphics**: Built on wgpu for cross-platform GPU acceleration
//! - **PBR rendering**: Physically-based rendering with metallic-roughness workflow
//! - **Simulation ready**: Built-in support for physics and particle systems
//! - **Cross-platform**: Works on Windows, macOS, Linux, and web via WASM
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and run a basic Haggis application
//!     let haggis = haggis::default();
//!     haggis.run();
//!     Ok(())
//! }
//! ```
//!
//! ## Examples
//!
//! ### Basic Triangle
//!
//! ```rust,no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let haggis = haggis::default();
//!     haggis.run();
//!     Ok(())
//! }
//! ```
//!
//! ### Loading Objects (requires async)
//!
//! ```rust,no_run
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut haggis = haggis::new().await;
//!     
//!     // Load a 3D model
//!     let object = haggis::Object::from_file("assets/model.obj")
//!         .with_material(haggis::Material::default())
//!         .build()?;
//!     
//!     haggis.add_object(object);
//!     haggis.run();
//!     Ok(())
//! }
//! ```
//!
//! ### Custom Materials
//!
//! ```rust,no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut haggis = haggis::new().await;
//!
//! let material = haggis::Material::default()
//!     .with_albedo([1.0, 0.0, 0.0, 1.0])  // Red color
//!     .with_metallic_roughness(0.0, 0.8); // Non-metallic, rough
//!
//! let object = haggis::Object::from_file("cube.obj")
//!     .with_material(material)
//!     .build()?;
//!
//! haggis.add_object(object);
//! haggis.run();
//! # Ok(())
//! # }
//! ```

use app::HaggisApp;

/// Core application and windowing functionality
pub mod app;

/// Graphics rendering system
pub mod gfx;

/// Low-level wgpu utilities and helpers
pub mod wgpu_utils;

// Re-export commonly used types for convenience

/// Creates a new Haggis application synchronously using default settings.
///
/// This is a convenience function that blocks the current thread until
/// the application is fully initialized. For async initialization, use [`new()`] instead.
///
/// # Examples
///
/// ```rust,no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a new Haggis application with default settings
///     let haggis = haggis::default();
///     
///     // Run the application (this will take control of the thread)
///     haggis.run();
///     
///     Ok(())
/// }
/// ```
///
/// # Default Settings
///
/// - **Window size**: 1280x720
/// - **Window title**: "Haggis Engine"
/// - **VSync**: Enabled
/// - **Resizable**: True
///
/// For custom settings, use [`HaggisApp::new()`] or [`with_settings()`] instead.
///
/// # Performance Note
///
/// This function uses `pollster::block_on()` internally to run the async
/// initialization synchronously. For better performance in async contexts,
/// prefer using [`new()`] directly.
///
/// [`new()`]: fn.new.html
/// [`with_settings()`]: fn.with_settings.html
pub fn default() -> HaggisApp {
    pollster::block_on(HaggisApp::new())
}

/// Creates a new Haggis application asynchronously with default settings.
///
/// This function initializes the graphics context and creates a window with
/// default settings. Use this in async contexts for better performance.
///
/// # Examples
///
/// ```rust,no_run
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a new Haggis application asynchronously
///     let mut haggis = haggis::new().await;
///     
///     // Add some content to the scene
///     let object = haggis::Object::from_file("model.obj")
///         .with_material(haggis::Material::default())
///         .build()?;
///     
///     haggis.add_object(object);
///     
///     // Run the application
///     haggis.run();
///     
///     Ok(())
/// }
/// ```
///
/// # Default Settings
///
/// - **Window size**: 1280x720
/// - **Window title**: "Haggis Engine"  
/// - **VSync**: Enabled
/// - **Resizable**: True
///
/// For custom window settings, use [`HaggisApp::with_settings()`] instead.
pub async fn new() -> HaggisApp {
    HaggisApp::new().await
}
