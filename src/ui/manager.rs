// src/ui/manager.rs
//! ImGui UI manager for Haggis engine
//!
//! Handles ImGui integration with wgpu and winit, providing frame management,
//! input handling, and rendering capabilities for the engine's user interface.

use imgui::{Context, FontConfig, FontSource, MouseCursor};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use winit::{
    event::{Event, WindowEvent},
    window::Window,
};

/// ImGui UI manager
///
/// Manages ImGui context, platform integration, and rendering pipeline.
/// Handles input capture, frame timing, and coordinate scaling for proper
/// UI display across different DPI settings.
pub struct UiManager {
    pub context: Context,
    platform: WinitPlatform,
    renderer: Renderer,
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
}

impl UiManager {
    /// Creates a new UI manager
    ///
    /// Sets up ImGui with proper DPI handling and font configuration.
    /// Uses locked DPI mode to prevent automatic scaling conflicts.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating renderer resources
    /// * `queue` - WGPU queue for renderer operations
    /// * `output_color_format` - Target texture format for rendering
    /// * `window` - Window for platform integration
    pub fn new(
        device: &Device,
        queue: &Queue,
        output_color_format: TextureFormat,
        window: &Window,
    ) -> Self {
        let mut context = Context::create();
        context.set_ini_filename(None);

        // Setup platform with locked DPI to handle scaling manually
        let mut platform = WinitPlatform::new(&mut context);
        platform.attach_window(context.io_mut(), window, HiDpiMode::Locked(1.0));

        // Configure fonts with consistent sizing
        let font_size = 24.0;
        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        // Uncomment to use custom font:
        // context.fonts().add_font(&[FontSource::TtfData {
        //     data: include_bytes!("./fonts/roboto.ttf"),
        //     size_pixels: font_size,
        //     config: Some(FontConfig {
        //         oversample_h: 1,
        //         pixel_snap_h: true,
        //         size_pixels: font_size,
        //         ..Default::default()
        //     }),
        // }]);

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

    /// Updates ImGui's display size to match render target
    ///
    /// Must be called when the render target size changes to ensure
    /// proper UI scaling and positioning.
    ///
    /// # Arguments
    /// * `width` - New display width in pixels
    /// * `height` - New display height in pixels
    pub fn update_display_size(&mut self, width: u32, height: u32) {
        self.context.io_mut().display_size = [width as f32, height as f32];
    }

    /// Returns current ImGui display size for debugging
    pub fn get_display_size(&self) -> [f32; 2] {
        self.context.io().display_size
    }

    /// Handles input events and returns whether UI captured them
    ///
    /// Processes mouse and keyboard events through ImGui's input system.
    /// Returns true if the UI wants to capture the input (preventing
    /// it from reaching other systems like camera controls).
    ///
    /// # Arguments
    /// * `window` - Window reference for platform integration
    /// * `event` - Input event to process
    ///
    /// # Returns
    /// True if UI captured the input, false otherwise
    pub fn handle_input<T>(&mut self, window: &Window, event: &Event<T>) -> bool {
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

    /// Updates UI logic and returns whether UI wants input capture
    ///
    /// Prepares a new ImGui frame, runs the provided UI callback,
    /// and handles cursor changes. This should be called once per frame
    /// before rendering.
    ///
    /// # Arguments
    /// * `window` - Window reference for platform operations
    /// * `run_ui` - Callback function that builds the UI
    ///
    /// # Returns
    /// True if UI wants to capture input this frame
    pub fn update_logic<F>(&mut self, window: &Window, run_ui: F) -> bool
    where
        F: FnOnce(&imgui::Ui),
    {
        // Update timing
        let now = Instant::now();
        self.context
            .io_mut()
            .update_delta_time(now - self.last_frame);
        self.last_frame = now;

        // Prepare frame
        self.platform
            .prepare_frame(self.context.io_mut(), window)
            .expect("Failed to prepare frame");

        // Build UI
        let ui = self.context.frame();
        run_ui(&ui);

        // Handle cursor changes
        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(&ui, window);
        }

        let io = self.context.io();
        io.want_capture_mouse || io.want_capture_keyboard
    }

    /// Renders the UI overlay to the specified render target
    ///
    /// Renders the UI built in the last `update_logic()` call to the
    /// provided color attachment. Uses LoadOp::Load to preserve the
    /// existing 3D scene content.
    ///
    /// # Arguments
    /// * `device` - WGPU device for render operations
    /// * `queue` - WGPU queue for command submission
    /// * `encoder` - Command encoder to record render commands
    /// * `window` - Window reference (unused but kept for API consistency)
    /// * `color_attachment` - Target texture view for rendering
    pub fn render_display_only(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        _window: &Window,
        color_attachment: &TextureView,
    ) {
        let draw_data = self.context.render();

        // Validate draw data to prevent render errors
        if draw_data.display_size[0] <= 0.0 || draw_data.display_size[1] <= 0.0 {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("imgui_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_attachment,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Preserve 3D scene
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

    /// Convenience method that combines update and render
    ///
    /// Equivalent to calling `update_logic()` followed by `render_display_only()`.
    /// Useful for simple single-pass UI rendering.
    ///
    /// # Arguments
    /// * `device` - WGPU device for render operations
    /// * `queue` - WGPU queue for command submission
    /// * `encoder` - Command encoder to record render commands
    /// * `window` - Window reference for platform integration
    /// * `color_attachment` - Target texture view for rendering
    /// * `run_ui` - Callback function that builds the UI
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
        self.update_logic(window, run_ui);
        self.render_display_only(device, queue, encoder, window, color_attachment);
    }
}
