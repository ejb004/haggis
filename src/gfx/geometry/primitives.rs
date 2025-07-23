//! # Primitive Shape Generation
//!
//! This module contains functions to generate common 3D primitive shapes.
//! All shapes are generated with proper normals and texture coordinates.

use super::GeometryData;
use std::f32::consts::PI;

/// Generate a unit cube centered at the origin
/// 
/// Returns a cube with vertices from -0.5 to 0.5 on all axes.
/// Each face has proper normals pointing outward and UV coordinates from 0 to 1.
pub fn generate_cube() -> GeometryData {
    let mut data = GeometryData::new();
    
    // Cube vertices (8 corners)
    let positions = [
        // Front face
        [-0.5, -0.5,  0.5], [ 0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [-0.5,  0.5,  0.5],
        // Back face  
        [-0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [ 0.5,  0.5, -0.5], [ 0.5, -0.5, -0.5],
        // Left face
        [-0.5, -0.5, -0.5], [-0.5, -0.5,  0.5], [-0.5,  0.5,  0.5], [-0.5,  0.5, -0.5],
        // Right face
        [ 0.5, -0.5,  0.5], [ 0.5, -0.5, -0.5], [ 0.5,  0.5, -0.5], [ 0.5,  0.5,  0.5],
        // Top face
        [-0.5,  0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5],
        // Bottom face
        [-0.5, -0.5, -0.5], [ 0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [-0.5, -0.5,  0.5],
    ];
    
    // Texture coordinates (same for each face)
    let tex_coords = [
        // Front, Back, Left, Right, Top, Bottom faces
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0],
        [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0],
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0],
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
    ];
    
    // Face normals
    let normals = [
        // Front face (positive Z)
        [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0],
        // Back face (negative Z)
        [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0],
        // Left face (negative X)
        [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
        // Right face (positive X)
        [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0],
        // Top face (positive Y in Z-up becomes positive Z in Y-up for rendering)
        [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
        // Bottom face (negative Y in Z-up becomes negative Z in Y-up for rendering)
        [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
    ];
    
    data.vertices = positions.to_vec();
    data.tex_coords = tex_coords.to_vec();
    data.normals = normals.to_vec();
    
    // Indices for each face (2 triangles per face, counter-clockwise)
    data.indices = vec![
        // Front face
        0, 1, 2,    2, 3, 0,
        // Back face
        4, 5, 6,    6, 7, 4,
        // Left face
        8, 9, 10,   10, 11, 8,
        // Right face
        12, 13, 14, 14, 15, 12,
        // Top face
        16, 17, 18, 18, 19, 16,
        // Bottom face
        20, 21, 22, 22, 23, 20,
    ];
    
    data
}

/// Generate a UV sphere with specified resolution
/// 
/// # Arguments
/// * `longitude_segments` - Number of vertical segments (longitude lines)
/// * `latitude_segments` - Number of horizontal segments (latitude lines)
/// 
/// Returns a sphere of radius 1.0 centered at the origin.
pub fn generate_sphere(longitude_segments: u32, latitude_segments: u32) -> GeometryData {
    let mut data = GeometryData::new();
    
    let long_segs = longitude_segments.max(3);
    let lat_segs = latitude_segments.max(2);
    
    // Generate vertices
    for lat in 0..=lat_segs {
        let theta = lat as f32 * PI / lat_segs as f32; // 0 to PI
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        
        for long in 0..=long_segs {
            let phi = long as f32 * 2.0 * PI / long_segs as f32; // 0 to 2*PI
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            
            // Spherical to Cartesian coordinates
            let x = sin_theta * cos_phi;
            let y = cos_theta;  // Y-up for rendering
            let z = sin_theta * sin_phi;
            
            data.vertices.push([x, y, z]);
            data.normals.push([x, y, z]); // Normal is same as position for unit sphere
            
            // UV coordinates
            let u = long as f32 / long_segs as f32;
            let v = lat as f32 / lat_segs as f32;
            data.tex_coords.push([u, v]);
        }
    }
    
    // Generate indices
    for lat in 0..lat_segs {
        for long in 0..long_segs {
            let first = lat * (long_segs + 1) + long;
            let second = first + long_segs + 1;
            
            // First triangle
            data.indices.push(first);
            data.indices.push(second);
            data.indices.push(first + 1);
            
            // Second triangle
            data.indices.push(second);
            data.indices.push(second + 1);
            data.indices.push(first + 1);
        }
    }
    
    data
}

/// Generate a plane in the XY plane (horizontal in Z-up coordinate system)
/// 
/// # Arguments
/// * `width` - Width of the plane (X direction)
/// * `height` - Height of the plane (Y direction) 
/// * `width_segments` - Number of subdivisions along width
/// * `height_segments` - Number of subdivisions along height
/// 
/// Returns a plane centered at the origin with normal pointing up (positive Z).
pub fn generate_plane(width: f32, height: f32, width_segments: u32, height_segments: u32) -> GeometryData {
    let mut data = GeometryData::new();
    
    let w_segs = width_segments.max(1);
    let h_segs = height_segments.max(1);
    
    let _half_width = width * 0.5;
    let _half_height = height * 0.5;
    
    // Generate vertices
    for y in 0..=h_segs {
        let v = y as f32 / h_segs as f32;
        let pos_y = (v - 0.5) * height;
        
        for x in 0..=w_segs {
            let u = x as f32 / w_segs as f32;
            let pos_x = (u - 0.5) * width;
            
            // Z-up coordinate system: plane lies in XY plane
            data.vertices.push([pos_x, pos_y, 0.0]);
            data.normals.push([0.0, 0.0, 1.0]); // Normal points up (positive Z)
            data.tex_coords.push([u, v]);
        }
    }
    
    // Generate indices (counter-clockwise winding when viewed from above)
    for y in 0..h_segs {
        for x in 0..w_segs {
            let i = y * (w_segs + 1) + x;
            let next_row = i + w_segs + 1;
            
            // First triangle (counter-clockwise)
            data.indices.push(i);
            data.indices.push(next_row);
            data.indices.push(i + 1);
            
            // Second triangle (counter-clockwise)
            data.indices.push(next_row);
            data.indices.push(next_row + 1);
            data.indices.push(i + 1);
        }
    }
    
    data
}

/// Generate a cylinder with specified parameters
/// 
/// # Arguments
/// * `radius` - Radius of the cylinder
/// * `height` - Height of the cylinder (along Z-axis)
/// * `segments` - Number of circular segments
/// 
/// Returns a cylinder centered at the origin extending from -height/2 to height/2 in Z.
pub fn generate_cylinder(radius: f32, height: f32, segments: u32) -> GeometryData {
    let mut data = GeometryData::new();
    
    let segs = segments.max(3);
    let half_height = height * 0.5;
    
    // Generate side vertices
    for i in 0..=segs {
        let angle = i as f32 * 2.0 * PI / segs as f32;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let x = radius * cos_a;
        let y = radius * sin_a;
        
        // Bottom vertex
        data.vertices.push([x, y, -half_height]);
        data.normals.push([cos_a, sin_a, 0.0]);
        data.tex_coords.push([i as f32 / segs as f32, 0.0]);
        
        // Top vertex
        data.vertices.push([x, y, half_height]);
        data.normals.push([cos_a, sin_a, 0.0]);
        data.tex_coords.push([i as f32 / segs as f32, 1.0]);
    }
    
    // Side faces
    for i in 0..segs {
        let bottom_current = i * 2;
        let top_current = bottom_current + 1;
        let bottom_next = ((i + 1) % (segs + 1)) * 2;
        let top_next = bottom_next + 1;
        
        // First triangle
        data.indices.push(bottom_current);
        data.indices.push(top_current);
        data.indices.push(bottom_next);
        
        // Second triangle
        data.indices.push(top_current);
        data.indices.push(top_next);
        data.indices.push(bottom_next);
    }
    
    // Add center vertices for caps
    let center_bottom_idx = data.vertices.len() as u32;
    data.vertices.push([0.0, 0.0, -half_height]);
    data.normals.push([0.0, 0.0, -1.0]);
    data.tex_coords.push([0.5, 0.5]);
    
    let center_top_idx = data.vertices.len() as u32;
    data.vertices.push([0.0, 0.0, half_height]);
    data.normals.push([0.0, 0.0, 1.0]);
    data.tex_coords.push([0.5, 0.5]);
    
    // Bottom cap
    for i in 0..segs {
        let current = i * 2;
        let next = ((i + 1) % (segs + 1)) * 2;
        
        data.indices.push(center_bottom_idx);
        data.indices.push(next);
        data.indices.push(current);
    }
    
    // Top cap
    for i in 0..segs {
        let current = i * 2 + 1;
        let next = ((i + 1) % (segs + 1)) * 2 + 1;
        
        data.indices.push(center_top_idx);
        data.indices.push(current);
        data.indices.push(next);
    }
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cube_generation() {
        let cube = generate_cube();
        assert_eq!(cube.vertices.len(), 24); // 6 faces * 4 vertices
        assert_eq!(cube.indices.len(), 36); // 6 faces * 2 triangles * 3 indices
        assert_eq!(cube.vertex_count(), 24);
        assert_eq!(cube.triangle_count(), 12);
    }
    
    #[test]
    fn test_sphere_generation() {
        let sphere = generate_sphere(8, 6);
        assert!(sphere.vertices.len() > 0);
        assert!(sphere.indices.len() > 0);
        assert_eq!(sphere.vertices.len(), sphere.normals.len());
        assert_eq!(sphere.vertices.len(), sphere.tex_coords.len());
    }
    
    #[test]
    fn test_plane_generation() {
        let plane = generate_plane(2.0, 2.0, 2, 2);
        assert_eq!(plane.vertices.len(), 9); // 3x3 grid
        assert_eq!(plane.indices.len(), 24); // 4 quads * 2 triangles * 3 indices
    }
}