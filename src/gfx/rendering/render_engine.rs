//! WGPU-based rendering engine for the Haggis 3D engine
//!
//! Provides high-level rendering functionality built on top of wgpu, including
//! pipeline management, depth testing, shadow mapping, and UI overlay support.

use std::{iter, sync::Arc};
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

/// Core rendering engine managing GPU resources and draw calls
///
/// The RenderEngine handles all low-level graphics operations including:
/// - Surface and device management
/// - Pipeline creation and management  
/// - Depth buffer handling
/// - Shadow mapping
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
    shadow_texture: TextureResource,
    shadow_bind_group: wgpu::BindGroup,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    light_config: LightConfig,
}

impl RenderEngine {
    /// Creates a new render engine for the given window
    ///
    /// Initializes wgpu with default settings, creates depth and shadow buffers,
    /// and sets up the PBR rendering pipeline with shadow mapping support.
    ///
    /// # Arguments
    /// * `window` - Window surface target for rendering
    /// * `width` - Initial surface width in pixels
    /// * `height` - Initial surface height in pixels
    ///
    /// # Returns
    /// Configured RenderEngine ready for rendering with shadows
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
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create depth texture for main rendering
        let depth_texture =
            TextureResource::create_depth_texture(&device, &config, "depth_texture");

        // Create shadow map texture
        let shadow_size = 2048u32;
        let shadow_texture = TextureResource::create_shadow_map(&device, shadow_size);

        // Create shadow bind group layout
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Create shadow bind group
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_texture.sampler),
                },
            ],
        });

        // Initialize global uniform bindings for camera and lighting
        let light_config = LightConfig {
            position: [8.0, 8.0, 8.0], // More diagonal, better for seeing shadows
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
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

        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Material Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Wrap device and queue in Arc for pipeline manager
        let device_handle: Arc<Device> = device.into();
        let queue_handle: Arc<wgpu::Queue> = queue.into();
        let mut pipeline_manager = PipelineManager::new(device_handle.clone());

        // Load shaders
        let _ = pipeline_manager.load_shader("default", include_str!("pbr.wgsl"));
        let _ = pipeline_manager.load_shader("shadow", include_str!("shadow_pass.wgsl"));

        // Register PBR pipeline with shadow support
        pipeline_manager.register_pipeline(
            "PBR",
            PipelineConfig::default()
                .with_shader("default")
                .with_depth_stencil(depth_texture.texture.clone())
                .with_bind_group_layouts(vec![
                    global_bindings.bind_group_layouts().clone(),
                    transform_bind_group_layout.clone(),
                    material_bind_group_layout.clone(),
                    shadow_bind_group_layout.clone(), // Shadow map binding
                ]),
        );

        // Register shadow pipeline (depth-only rendering)
        pipeline_manager.register_pipeline(
            "Shadow",
            PipelineConfig::default()
                .with_label("SHADOW")
                .with_shader("shadow")
                .with_depth_stencil(shadow_texture.texture.clone())
                .with_bind_group_layouts(vec![
                    global_bindings.bind_group_layouts().clone(), // For light matrix
                    transform_bind_group_layout,                  // For model matrix
                ])
                .with_vertex_only(), // No fragment shader needed
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
            shadow_texture,
            shadow_bind_group,
            shadow_bind_group_layout,
            light_config,
        }
    }

    /// Renders a frame with shadow mapping
    ///
    /// Performs two-pass rendering: first renders the scene from the light's
    /// perspective to create a shadow map, then renders the scene normally
    /// with shadows applied.
    ///
    /// # Arguments
    /// * `scene` - Scene containing objects to render
    pub fn render_frame(&mut self, scene: &Scene) {
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

        // PASS 1: Shadow mapping (render from light's perspective)
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[], // No color output
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_texture.view,
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

                // Render all objects to shadow map
                for object in scene.objects.iter() {
                    if object.visible {
                        shadow_pass.draw_object(object);
                    }
                }
            } else {
                println!("-----------cry--------------")
            }
        }

        // PASS 2: Main rendering (with shadows)
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
            render_pass.set_bind_group(3, &self.shadow_bind_group, &[]); // Bind shadow map

            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                let default_material = scene.material_manager.get_default_material();

                match default_material.get_bind_group() {
                    Some(bind_group) => {
                        render_pass.set_bind_group(2, bind_group, &[]);

                        for object in scene.objects.iter() {
                            if object.visible {
                                render_pass.draw_object(object);
                            }
                        }
                    }
                    None => {
                        for material_name in scene.material_manager.list_materials() {
                            let material =
                                scene.material_manager.get_material(material_name).unwrap();
                            if !material.get_bind_group().is_some() {
                                println!(
                                    "  ❌ Material '{}' missing bind group during render",
                                    material.name
                                );
                            }
                        }
                    }
                }
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        surface_texture.present();
    }

    /// Renders a frame with 3D scene, shadows, and UI overlay
    ///
    /// Performs shadow mapping, then renders the 3D scene with shadows,
    /// and finally calls the UI callback to render interface elements on top.
    ///
    /// # Arguments
    /// * `scene` - Scene containing 3D objects to render
    /// * `ui_callback` - Function that renders UI elements using the command encoder
    pub fn render_frame_with_ui<F>(&mut self, scene: &Scene, ui_callback: F)
    where
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

        // PASS 1: Shadow mapping
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_texture.view,
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
            }
        }

        // PASS 2: Main 3D rendering with shadows
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D Render Pass"),
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
            render_pass.set_bind_group(3, &self.shadow_bind_group, &[]); // Bind shadow map

            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                for object in scene.objects.iter() {
                    if object.visible {
                        let material = scene.get_material_for_object(object);

                        if let Some(material_bind_group) = material.get_bind_group() {
                            render_pass.set_bind_group(2, material_bind_group, &[]);
                            render_pass.draw_object(object);
                        } else {
                            println!(
                                "Skipping '{}' - material '{}' has no GPU resources",
                                object.name, material.name
                            );
                        }
                    }
                }
            }
        }

        // PASS 3: UI overlay
        ui_callback(
            &self.device,
            &self.queue,
            &mut encoder,
            &surface_texture_view,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    /// Updates camera and light uniform buffers
    ///
    /// Should be called each frame with updated camera data and optionally
    /// new light configuration for shadow mapping.
    ///
    /// # Arguments
    /// * `camera_uniform` - Updated camera uniform data
    /// * `light_config` - Optional new light configuration
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
}
