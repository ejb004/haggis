//! Texture resource management for wgpu
//!
//! Provides utilities for creating and managing GPU textures, views, and samplers
//! with specialized support for depth buffers and render targets.

/// GPU texture resource containing texture, view, and sampler
///
/// Bundles the three main components needed for texture operations:
/// - Texture: The actual GPU memory allocation
/// - View: Interface for shader access
/// - Sampler: Filtering and addressing configuration
pub struct TextureResource {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl TextureResource {
    /// Standard depth buffer format used throughout the engine
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// Creates a depth texture matching the surface configuration
    ///
    /// Creates a depth buffer with the same dimensions as the render surface,
    /// configured for depth testing and optional texture sampling.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating resources
    /// * `config` - Surface configuration to match dimensions
    /// * `label` - Debug label for the texture
    ///
    /// # Returns
    /// TextureResource configured for depth testing
    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[Self::DEPTH_FORMAT],
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Configure sampler for depth comparison operations
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}
