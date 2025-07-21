//! Bridge between old visualization system and new rendering architecture
//!
//! Converts visualization components to VisualizationPlanes for the new rendering system

use crate::gfx::rendering::VisualizationPlane;
use crate::visualization::cut_plane_2d::CutPlane2D;

/// Trait for converting visualization components to render-ready planes
pub trait ToVisualizationPlane {
    /// Convert this component to a VisualizationPlane for rendering
    fn to_visualization_plane(&self) -> Option<VisualizationPlane>;
}

impl ToVisualizationPlane for CutPlane2D {
    fn to_visualization_plane(&self) -> Option<VisualizationPlane> {
        use cgmath::Vector3;
        
        // Get the material from the cut plane if it exists
        if let Some(material) = self.get_material() {
            let size_scalar = self.get_size();
            let plane = VisualizationPlane {
                position: self.get_position(),
                size: Vector3::new(size_scalar, size_scalar, 1.0), // Convert scalar to vector
                material: material.clone(), // Clone the material to own it
                data_buffer: None, // For now, data is handled through texture
                texture: None,     // Texture is handled in material
            };
            
            Some(plane)
        } else {
            None
        }
    }
}

/// Helper to collect all visualization planes from a manager
pub fn collect_visualization_planes<T: ToVisualizationPlane>(
    components: &[T]
) -> Vec<VisualizationPlane> {
    components
        .iter()
        .filter_map(|component| component.to_visualization_plane())
        .collect()
}