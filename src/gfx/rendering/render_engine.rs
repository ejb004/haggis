//! WGPU-based rendering engine for the Haggis 3D engine
//!
//! Provides high-level rendering functionality built on top of wgpu, including
//! pipeline management, depth testing, shadow mapping with blur, and UI overlay support.

use std::sync::Arc;
use wgpu::{Device, TextureFormat};

use crate::gfx::{
    camera::camera_utils::CameraUniform,
    resources::{
        global_bindings::{update_global_ubo_with_light, GlobalBindings, GlobalUBO, LightConfig},
        texture_resource::TextureResource,
    },
    scene::{object::DrawObject, scene::Scene},
};

use super::pipeline_manager::{PipelineConfig, PipelineManager};
use super::shadow_cache::ShadowCache;
use super::visualization_renderer::{VisualizationPlane, VisualizationRenderer};
use super::instanced_grid::InstancedGrid;

/// Core rendering engine managing GPU resources and draw calls
///
/// The RenderEngine handles all low-level graphics operations including:
/// - Surface and device management
/// - Pipeline creation and management  
/// - Depth buffer handling
/// - Shadow mapping with gaussian blur
/// - Camera uniform updates
/// - UI overlay rendering
pub struct RenderEngine {
    surface: wgpu::Surface<'static>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    depth_texture: TextureResource,
    format: TextureFormat,
    pub pipeline_manager: PipelineManager,
    global_ubo: GlobalUBO,
    global_bindings: GlobalBindings,

    // Shadow mapping resources
    shadow_depth_texture: TextureResource, // Original depth shadow map
    shadow_color_view: wgpu::TextureView,
    blurred_shadow_view: wgpu::TextureView,

    // Bind groups and layouts
    shadow_bind_group: wgpu::BindGroup, // For final rendering
    blur_bind_group: wgpu::BindGroup,   // For blur pass

    light_config: LightConfig,

    // Shadow map caching system
    shadow_cache: ShadowCache,

    // Visualization rendering system
    visualization_renderer: VisualizationRenderer,

    // Instanced grid rendering system
    instanced_grid: Option<InstancedGrid>,
}

impl RenderEngine {
    /// Creates a new render engine for the given window
    ///
    /// Initializes wgpu with default settings, creates depth and shadow buffers,
    /// sets up the PBR rendering pipeline with blurred shadow mapping support.
    ///
    /// # Arguments
    /// * `window` - Window surface target for rendering
    /// * `width` - Initial surface width in pixels
    /// * `height` - Initial surface height in pixels
    ///
    /// # Returns
    /// Configured RenderEngine ready for rendering with blurred shadows
    ///
    /// # Panics
    /// Panics if unable to create wgpu adapter or device
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> RenderEngine {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");

        let (device, queue) = {
            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("WGPU Device"),
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits {
                        max_texture_dimension_2d: 4096,
                        ..wgpu::Limits::downlevel_defaults()
                    },
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                })
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);
        let format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Immediate, // High performance by default
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create depth texture for main rendering
        let depth_texture =
            TextureResource::create_depth_texture(&device, &config, "depth_texture");

        let shadow_size = 4096u32; // Higher resolution for better contact shadows

        // 1. Create depth shadow map (for initial shadow rendering)
        let shadow_depth_texture = TextureResource::create_shadow_map(&device, shadow_size);

        // 2. Create color texture for depth-to-color conversion
        let shadow_color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Color Texture"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, // BOTH flags needed!
            view_formats: &[],
        });

        let shadow_color_view =
            shadow_color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 3. Create blurred shadow texture
        let blurred_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Blurred Shadow Texture"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let blurred_shadow_view =
            blurred_shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 4. Create samplers

        let color_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            ..Default::default()
        });

        // 5. Create bind group layouts

        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blur Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true }, // Must match Rgba8Unorm
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shadow_final_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Final Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth, // Depth texture
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison), // Comparison sampler
                        count: None,
                    },
                ],
            });

        // 6. Create bind groups

        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur Bind Group"),
            layout: &blur_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&color_sampler),
                },
            ],
        });

        // Create comparison sampler for hardware shadow mapping
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // Hardware shadow comparison
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            ..Default::default()
        });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_final_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_depth_texture.view), // Use depth texture
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler), // Use comparison sampler
                },
            ],
        });

        // Initialize global uniform bindings for camera and lighting
        let light_config = LightConfig {
            position: [20.0, 20.0, 20.0],
            color: [1.0, 1.0, 1.0],
            intensity: 10000.0,
        };
        let global_ubo = GlobalUBO::new(&device);
        let mut global_bindings = GlobalBindings::new(&device);
        global_bindings.create_bind_group(&device, &global_ubo);

        // Create transform bind group layout for per-object transforms
        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Transform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create a temporary material bindings to get the layout that matches our material system
        let temp_material_bindings =
            crate::gfx::resources::material::MaterialBindings::new(&device);
        let material_bind_group_layout = temp_material_bindings.bind_group_layouts().clone();

        // Create visualization renderer (before device is moved)
        let visualization_renderer = VisualizationRenderer::new(&device, format);

        // Wrap device and queue in Arc for pipeline manager
        let device_handle: Arc<Device> = device.into();
        let queue_handle: Arc<wgpu::Queue> = queue.into();
        let mut pipeline_manager = PipelineManager::new(device_handle.clone());

        // Load shaders
        let _ = pipeline_manager.load_shader("default", include_str!("pbr.wgsl"));
        let _ = pipeline_manager.load_shader("shadow", include_str!("shadow_pass.wgsl"));
        let _ = pipeline_manager.load_shader("blur", include_str!("shadow_blur.wgsl"));

        // Register shadow depth pass - NO CULLING to prevent light leaks
        pipeline_manager.register_pipeline(
            "Shadow",
            PipelineConfig::default()
                .with_label("SHADOW")
                .with_shader("shadow")
                .with_depth_stencil(shadow_depth_texture.texture.clone())
                .with_cull_mode(None) // No culling - render both front and back faces
                .with_bind_group_layouts(vec![
                    global_bindings.bind_group_layouts().clone(),
                    transform_bind_group_layout.clone(),
                ])
                .with_color_targets(vec![]), // No color targets - depth only
        );

        // Register blur pass
        pipeline_manager.register_pipeline(
            "Blur",
            PipelineConfig::default()
                .with_label("BLUR")
                .with_shader("blur")
                .with_bind_group_layouts(vec![blur_layout])
                .with_color_targets(vec![Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })])
                .with_cull_mode(None)
                .with_primitive_topology(wgpu::PrimitiveTopology::TriangleList)
                .with_no_vertex_buffers(), // This is crucial!
        );

        // Register PBR pipeline with shadow support
        pipeline_manager.register_pipeline(
            "PBR",
            PipelineConfig::default()
                .with_shader("default")
                .with_depth_stencil(depth_texture.texture.clone())
                .with_bind_group_layouts(vec![
                    global_bindings.bind_group_layouts().clone(),
                    transform_bind_group_layout,
                    material_bind_group_layout,
                    shadow_final_layout,
                ]),
        );

        let _ = pipeline_manager.create_all_pipelines();

        RenderEngine {
            device: device_handle,
            config,
            format,
            surface,
            queue: queue_handle,
            depth_texture,
            pipeline_manager,
            global_bindings,
            global_ubo,
            shadow_depth_texture,
            shadow_color_view,
            blurred_shadow_view,
            shadow_bind_group,
            blur_bind_group,
            light_config,
            shadow_cache: ShadowCache::new(),
            visualization_renderer,
            instanced_grid: None,
        }
    }

    /// Renders a frame with optional UI overlay and visualization planes
    ///
    /// Performs multi-pass rendering: shadow mapping, depth-to-color conversion,
    /// blur, main scene rendering, visualization rendering, and optional UI overlay.
    ///
    /// # Arguments
    /// * `scene` - Scene containing objects to render
    /// * `visualization_planes` - Visualization planes with simulation data
    /// * `ui_callback` - Optional function that renders UI elements
    pub fn render_frame<F>(
        &mut self,
        scene: &Scene,
        visualization_planes: &[VisualizationPlane],
        ui_callback: Option<F>,
    ) where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView),
    {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // PASS 1: Shadow mapping (render to depth AND color for depth extraction)
        // Check if shadow map needs to be regenerated using cache
        let needs_shadow_update = self.shadow_cache.needs_update(&self.light_config, &scene.objects);
        
        if needs_shadow_update {
            // #[cfg(debug_assertions)]
            // println!("🌒 Shadow map cache MISS - regenerating shadows");
            
            // Alternative: Environment variable debug
            if std::env::var("HAGGIS_SHADOW_DEBUG").is_ok() {
                eprintln!("SHADOW DEBUG: Regenerating shadow map");
            }
            
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Depth Pass"),
                color_attachments: &[], // No color attachment - depth only
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            shadow_pass.set_bind_group(0, self.global_bindings.bind_groups(), &[]);

            if let Some(shadow_pipeline) = self.pipeline_manager.get_pipeline("Shadow") {
                shadow_pass.set_pipeline(shadow_pipeline);

                for object in scene.objects.iter() {
                    if object.visible {
                        shadow_pass.draw_object(object);
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                println!("❌ Shadow pipeline not found!");
            }
        }

        // PASS 2: Convert depth to color (SKIP - we're already rendering depth as color)
        // The shadow pass now outputs depth directly to the shadow_color_texture

        // PASS 3: Skip blur pass completely to test if it's causing stripes
        if needs_shadow_update {
            // Skip blur pass - use shadow_color_view directly
            // Mark shadow cache as valid after successful update
            self.shadow_cache.mark_valid(&self.light_config, &scene.objects);
        } else {
            // #[cfg(debug_assertions)]
            // println!("✨ Shadow map cache HIT - skipping shadow passes");
            
            if std::env::var("HAGGIS_SHADOW_DEBUG").is_ok() {
                eprintln!("SHADOW DEBUG: Using cached shadow map");
            }
        }

        // PASS 4: Main rendering with shadows
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(0, self.global_bindings.bind_groups(), &[]);
            render_pass.set_bind_group(3, &self.shadow_bind_group, &[]);

            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                for object in scene.objects.iter() {
                    if object.visible {
                        let material = scene.get_material_for_object(object);

                        if let Some(material_bind_group) = material.get_bind_group() {
                            render_pass.set_bind_group(2, material_bind_group, &[]);
                            render_pass.draw_object(object);
                        } else {
                            #[cfg(debug_assertions)]
                            println!(
                                "Skipping '{}' - material '{}' has no GPU resources",
                                object.name, material.name
                            );
                        }
                    }
                }
            }

            // Render instanced grid after scene objects (same render pass for proper depth testing)
            self.render_instanced_grid(&mut render_pass);
        }

        // PASS 5: Visualization rendering (separate from scene objects)
        if !visualization_planes.is_empty() {
            // Update visualization camera with scene camera
            self.visualization_renderer
                .update_camera(&self.queue, scene.camera_manager.get_view_proj_matrix());

            // Render visualization planes with their simulation data
            self.visualization_renderer.render_visualization_pass(
                &mut encoder,
                &surface_texture_view,
                &self.depth_texture.view,
                visualization_planes,
                &self.queue,
            );
        }

        // PASS 6: UI overlay (if provided)
        if let Some(ui_callback) = ui_callback {
            ui_callback(
                &self.device,
                &self.queue,
                &mut encoder,
                &surface_texture_view,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    /// Convenience method for rendering without UI or visualizations
    pub fn render_frame_simple(&mut self, scene: &Scene) {
        self.render_frame(
            scene,
            &[], // No visualization planes
            None::<fn(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView)>,
        );
    }

    /// Convenience method for rendering with UI but no visualizations
    pub fn render_frame_with_ui<F>(&mut self, scene: &Scene, ui_callback: F)
    where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView),
    {
        self.render_frame(scene, &[], Some(ui_callback));
    }

    /// Convenience method for rendering with visualizations but no UI
    pub fn render_frame_with_visualizations(
        &mut self,
        scene: &Scene,
        visualization_planes: &[VisualizationPlane],
    ) {
        self.render_frame(
            scene,
            visualization_planes,
            None::<fn(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView)>,
        );
    }

    /// Convenience method for rendering with both visualizations and UI
    pub fn render_frame_with_visualizations_and_ui<F>(
        &mut self,
        scene: &Scene,
        visualization_planes: &[VisualizationPlane],
        ui_callback: F,
    ) where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView),
    {
        self.render_frame(scene, visualization_planes, Some(ui_callback));
    }

    /// Updates camera and light uniform buffers
    ///
    /// Should be called each frame with updated camera data and optionally
    /// new light configuration for shadow mapping.
    ///
    /// # Arguments
    /// * `camera_uniform` - Updated camera uniform data
    pub fn update(&mut self, camera_uniform: CameraUniform) {
        update_global_ubo_with_light(
            &mut self.global_ubo,
            &self.queue,
            camera_uniform,
            self.light_config,
        );
    }

    /// Updates the light configuration
    ///
    /// Changes the light position, color, and intensity for shadow mapping.
    /// The light matrix will be recalculated on the next update() call.
    ///
    /// # Arguments
    /// * `light_config` - New light configuration
    pub fn set_light(&mut self, light_config: LightConfig) {
        self.light_config = light_config;
    }

    /// Gets the current light configuration
    pub fn get_light(&self) -> LightConfig {
        self.light_config
    }

    /// Resizes the render engine surface and recreates depth buffer
    ///
    /// Validates dimensions and clamps to minimum viable size to prevent
    /// crashes. Recreates the depth texture to match new dimensions.
    /// Shadow map size remains unchanged.
    ///
    /// # Arguments
    /// * `width` - New surface width in pixels
    /// * `height` - New surface height in pixels
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let safe_width = width.max(1);
        let safe_height = height.max(1);

        self.config.width = safe_width;
        self.config.height = safe_height;

        // Reconfigure surface with new dimensions
        self.surface.configure(&self.device, &self.config);

        // Recreate depth texture to match new surface size
        self.depth_texture =
            TextureResource::create_depth_texture(&self.device, &self.config, "depth_texture");

        // Note: Shadow map doesn't need to be recreated as it has fixed resolution
    }

    /// Returns current surface dimensions
    ///
    /// Used for UI scaling and camera aspect ratio calculations.
    ///
    /// # Returns
    /// Tuple of (width, height) in pixels
    pub fn get_surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Returns reference to the wgpu device
    ///
    /// Used for creating GPU resources like buffers and textures.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns reference to the wgpu command queue
    ///
    /// Used for submitting GPU commands and updating buffers.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns the surface texture format
    ///
    /// Used for creating compatible render targets and UI systems.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Invalidates the shadow map cache, forcing regeneration on next frame
    ///
    /// Use this when you know that shadow-affecting changes have occurred
    /// that the cache might not have detected automatically.
    pub fn invalidate_shadow_cache(&mut self) {
        self.shadow_cache.invalidate();
    }

    /// Returns whether the shadow cache is currently valid
    ///
    /// A valid cache means the shadow map does not need to be regenerated.
    pub fn is_shadow_cache_valid(&self) -> bool {
        self.shadow_cache.is_valid()
    }

    /// Clears all shadow cache state
    ///
    /// This forces a complete cache rebuild from scratch.
    pub fn clear_shadow_cache(&mut self) {
        self.shadow_cache.clear();
    }

    /// Gets shadow cache statistics for debugging and performance monitoring
    ///
    /// Returns information about cache state, tracked objects, and shadow bounds.
    pub fn get_shadow_cache_stats(&self) -> super::shadow_cache::ShadowCacheStats {
        self.shadow_cache.get_stats()
    }

    /// Initialize the instanced grid system
    ///
    /// Creates a new instanced grid renderer with the specified maximum instance count.
    /// This should be called after the render engine is created and before rendering.
    pub fn initialize_instanced_grid(&mut self, max_instances: u32) {
        let mut grid = InstancedGrid::new(&self.device, max_instances);
        grid.initialize_pipeline(&self.device, self.format, &self.global_bindings);
        self.instanced_grid = Some(grid);
    }

    /// Get a mutable reference to the instanced grid
    ///
    /// Returns None if the instanced grid hasn't been initialized yet.
    pub fn instanced_grid_mut(&mut self) -> Option<&mut InstancedGrid> {
        self.instanced_grid.as_mut()
    }

    /// Get a reference to the instanced grid
    ///
    /// Returns None if the instanced grid hasn't been initialized yet.
    pub fn instanced_grid(&self) -> Option<&InstancedGrid> {
        self.instanced_grid.as_ref()
    }

    /// Update the instanced grid with new data (convenience method that handles borrow issues)
    ///
    /// This method combines enable/disable and update operations to avoid borrow checker conflicts.
    /// If the instanced grid hasn't been initialized yet, it will be automatically created.
    pub fn update_instanced_grid_data(&mut self, instances: &[(cgmath::Vector3<f32>, f32, cgmath::Vector4<f32>)]) {
        // Lazy initialization: only create instanced grid when first used
        if self.instanced_grid.is_none() {
            self.initialize_instanced_grid(8192);
        }

        if let Some(ref mut grid) = self.instanced_grid {
            let enabled = !instances.is_empty();
            grid.set_enabled(enabled);
            if enabled {
                grid.update(&self.queue, instances);
            }
        }
    }

    /// Set VSync (vertical synchronization) state
    ///
    /// When VSync is enabled, rendering is synchronized to the display refresh rate.
    /// When disabled, the application can achieve consistent frame times through framerate limiting.
    ///
    /// # Arguments
    /// * `enable` - Whether to enable VSync
    pub fn set_vsync(&mut self, enable: bool) {
        // Update the surface configuration with new present mode
        self.config.present_mode = if enable {
            wgpu::PresentMode::Fifo        // VSync enabled
        } else {
            wgpu::PresentMode::Immediate   // VSync disabled, immediate presentation
        };
        
        // Reconfigure surface with new present mode
        self.surface.configure(&self.device, &self.config);
    }

    /// Render the instanced grid during the main render pass
    ///
    /// This should be called during the main rendering phase after scene objects
    /// but before UI rendering for proper depth testing.
    pub fn render_instanced_grid<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(ref grid) = self.instanced_grid {
            grid.render(render_pass, self.global_bindings.bind_groups());
        }
    }
}
