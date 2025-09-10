//! # Visualization Builders
//!
//! Consistent builder patterns for visualization components

use crate::builder::{Builder, CommonConfig, ConfigurableBuilder};
use crate::visualization::cut_plane_2d::{CutPlane2D, DataSource, BufferFormat, BufferElementType};
use crate::visualization::ui::cut_plane_controls::{FilterMode, VisualizationMode};
use std::sync::Arc;
use wgpu::Buffer;

/// Builder for 2D cut plane visualization
pub struct CutPlane2DBuilder {
    pub(crate) common: CommonConfig,
    pub(crate) data_source: Option<DataSource>,
    pub(crate) mode: VisualizationMode,
    pub(crate) filter: FilterMode,
    pub(crate) position: f32,
    pub(crate) axis: u8, // 0=X, 1=Y, 2=Z
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) min_value: f32,
    pub(crate) max_value: f32,
}

impl CutPlane2DBuilder {
    /// Create new cut plane builder
    pub fn new() -> Self {
        Self {
            common: CommonConfig::default(),
            data_source: None,
            mode: VisualizationMode::Heatmap,
            filter: FilterMode::Sharp,
            position: 0.0,
            axis: 2, // Z-axis by default
            width: 256,
            height: 256,
            min_value: 0.0,
            max_value: 1.0,
        }
    }

    /// Set the data source from CPU data
    pub fn with_cpu_data(mut self, data: Vec<f32>, width: u32, height: u32) -> Self {
        self.data_source = Some(DataSource::CpuData(data));
        self.width = width;
        self.height = height;
        self
    }

    /// Set the data source from GPU buffer
    pub fn with_gpu_buffer(
        mut self,
        buffer: Arc<Buffer>,
        element_type: BufferElementType,
        width: u32,
        height: u32,
    ) -> Self {
        self.data_source = Some(DataSource::GpuBuffer {
            buffer,
            format: BufferFormat {
                element_type,
                width,
                height,
            },
        });
        self.width = width;
        self.height = height;
        self
    }

    /// Set visualization mode
    pub fn with_mode(mut self, mode: VisualizationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set filter mode
    pub fn with_filter(mut self, filter: FilterMode) -> Self {
        self.filter = filter;
        self
    }

    /// Set the axis for slicing (0=X, 1=Y, 2=Z)
    pub fn with_axis(mut self, axis: u8) -> Self {
        self.axis = axis.min(2); // Clamp to valid range
        self
    }

    /// Set the slice position along the axis
    pub fn with_position(mut self, position: f32) -> Self {
        self.position = position;
        self
    }

    /// Set the value range for color mapping
    pub fn with_value_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }

    /// Set grid dimensions
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl Builder<CutPlane2D> for CutPlane2DBuilder {
    fn build(self) -> CutPlane2D {
        let data_source = self.data_source.unwrap_or_else(|| {
            // Default empty data
            DataSource::CpuData(vec![0.0; (self.width * self.height) as usize])
        });

        CutPlane2D::new_with_config(
            data_source,
            self.mode,
            self.filter,
            self.position,
            self.axis,
            self.min_value,
            self.max_value,
            self.common,
        )
    }
}

impl ConfigurableBuilder<CutPlane2D> for CutPlane2DBuilder {
    fn merge(mut self, other: Self) -> Self {
        // Other takes precedence for most fields
        if other.data_source.is_some() {
            self.data_source = other.data_source;
        }
        self.mode = other.mode;
        self.filter = other.filter;
        if other.position != 0.0 {
            self.position = other.position;
        }
        if other.axis != 2 {
            self.axis = other.axis;
        }
        if other.width != 256 {
            self.width = other.width;
        }
        if other.height != 256 {
            self.height = other.height;
        }
        self
    }

    fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("Width and height must be greater than 0".to_string());
        }
        if self.axis > 2 {
            return Err("Axis must be 0 (X), 1 (Y), or 2 (Z)".to_string());
        }
        if self.min_value >= self.max_value {
            return Err("Min value must be less than max value".to_string());
        }
        Ok(())
    }
}

// Implement common builder methods using macro
crate::impl_common_builder_methods!(CutPlane2DBuilder);