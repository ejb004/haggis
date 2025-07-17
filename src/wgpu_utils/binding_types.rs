// src/wgpu_utils/binding_types.rs - Enhanced with compute types
//! WGPU binding type utilities

pub fn buffer(read_only: bool) -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only },
        has_dynamic_offset: false,
        min_binding_size: None,
    }
}

pub fn storage_buffer_read_write() -> wgpu::BindingType {
    buffer(false)
}

pub fn storage_buffer_read_only() -> wgpu::BindingType {
    buffer(true)
}

pub fn uniform() -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    }
}

pub fn sampler(filtering: wgpu::SamplerBindingType) -> wgpu::BindingType {
    wgpu::BindingType::Sampler(filtering)
}

pub fn texture_2d() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    }
}

pub fn texture_2d_array() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D2Array,
        multisampled: false,
    }
}

pub fn itexture_2d() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Sint,
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    }
}

pub fn utexture_2d() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Uint,
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    }
}

pub fn texture_3d() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D3,
        multisampled: false,
    }
}

pub fn itexture_3d() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Sint,
        view_dimension: wgpu::TextureViewDimension::D3,
        multisampled: false,
    }
}

pub fn utexture_3d() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Uint,
        view_dimension: wgpu::TextureViewDimension::D3,
        multisampled: false,
    }
}

pub fn texture_cube() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::Cube,
        multisampled: false,
    }
}

pub fn image_2d(
    format: wgpu::TextureFormat,
    access: wgpu::StorageTextureAccess,
) -> wgpu::BindingType {
    wgpu::BindingType::StorageTexture {
        access,
        view_dimension: wgpu::TextureViewDimension::D2,
        format,
    }
}

pub fn image_2d_array(
    format: wgpu::TextureFormat,
    access: wgpu::StorageTextureAccess,
) -> wgpu::BindingType {
    wgpu::BindingType::StorageTexture {
        access,
        view_dimension: wgpu::TextureViewDimension::D2Array,
        format,
    }
}

pub fn image_3d(
    format: wgpu::TextureFormat,
    access: wgpu::StorageTextureAccess,
) -> wgpu::BindingType {
    wgpu::BindingType::StorageTexture {
        access,
        view_dimension: wgpu::TextureViewDimension::D3,
        format,
    }
}

// Additional compute-specific helpers
pub fn compute_storage_read_write() -> wgpu::BindingType {
    storage_buffer_read_write()
}

pub fn compute_storage_read_only() -> wgpu::BindingType {
    storage_buffer_read_only()
}

pub fn compute_uniform() -> wgpu::BindingType {
    uniform()
}
