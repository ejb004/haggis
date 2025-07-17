//! # Graphics Module
//!
//! This module contains all graphics-related functionality for the Haggis 3D engine,
//! including camera systems, rendering pipelines, scene management, and resource handling.
//!
//! ## Architecture Overview
//!
//! The graphics system is organized into several key components:
//!
//! - **Camera System** ([`camera`]) - Orbit camera with smooth controls
//! - **Rendering Pipeline** ([`rendering`]) - PBR rendering with shadow mapping
//! - **Scene Management** ([`scene`]) - Object hierarchy and scene graph
//! - **Resource Management** ([`resources`]) - Materials, textures, and GPU resources
//!
//! ## Key Features
//!
//! - **Physically-Based Rendering (PBR)** - Realistic material rendering
//! - **Shadow Mapping** - Real-time shadows with blur post-processing
//! - **Orbit Camera** - Smooth camera controls with zoom and pan
//! - **Material System** - Flexible material definition and GPU resource management
//! - **Efficient Resource Management** - Optimized GPU buffer and texture handling
//!
//! ## Usage
//!
//! The graphics system is primarily used through the [`RenderEngine`] and [`Scene`] types:
//!
//! ```no_run
//! use haggis::gfx::{RenderEngine, scene::Scene};
//!
//! // The render engine is typically created automatically by HaggisApp
//! // let render_engine = RenderEngine::new(window, width, height).await;
//!
//! // Scene management is handled through the main app
//! // let mut scene = Scene::new(camera_manager);
//! ```
//!
//! [`Scene`]: scene::Scene

pub mod camera;
pub mod rendering;
pub mod resources;
pub mod scene;

// Re-export commonly used types
pub use camera::orbit_camera::OrbitCamera;
pub use rendering::render_engine::RenderEngine;
