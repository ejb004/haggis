//! WGPU-based rendering engine for the Haggis 3D engine
//!
//! Provides high-level rendering functionality built on top of wgpu, including
//! pipeline management, depth testing, and UI overlay support.

use std::{iter, sync::Arc};
use wgpu::{Device, TextureFormat};

use crate::gfx::{
    camera::camera_utils::CameraUniform,
    resources::{
        global_bindings::{update_global_ubo, GlobalBindings, GlobalUBO},
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
}

impl RenderEngine {
    /// Creates a new render engine for the given window
    ///
    /// Initializes wgpu with default settings, creates a depth buffer,
    /// and sets up the default PBR rendering pipeline.
    ///
    /// # Arguments
    /// * `window` - Window surface target for rendering
    /// * `width` - Initial surface width in pixels
    /// * `height` - Initial surface height in pixels
    ///
    /// # Returns
    /// Configured RenderEngine ready for rendering
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
        let depth_texture =
            TextureResource::create_depth_texture(&device, &config, "depth_texture");

        // Initialize global uniform bindings for camera and lighting
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

        let device_handle: Arc<Device> = device.into();
        let mut pipeline_manager = PipelineManager::new(device_handle.clone());

        // Load default PBR shader and create pipeline
        let _ = pipeline_manager.load_shader("default", include_str!("pbr.wgsl"));

        pipeline_manager.register_pipeline(
            "PBR",
            PipelineConfig::default()
                .with_shader("default")
                .with_depth_stencil(depth_texture.texture.clone())
                .with_bind_group_layouts(vec![
                    global_bindings.bind_group_layouts().clone(),
                    transform_bind_group_layout,
                    material_bind_group_layout,
                ]),
        );

        let _ = pipeline_manager.create_all_pipelines();

        RenderEngine {
            device: device_handle,
            config,
            format,
            surface,
            queue: queue.into(),
            depth_texture,
            pipeline_manager,
            global_bindings,
            global_ubo,
        }
    }

    /// Renders a frame with only 3D scene content
    ///
    /// Clears the color and depth buffers, then renders all visible objects
    /// in the scene using the default PBR pipeline.
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

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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

            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                // Debug material access during rendering

                let default_material = scene.material_manager.get_default_material();

                // Test get_bind_group directly
                match default_material.get_bind_group() {
                    Some(bind_group) => {
                        render_pass.set_bind_group(2, bind_group, &[]);

                        // Draw all objects with the same material
                        for object in scene.objects.iter() {
                            render_pass.draw_object(object);
                        }
                    }
                    None => {
                        // Let's check ALL materials during render time
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

    /// Renders a frame with 3D scene and UI overlay
    ///
    /// First renders the 3D scene to the color buffer, then calls the provided
    /// UI callback to render interface elements on top.
    ///
    /// # Arguments
    /// * `scene` - Scene containing 3D objects to render
    /// * `ui_callback` - Function that renders UI elements using the command encoder
    ///
    /// # Type Parameters
    /// * `F` - UI callback function signature
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

        // Render 3D scene first
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

            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                for object in scene.objects.iter() {
                    if object.visible {
                        // Get the SPECIFIC material for THIS object
                        let material = scene.get_material_for_object(object);

                        if let Some(material_bind_group) = material.get_bind_group() {
                            // Bind THIS object's material
                            render_pass.set_bind_group(2, material_bind_group, &[]);

                            // Draw with this material
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

        // Render UI overlay on top of 3D scene

        ui_callback(
            &self.device,
            &self.queue,
            &mut encoder,
            &surface_texture_view,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    /// Updates camera uniform buffer
    ///
    /// Should be called each frame with updated camera data to ensure
    /// correct view and projection matrices.
    ///
    /// # Arguments
    /// * `camera_uniform` - Updated camera uniform data
    pub fn update(&mut self, camera_uniform: CameraUniform) {
        update_global_ubo(&mut self.global_ubo, &self.queue, camera_uniform);
    }

    /// Resizes the render engine surface and recreates depth buffer
    ///
    /// Validates dimensions and clamps to minimum viable size to prevent
    /// crashes. Recreates the depth texture to match new dimensions.
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
