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
#[derive(Clone)]
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

    pub fn create_shadow_map(device: &wgpu::Device, size: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // IMPORTANT: Shadow maps need a comparison sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // This is crucial for shadow mapping!
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler, // Don't forget this!
        }
    }

    /// Creates a 2D texture from raw RGBA data with configurable filtering
    ///
    /// Creates a texture that can be used for data visualization, such as heatmaps
    /// or other procedurally generated content.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating resources
    /// * `queue` - WGPU queue for uploading data
    /// * `data` - Raw RGBA8 pixel data (4 bytes per pixel)
    /// * `width` - Width of the texture in pixels
    /// * `height` - Height of the texture in pixels
    /// * `label` - Debug label for the texture
    /// * `filter_mode` - Texture filtering mode (Nearest for sharp, Linear for smooth)
    ///
    /// # Returns
    /// TextureResource with the uploaded data
    pub fn create_from_rgba_data_with_filter(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
        label: &str,
        filter_mode: wgpu::FilterMode,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload the data to the texture
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{} Sampler", label)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    /// Creates a 2D texture from raw RGBA data with default linear filtering
    ///
    /// Convenience method that uses Linear filtering for backwards compatibility.
    /// For sharp, pixelated rendering, use `create_from_rgba_data_with_filter` with `FilterMode::Nearest`.
    ///
    /// # Arguments
    /// * `device` - WGPU device for creating resources
    /// * `queue` - WGPU queue for uploading data
    /// * `data` - Raw RGBA8 pixel data (4 bytes per pixel)
    /// * `width` - Width of the texture in pixels
    /// * `height` - Height of the texture in pixels
    /// * `label` - Debug label for the texture
    ///
    /// # Returns
    /// TextureResource with the uploaded data
    pub fn create_from_rgba_data(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        Self::create_from_rgba_data_with_filter(
            device,
            queue,
            data,
            width,
            height,
            label,
            wgpu::FilterMode::Linear, // Default to smooth for backwards compatibility
        )
    }
}
