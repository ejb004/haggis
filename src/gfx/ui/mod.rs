// Add to Cargo.toml:
/*
[dependencies]
imgui = "0.12"
imgui-wgpu = "0.24"
imgui-winit-support = "0.12"
*/

use imgui::{Context, FontConfig, FontSource, MouseCursor, Ui};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use winit::{
    event::{Event, WindowEvent},
    window::Window,
};

pub struct UiManager {
    pub context: Context,
    platform: WinitPlatform,
    renderer: Renderer,
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
}

impl UiManager {
    pub fn new(
        device: &Device,
        queue: &Queue,
        output_color_format: TextureFormat,
        window: &Window,
    ) -> Self {
        // Create ImGui context
        let mut context = Context::create();
        context.set_ini_filename(None);

        // Setup platform integration
        let mut platform = WinitPlatform::new(&mut context);
        platform.attach_window(context.io_mut(), window, HiDpiMode::Default);

        // Setup fonts
        let hidpi_factor = window.scale_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        // Create renderer
        let renderer_config = RendererConfig {
            texture_format: output_color_format,
            ..Default::default()
        };
        let renderer = Renderer::new(&mut context, device, queue, renderer_config);

        Self {
            context,
            platform,
            renderer,
            last_frame: Instant::now(),
            last_cursor: None,
        }
    }

    pub fn handle_input<T>(&mut self, window: &Window, event: &Event<T>) -> bool {
        // Only handle certain event types to avoid conflicts
        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::CursorMoved { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::KeyboardInput { .. }
                | WindowEvent::Focused(_) => {
                    self.platform
                        .handle_event(self.context.io_mut(), window, event);

                    let io = self.context.io();
                    io.want_capture_mouse || io.want_capture_keyboard
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn draw<F>(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        color_attachment: &TextureView,
        run_ui: F,
    ) where
        F: FnOnce(&Ui),
    {
        // Update delta time
        let now = Instant::now();
        self.context
            .io_mut()
            .update_delta_time(now - self.last_frame);
        self.last_frame = now;

        // Prepare frame
        self.platform
            .prepare_frame(self.context.io_mut(), window)
            .expect("Failed to prepare frame");

        // Create UI frame
        let ui = self.context.frame();

        // Run user UI code
        run_ui(&ui);

        // Handle cursor changes
        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(&ui, window);
        }

        // Get draw data from context
        let draw_data = self.context.render();

        // Render
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("imgui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_attachment,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer
                .render(draw_data, queue, device, &mut render_pass)
                .expect("Failed to render ImGui");
        }
    }
}
