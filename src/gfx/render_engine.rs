use std::{iter, os::unix::fs::PermissionsExt, sync::Arc};

use cgmath::num_traits::PrimInt;
use wgpu::{
    BindGroupLayout, Buffer, DepthStencilState, Device, FrontFace, RenderPipeline, TextureFormat,
    TextureUsages,
};

use super::{
    camera::camera_utils::CameraUniform,
    global_bindings::{update_global_ubo, GlobalBindings, GlobalUBO},
    object::DrawObject,
    pipeline_manager::{self, PipelineConfig, PipelineManager},
    scene::Scene,
    texture_resource::TextureResource,
    vertex::*,
};

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
                        max_texture_dimension_2d: 4096, // Allow higher resolutions on native
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
            format: format,
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

        // GLOBAL UNIFORMS - CAMERA ETC, NEED FOR PIPELINES

        let global_ubo = GlobalUBO::new(&device);
        let mut global_bindings = GlobalBindings::new(&device);
        global_bindings.create_bind_group(&device, &global_ubo);

        // Create transform bind group layout
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

        let device_handle: Arc<Device> = device.into();

        let mut pipeline_manager = PipelineManager::new(device_handle.clone());

        let _ = pipeline_manager.load_shader("default", include_str!("shader.wgsl"));

        pipeline_manager.register_pipeline(
            "PBR",
            PipelineConfig::default()
                .with_shader("default")
                .with_depth_stencil(depth_texture.texture.clone())
                .with_bind_group_layouts(vec![
                    global_bindings.bind_group_layouts().clone(),
                    transform_bind_group_layout, // Add transform layout
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

    pub fn render_frame(&mut self, scene: &Scene) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: None,
                });
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
                // depth_stencil_attachment: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    //attach depth texture to stencil attatchement of render pass
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

            //global bindings
            render_pass.set_bind_group(0, self.global_bindings.bind_groups(), &[]);

            // render_pass.set_pipeline(&self.pipeline);

            // for object in scene.objects.iter() {
            //     render_pass.draw_object(object);
            // }

            // self.pipeline_manager.list_pipelines_to_terminal();

            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                // Debug: Check each object's transform state
                println!("=== RENDER DEBUG ===");
                println!("Rendering {} objects", scene.objects.len());

                // Render all objects in the scene
                for (i, object) in scene.objects.iter().enumerate() {
                    println!(
                        "Object {}: GPU resources: {}",
                        i,
                        object.gpu_resources.is_some()
                    );

                    if let Some(_gpu_resources) = &object.gpu_resources {
                        // Print the transform matrix (translation components)
                        let transform_data: &[f32; 16] = object.transform.as_ref();
                        println!(
                            "  Transform matrix translation: [{:.2}, {:.2}, {:.2}]",
                            transform_data[12], transform_data[13], transform_data[14]
                        );
                        println!(
                            "  Transform matrix scale: [{:.2}, {:.2}, {:.2}]",
                            transform_data[0], transform_data[5], transform_data[10]
                        );
                    } else {
                        println!("  ❌ No GPU resources - transform won't be applied!");
                    }

                    render_pass.draw_object(object);
                    println!("  ✓ Object {} rendered", i);
                }
                println!("=== END RENDER DEBUG ===");
            } else {
                println!("❌ PBR pipeline not found!");
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        surface_texture.present();
    }

    // Method to render with UI callback
    pub fn render_frame_with_ui<F>(&mut self, scene: &Scene, ui_callback: F)
    where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder, &wgpu::TextureView),
    {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: None,
                });

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

            // Global bindings
            render_pass.set_bind_group(0, self.global_bindings.bind_groups(), &[]);

            // Render scene with your existing pipeline
            if let Some(pipeline) = self.pipeline_manager.get_pipeline("PBR") {
                render_pass.set_pipeline(pipeline);

                // Render all objects in the scene
                for object in scene.objects.iter() {
                    if object.visible {
                        render_pass.draw_object(object);
                    }
                }
            }
        }

        // Call UI callback to render UI on top
        ui_callback(
            &self.device,
            &self.queue,
            &mut encoder,
            &surface_texture_view,
        );

        // Submit commands and present
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    pub fn update(&mut self, camera_uniform: CameraUniform) {
        update_global_ubo(&mut self.global_ubo, &self.queue, camera_uniform);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);

        self.depth_texture =
            TextureResource::create_depth_texture(&self.device, &self.config, "depth_texture");
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    // Method to expose the surface format for UI manager creation
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.format
    }
}
