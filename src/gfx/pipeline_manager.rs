use std::{collections::HashMap, sync::Arc};
use wgpu::*;

use super::vertex::Vertex3D;

/// Configuration for creating a render pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub label: String,
    pub shader: String, // Path or identifier
    pub bind_group_layouts: Vec<BindGroupLayout>,
    pub primitive_topology: PrimitiveTopology,
    pub cull_mode: Option<Face>,
    pub depth_texture: Option<Texture>,
    pub multisample: MultisampleState,
    pub color_targets: Vec<Option<ColorTargetState>>,
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
        }
    }
}

impl PipelineConfig {
    /// Create a new config with a specific shader
    pub fn default_with_shader(shader: &str) -> Self {
        Self {
            shader: shader.to_string(),
            ..Default::default()
        }
    }

    /// Set the shader
    pub fn with_shader(mut self, shader: &str) -> Self {
        self.shader = shader.to_string();
        self
    }

    /// Set all bind group layouts at once
    pub fn with_bind_group_layouts(mut self, layouts: Vec<BindGroupLayout>) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    /// Set depth stencil state
    pub fn with_depth_stencil(mut self, texture: Texture) -> Self {
        self.depth_texture = Some(texture);
        self
    }
}

/// Manages render pipelines with caching and hot-reloading capabilities
pub struct PipelineManager {
    device: Arc<Device>,

    // Pipeline storage
    pipelines: HashMap<String, RenderPipeline>,
    pipeline_configs: HashMap<String, PipelineConfig>,

    // Shader management
    shader_modules: HashMap<String, ShaderModule>,
    shader_sources: HashMap<String, String>, // For hot-reloading

    // Common bind group layouts (shared across pipelines)
    common_layouts: HashMap<String, BindGroupLayout>,

    // Pipeline creation queue for lazy loading
    pending_pipelines: Vec<String>,
}

impl PipelineManager {
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

    /// Register a common bind group layout that can be shared across pipelines
    pub fn register_bind_group_layout(&mut self, name: &str, layout: BindGroupLayout) {
        self.common_layouts.insert(name.to_string(), layout);
    }

    /// Get a registered bind group layout
    pub fn get_bind_group_layout(&self, name: &str) -> Option<&BindGroupLayout> {
        self.common_layouts.get(name)
    }

    /// Register a pipeline configuration (doesn't create it yet)
    pub fn register_pipeline(&mut self, name: &str, config: PipelineConfig) {
        self.pipeline_configs.insert(name.to_string(), config);
        self.pending_pipelines.push(name.to_string());
    }

    /// Load and compile a shader module
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

    /// Get or create a pipeline (lazy loading)
    pub fn get_pipeline(&mut self, name: &str) -> Option<&RenderPipeline> {
        // If pipeline exists, return it
        if self.pipelines.contains_key(name) {
            return self.pipelines.get(name);
        }

        // If config exists but pipeline doesn't, create it
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

    /// Force create all pending pipelines
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

    /// Hot-reload a shader and recreate affected pipelines
    #[cfg(debug_assertions)]
    pub fn hot_reload_shader(
        &mut self,
        shader_name: &str,
        new_source: &str,
    ) -> Result<Vec<String>, String> {
        // Update shader module
        self.load_shader(shader_name, new_source)?;

        // Find pipelines that use this shader
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

    /// Create a render pipeline from configuration
    fn create_pipeline_from_config(
        &self,
        name: &str,
        config: &PipelineConfig,
    ) -> Result<RenderPipeline, String> {
        // Get vertex shader
        let shader = self
            .shader_modules
            .get(&config.shader)
            .ok_or_else(|| format!("Vertex shader '{}' not found", config.shader))?;

        // Create pipeline layout
        let bind_group_layout_refs: Vec<&BindGroupLayout> =
            config.bind_group_layouts.iter().collect();
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(&format!("{} Layout", name)),
                bind_group_layouts: &bind_group_layout_refs,
                push_constant_ranges: &[],
            });

        // Create render pipeline
        let pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(&config.label),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex3D::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    targets: &config.color_targets,
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: config.primitive_topology,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: config.cull_mode,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(DepthStencilState {
                    format: config.depth_texture.clone().unwrap().format(),
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: config.multisample,
                multiview: None,
                cache: None,
            });

        Ok(pipeline)
    }

    /// Get pipeline statistics
    pub fn get_stats(&self) -> PipelineStats {
        PipelineStats {
            total_pipelines: self.pipelines.len(),
            pending_pipelines: self.pending_pipelines.len(),
            loaded_shaders: self.shader_modules.len(),
            common_layouts: self.common_layouts.len(),
        }
    }

    /// List all registered pipeline names
    pub fn list_pipelines(&self) -> Vec<&String> {
        self.pipeline_configs.keys().collect()
    }

    /// Check if a pipeline exists (created or pending)
    pub fn has_pipeline(&self, name: &str) -> bool {
        self.pipeline_configs.contains_key(name)
    }

    /// Print all registered pipelines to the terminal
    pub fn list_pipelines_to_terminal(&self) {
        println!("=== PIPELINE MANAGER STATUS ===");

        let stats = self.get_stats();
        println!("Total registered pipelines: {}", stats.total_pipelines);
        println!("Pending (not yet created): {}", stats.pending_pipelines);
        println!("Loaded shaders: {}", stats.loaded_shaders);
        println!("Common bind group layouts: {}", stats.common_layouts);
        println!();

        if self.pipeline_configs.is_empty() {
            println!("No pipelines registered.");
            return;
        }

        println!("REGISTERED PIPELINES:");
        println!(
            "{:<20} {:<15} {:<20} {:<10}",
            "Name", "Status", "Shader", "Layouts"
        );
        println!("{:-<65}", "");

        for (name, config) in &self.pipeline_configs {
            let status = if self.pipelines.contains_key(name) {
                "✓ Created"
            } else {
                "⏳ Pending"
            };

            let layout_count = config.bind_group_layouts.len();

            println!(
                "{:<20} {:<15} {:<20} {:<10}",
                name, status, config.shader, layout_count
            );
        }

        if !self.pending_pipelines.is_empty() {
            println!();
            println!("PENDING PIPELINES:");
            for pending in &self.pending_pipelines {
                println!("  - {}", pending);
            }
        }

        if !self.shader_modules.is_empty() {
            println!();
            println!("LOADED SHADERS:");
            for shader_name in self.shader_modules.keys() {
                println!("  - {}", shader_name);
            }
        }

        if !self.common_layouts.is_empty() {
            println!();
            println!("COMMON BIND GROUP LAYOUTS:");
            for layout_name in self.common_layouts.keys() {
                println!("  - {}", layout_name);
            }
        }

        println!("=== END PIPELINE STATUS ===\n");
    }
}

/// Statistics about the pipeline manager
#[derive(Debug)]
pub struct PipelineStats {
    pub total_pipelines: usize,
    pub pending_pipelines: usize,
    pub loaded_shaders: usize,
    pub common_layouts: usize,
}
