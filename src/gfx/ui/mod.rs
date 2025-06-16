// Add to Cargo.toml:
/*
[dependencies]
imgui = "0.12"
imgui-wgpu = "0.24"
imgui-winit-support = "0.12"
*/

pub mod panel;

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

        // CRITICAL: Don't set display_size here - it will be set correctly later
        println!(
            "Creating UiManager - initial display_size will be set to [0.0, 0.0] as placeholder"
        );

        // Setup platform integration with FIXED DPI handling
        let mut platform = WinitPlatform::new(&mut context);
        // CRITICAL: Use Locked mode to prevent automatic DPI scaling
        // We'll handle the scaling manually by setting display_size to physical pixels
        platform.attach_window(context.io_mut(), window, HiDpiMode::Locked(1.0));

        // FIXED: Simplified font setup to avoid DPI scaling issues
        let hidpi_factor = window.scale_factor();
        println!("Window scale factor: {}", hidpi_factor);

        // Use a reasonable base font size that works across different DPI settings
        let font_size = 24.0; // Fixed size, let the platform handle DPI scaling

        // IMPORTANT: Don't set font_global_scale manually - let imgui-winit-support handle it
        // The HiDpiMode::Default should handle this automatically

        // Setup fonts with consistent sizing
        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        // Option 2: Load a custom font from file (uncomment to use)
        // context.fonts().add_font(&[FontSource::TtfData {
        //     data: include_bytes!("./fonts/roboto.ttf"), // Path to your font
        //     size_pixels: font_size,
        //     config: Some(FontConfig {
        //         oversample_h: 1,
        //         pixel_snap_h: true,
        //         size_pixels: font_size,
        //         ..Default::default()
        //     }),
        // }]);

        // Create renderer
        let renderer_config = RendererConfig {
            texture_format: output_color_format,
            ..Default::default()
        };
        let renderer = Renderer::new(&mut context, device, queue, renderer_config);

        println!(
            "UiManager created - display_size: {:?}",
            context.io().display_size
        );

        Self {
            context,
            platform,
            renderer,
            last_frame: Instant::now(),
            last_cursor: None,
        }
    }

    /// CRITICAL: Update ImGui's display size to match render target
    pub fn update_display_size(&mut self, width: u32, height: u32) {
        let old_size = self.context.io().display_size;
        self.context.io_mut().display_size = [width as f32, height as f32];
        println!(
            "UiManager: Updated ImGui display size from [{:.0}, {:.0}] to [{}x{}]",
            old_size[0], old_size[1], width, height
        );

        // Debug: Verify the change took effect
        let new_size = self.context.io().display_size;
        if new_size[0] != width as f32 || new_size[1] != height as f32 {
            println!(
                "WARNING: Display size update failed! Expected [{}x{}], got [{:.0}, {:.0}]",
                width, height, new_size[0], new_size[1]
            );
        }
    }

    /// Get current ImGui display size for debugging
    pub fn get_display_size(&self) -> [f32; 2] {
        self.context.io().display_size
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

    pub fn update_logic<F>(&mut self, window: &Window, run_ui: F) -> bool
    where
        F: FnOnce(&imgui::Ui),
    {
        // Update delta time
        let now = Instant::now();
        self.context
            .io_mut()
            .update_delta_time(now - self.last_frame);
        self.last_frame = now;

        // Debug display size before frame preparation
        let display_size = self.context.io().display_size;
        if display_size[0] == 0.0 || display_size[1] == 0.0 {
            println!(
                "WARNING: ImGui display_size is zero: [{}, {}]",
                display_size[0], display_size[1]
            );
        }

        // Prepare frame
        self.platform
            .prepare_frame(self.context.io_mut(), window)
            .expect("Failed to prepare frame");

        // Debug display size after frame preparation (platform might modify it)
        let display_size_after = self.context.io().display_size;
        if display_size != display_size_after {
            println!(
                "Display size changed during prepare_frame: [{:.0}, {:.0}] -> [{:.0}, {:.0}]",
                display_size[0], display_size[1], display_size_after[0], display_size_after[1]
            );
        }

        // Create UI frame and run logic
        let ui = self.context.frame();
        run_ui(&ui);

        // Handle cursor changes
        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(&ui, window);
        }

        // Don't render yet - just return if UI wants input capture
        let io = self.context.io();
        io.want_capture_mouse || io.want_capture_keyboard
    }

    pub fn render_display_only(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        color_attachment: &TextureView,
    ) {
        // Get the draw data from the last frame
        let draw_data = self.context.render();

        // Debug: Check draw data validity
        if draw_data.display_size[0] <= 0.0 || draw_data.display_size[1] <= 0.0 {
            println!(
                "ERROR: Invalid draw_data display_size: [{}, {}]",
                draw_data.display_size[0], draw_data.display_size[1]
            );
            return; // Skip rendering to avoid scissor rect error
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("imgui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_attachment,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load existing 3D scene
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

    pub fn draw<F>(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        color_attachment: &TextureView,
        run_ui: F,
    ) where
        F: FnOnce(&imgui::Ui),
    {
        // This combines update_logic + render_display_only
        self.update_logic(window, run_ui);
        self.render_display_only(device, queue, encoder, window, color_attachment);
    }
}
