//! Render pipeline management system for wgpu
//!
//! Provides high-level pipeline creation, caching, and hot-reloading capabilities
//! with support for shared bind group layouts and lazy pipeline creation.

use std::{collections::HashMap, sync::Arc};
use wgpu::*;

use crate::gfx::scene::vertex::Vertex3D;

/// Configuration for creating a render pipeline
///
/// Defines all parameters needed to create a wgpu render pipeline,
/// including shaders, bind group layouts, and render state.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub label: String,
    pub shader: String,
    pub bind_group_layouts: Vec<BindGroupLayout>,
    pub primitive_topology: PrimitiveTopology,
    pub cull_mode: Option<Face>,
    pub depth_texture: Option<Texture>,
    pub multisample: MultisampleState,
    pub color_targets: Vec<Option<ColorTargetState>>,
    pub vertex_only: bool,       //for shadow pass
    pub no_vertex_buffers: bool, // NEW: for fullscreen quads
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            label: "Default Pipeline".to_string(),
            shader: "shader.wgsl".to_string(),
            bind_group_layouts: Vec::new(),
            primitive_topology: PrimitiveTopology::TriangleList,
            cull_mode: Some(Face::Back),
            depth_texture: None,
            multisample: MultisampleState::default(),
            color_targets: vec![Some(ColorTargetState {
                format: TextureFormat::Bgra8Unorm,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            vertex_only: false,
            no_vertex_buffers: false, // NEW
        }
    }
}

impl PipelineConfig {
    /// Creates a new config with a specific shader
    ///
    /// # Arguments
    /// * `shader` - Shader identifier to use for this pipeline
    pub fn default_with_shader(shader: &str) -> Self {
        Self {
            shader: shader.to_string(),
            ..Default::default()
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_owned();
        self
    }

    pub fn with_cull_mode(mut self, face: Option<Face>) -> Self {
        self.cull_mode = face;
        self
    }

    pub fn with_vertex_only(mut self) -> Self {
        self.vertex_only = true;
        self
    }

    /// Sets the shader for this pipeline (builder pattern)
    ///
    /// # Arguments
    /// * `shader` - Shader identifier
    pub fn with_shader(mut self, shader: &str) -> Self {
        self.shader = shader.to_string();
        self
    }

    /// Sets all bind group layouts at once (builder pattern)
    ///
    /// # Arguments
    /// * `layouts` - Vector of bind group layouts to use
    pub fn with_bind_group_layouts(mut self, layouts: Vec<BindGroupLayout>) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    /// Sets the depth texture for depth testing (builder pattern)
    ///
    /// # Arguments
    /// * `texture` - Depth texture to use for depth testing
    pub fn with_depth_stencil(mut self, texture: Texture) -> Self {
        self.depth_texture = Some(texture);
        self
    }

    /// Sets color targets for this pipeline (builder pattern)
    ///
    /// # Arguments
    /// * `targets` - Vector of color target states
    pub fn with_color_targets(mut self, targets: Vec<Option<ColorTargetState>>) -> Self {
        self.color_targets = targets;
        self
    }

    /// Sets primitive topology for this pipeline (builder pattern)
    ///
    /// # Arguments
    /// * `topology` - Primitive topology (TriangleList, etc.)
    pub fn with_primitive_topology(mut self, topology: PrimitiveTopology) -> Self {
        self.primitive_topology = topology;
        self
    }

    /// Configures pipeline for fullscreen quad rendering (no vertex buffers needed)
    ///
    /// Used for post-processing effects like blur passes
    pub fn with_no_vertex_buffers(mut self) -> Self {
        self.no_vertex_buffers = true;
        self
    }
}

/// Manages render pipelines with caching and lazy creation
///
/// Provides efficient pipeline management with features like:
/// - Lazy pipeline creation (only created when first requested)
/// - Shader hot-reloading in debug builds
/// - Shared bind group layout management
/// - Pipeline statistics and debugging
pub struct PipelineManager {
    device: Arc<Device>,
    pipelines: HashMap<String, RenderPipeline>,
    pipeline_configs: HashMap<String, PipelineConfig>,
    shader_modules: HashMap<String, ShaderModule>,
    shader_sources: HashMap<String, String>,
    common_layouts: HashMap<String, BindGroupLayout>,
    pending_pipelines: Vec<String>,
}

impl PipelineManager {
    /// Creates a new pipeline manager
    ///
    /// # Arguments
    /// * `device` - Shared wgpu device for creating resources
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            pipelines: HashMap::new(),
            pipeline_configs: HashMap::new(),
            shader_modules: HashMap::new(),
            shader_sources: HashMap::new(),
            common_layouts: HashMap::new(),
            pending_pipelines: Vec::new(),
        }
    }

    /// Registers a shared bind group layout
    ///
    /// Allows multiple pipelines to reference the same layout by name,
    /// reducing memory usage and improving performance.
    ///
    /// # Arguments
    /// * `name` - Identifier for this layout
    /// * `layout` - The bind group layout to register
    pub fn register_bind_group_layout(&mut self, name: &str, layout: BindGroupLayout) {
        self.common_layouts.insert(name.to_string(), layout);
    }

    /// Gets a registered bind group layout by name
    ///
    /// # Arguments
    /// * `name` - Layout identifier
    ///
    /// # Returns
    /// Reference to the layout if found
    pub fn get_bind_group_layout(&self, name: &str) -> Option<&BindGroupLayout> {
        self.common_layouts.get(name)
    }

    /// Registers a pipeline configuration without creating it
    ///
    /// Pipelines are created lazily when first requested via `get_pipeline()`.
    ///
    /// # Arguments
    /// * `name` - Unique identifier for this pipeline
    /// * `config` - Pipeline configuration
    pub fn register_pipeline(&mut self, name: &str, config: PipelineConfig) {
        self.pipeline_configs.insert(name.to_string(), config);
        self.pending_pipelines.push(name.to_string());
    }

    /// Loads and compiles a shader module
    ///
    /// Stores both the compiled module and source code for hot-reloading.
    ///
    /// # Arguments
    /// * `name` - Shader identifier
    /// * `source` - WGSL shader source code
    ///
    /// # Returns
    /// Result indicating success or compilation error
    pub fn load_shader(&mut self, name: &str, source: &str) -> Result<(), String> {
        let shader_module = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(source.into()),
        });

        self.shader_modules.insert(name.to_string(), shader_module);
        self.shader_sources
            .insert(name.to_string(), source.to_string());
        Ok(())
    }

    /// Gets or creates a pipeline (lazy loading)
    ///
    /// Returns an existing pipeline if available, otherwise creates it
    /// from the registered configuration.
    ///
    /// # Arguments
    /// * `name` - Pipeline identifier
    ///
    /// # Returns
    /// Reference to the pipeline if successful, None if config not found or creation failed
    pub fn get_pipeline(&mut self, name: &str) -> Option<&RenderPipeline> {
        if self.pipelines.contains_key(name) {
            return self.pipelines.get(name);
        }

        if let Some(config) = self.pipeline_configs.get(name).cloned() {
            match self.create_pipeline_from_config(name, &config) {
                Ok(pipeline) => {
                    self.pipelines.insert(name.to_string(), pipeline);
                    self.pending_pipelines.retain(|n| n != name);
                    return self.pipelines.get(name);
                }
                Err(e) => {
                    eprintln!("Failed to create pipeline '{}': {}", name, e);
                    return None;
                }
            }
        }

        None
    }

    /// Creates all pending pipelines immediately
    ///
    /// Useful for pre-loading pipelines or validating configurations.
    ///
    /// # Returns
    /// Result with vector of error messages if any pipelines failed to create
    pub fn create_all_pipelines(&mut self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let pending = self.pending_pipelines.clone();

        for name in pending {
            if let Some(config) = self.pipeline_configs.get(&name).cloned() {
                match self.create_pipeline_from_config(&name, &config) {
                    Ok(pipeline) => {
                        self.pipelines.insert(name.clone(), pipeline);
                        self.pending_pipelines.retain(|n| n != &name);
                    }
                    Err(e) => {
                        errors.push(format!("Pipeline '{}': {}", name, e));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Hot-reloads a shader and recreates affected pipelines
    ///
    /// Only available in debug builds for development workflow.
    ///
    /// # Arguments
    /// * `shader_name` - Shader to reload
    /// * `new_source` - Updated shader source code
    ///
    /// # Returns
    /// List of pipeline names that were recreated
    #[cfg(debug_assertions)]
    pub fn hot_reload_shader(
        &mut self,
        shader_name: &str,
        new_source: &str,
    ) -> Result<Vec<String>, String> {
        self.load_shader(shader_name, new_source)?;

        let mut affected_pipelines = Vec::new();
        for (pipeline_name, config) in &self.pipeline_configs {
            if config.shader == shader_name {
                affected_pipelines.push(pipeline_name.clone());
            }
        }

        // Recreate affected pipelines
        for pipeline_name in &affected_pipelines {
            if let Some(config) = self.pipeline_configs.get(pipeline_name).cloned() {
                match self.create_pipeline_from_config(pipeline_name, &config) {
                    Ok(pipeline) => {
                        self.pipelines.insert(pipeline_name.clone(), pipeline);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to recreate pipeline '{}' after shader reload: {}",
                            pipeline_name, e
                        );
                    }
                }
            }
        }

        Ok(affected_pipelines)
    }

    /// Creates a render pipeline from configuration
    fn create_pipeline_from_config(
        &self,
        name: &str,
        config: &PipelineConfig,
    ) -> Result<RenderPipeline, String> {
        let shader = self
            .shader_modules
            .get(&config.shader)
            .ok_or_else(|| format!("Shader '{}' not found", config.shader))?;

        let bind_group_layout_refs: Vec<&BindGroupLayout> =
            config.bind_group_layouts.iter().collect();
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(&format!("{} Layout", name)),
                bind_group_layouts: &bind_group_layout_refs,
                push_constant_ranges: &[],
            });

        // Handle vertex-only pipelines (like shadow pass)
        let fragment_state = if config.vertex_only {
            None // No fragment shader for vertex-only pipelines
        } else {
            Some(FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &config.color_targets,
                compilation_options: PipelineCompilationOptions::default(),
            })
        };

        // Handle vertex buffers - use empty slice for fullscreen quads
        let vertex_buffers: &[VertexBufferLayout] = if config.no_vertex_buffers {
            &[] // No vertex buffers for fullscreen quads
        } else {
            &[Vertex3D::desc()]
        };

        // Handle depth stencil - only if depth texture is provided
        let depth_stencil = config
            .depth_texture
            .as_ref()
            .map(|texture| DepthStencilState {
                format: texture.format(),
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            });

        let pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(&config.label),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    buffers: vertex_buffers, // Now respects no_vertex_buffers flag
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: fragment_state, // Respects vertex_only flag
                primitive: PrimitiveState {
                    topology: config.primitive_topology,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: config.cull_mode,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil, // Now optional
                multisample: config.multisample,
                multiview: None,
                cache: None,
            });

        Ok(pipeline)
    }

    /// Returns pipeline manager statistics
    ///
    /// Useful for debugging and performance monitoring.
    pub fn get_stats(&self) -> PipelineStats {
        PipelineStats {
            total_pipelines: self.pipelines.len(),
            pending_pipelines: self.pending_pipelines.len(),
            loaded_shaders: self.shader_modules.len(),
            common_layouts: self.common_layouts.len(),
        }
    }

    /// Lists all registered pipeline names
    pub fn list_pipelines(&self) -> Vec<&String> {
        self.pipeline_configs.keys().collect()
    }

    /// Checks if a pipeline is registered
    ///
    /// # Arguments
    /// * `name` - Pipeline identifier
    ///
    /// # Returns
    /// True if pipeline config exists (created or pending)
    pub fn has_pipeline(&self, name: &str) -> bool {
        self.pipeline_configs.contains_key(name)
    }
}

/// Statistics about pipeline manager state
#[derive(Debug)]
pub struct PipelineStats {
    pub total_pipelines: usize,
    pub pending_pipelines: usize,
    pub loaded_shaders: usize,
    pub common_layouts: usize,
}
