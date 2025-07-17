//! # Scene Management Module
//!
//! This module provides 3D scene management functionality including object hierarchies,
//! scene graphs, and vertex data structures. It handles the organization and rendering
//! of 3D objects within the Haggis engine.
//!
//! ## Key Components
//!
//! - [`Scene`] - The main scene container that manages objects, camera, and materials
//! - [`Object`] - Individual 3D objects with meshes, materials, and transforms
//! - [`ObjectBuilder`] - Builder pattern for configuring objects
//! - [`Vertex3D`] - 3D vertex data structure with position, normal, and texture coordinates
//!
//! ## Usage
//!
//! The scene system is primarily used through the [`Scene`] struct:
//!
//! ```no_run
//! use haggis::gfx::scene::{Scene, Object};
//! use haggis::gfx::camera::camera_utils::CameraManager;
//!
//! // Scene creation is typically handled by HaggisApp
//! // let scene = Scene::new(camera_manager);
//!
//! // Objects are added through the builder pattern
//! // scene.add_object("model.obj");
//! ```
//!
//! ## Object Management
//!
//! Objects in the scene support:
//! - Mesh data loading (OBJ format)
//! - Material assignment and PBR properties
//! - Transform operations (position, rotation, scale)
//! - GPU resource management
//! - Builder pattern configuration

pub mod object;
pub mod scene;
pub mod vertex;

// Re-export main types
pub use object::{DrawObject, Object, ObjectBuilder};
pub use scene::Scene;
pub use vertex::Vertex3D;
