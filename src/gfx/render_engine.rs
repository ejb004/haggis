use std::{iter, sync::Arc};

use wgpu::{Buffer, DepthStencilState, FrontFace, RenderPipeline, TextureFormat, TextureUsages};

use super::{
    camera::camera_utils::CameraUniform,
    global_bindings::{update_global_ubo, GlobalBindings, GlobalUBO},
    object::DrawObject,
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

    pipeline: RenderPipeline,

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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[global_bindings.bind_group_layouts()],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex3D::desc()],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // cull_mode: Some(wgpu::Face::Back),
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            // depth_stencil: None,
            depth_stencil: Some(DepthStencilState {
                format: depth_texture.texture.format(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        RenderEngine {
            device: device.into(),
            config,
            format,
            surface,
            queue: queue.into(),
            pipeline,
            depth_texture,

            global_bindings,
            global_ubo,
        }
    }

    pub fn render_frame(&self, scene: &Scene) {
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

            render_pass.set_pipeline(&self.pipeline);
            // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            // render_pass.draw_indexed(0..18, 0, 0..1);

            for object in scene.objects.iter() {
                render_pass.draw_object(object);
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
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
}
