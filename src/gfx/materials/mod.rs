//! # Smart Material System
//!
//! Ergonomic material creation and management with presets, inheritance,
//! and automatic texture loading. Provides both simple presets and advanced
//! material building capabilities.

use crate::{
    error::{HaggisResult, HaggisError},
    gfx::resources::material::Material,
};
use cgmath::Vector3;
use std::collections::HashMap;

/// Material builder with fluent API and validation
pub struct MaterialBuilder {
    name: String,
    base_color: Vector3<f32>,
    metallic: f32,
    roughness: f32,
    emissive: Vector3<f32>,
    albedo_texture: Option<String>,
    normal_texture: Option<String>,
    metallic_roughness_texture: Option<String>,
    emissive_texture: Option<String>,
    parent: Option<String>,
}

impl MaterialBuilder {
    /// Create a new material builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_color: Vector3::new(0.8, 0.8, 0.8),
            metallic: 0.0,
            roughness: 0.5,
            emissive: Vector3::new(0.0, 0.0, 0.0),
            albedo_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            emissive_texture: None,
            parent: None,
        }
    }

    /// Set the base color (albedo)
    pub fn base_color(mut self, color: Vector3<f32>) -> Self {
        self.base_color = color;
        self
    }

    /// Set base color from RGB values
    pub fn base_color_rgb(mut self, r: f32, g: f32, b: f32) -> Self {
        self.base_color = Vector3::new(r, g, b);
        self
    }

    /// Set base color from hex string (e.g., "#FF0000")
    pub fn base_color_hex(mut self, hex: &str) -> HaggisResult<Self> {
        let color = parse_hex_color(hex)?;
        self.base_color = color;
        Ok(self)
    }

    /// Set metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub fn metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Set roughness factor (0.0 = mirror, 1.0 = completely rough)
    pub fn roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set emissive color for glowing materials
    pub fn emissive(mut self, color: Vector3<f32>) -> Self {
        self.emissive = color;
        self
    }

    /// Set emissive from RGB values
    pub fn emissive_rgb(mut self, r: f32, g: f32, b: f32) -> Self {
        self.emissive = Vector3::new(r, g, b);
        self
    }

    /// Set albedo texture path
    pub fn albedo_texture(mut self, path: impl Into<String>) -> Self {
        self.albedo_texture = Some(path.into());
        self
    }

    /// Set normal map texture path
    pub fn normal_texture(mut self, path: impl Into<String>) -> Self {
        self.normal_texture = Some(path.into());
        self
    }

    /// Set metallic/roughness texture path
    pub fn metallic_roughness_texture(mut self, path: impl Into<String>) -> Self {
        self.metallic_roughness_texture = Some(path.into());
        self
    }

    /// Set emissive texture path
    pub fn emissive_texture(mut self, path: impl Into<String>) -> Self {
        self.emissive_texture = Some(path.into());
        self
    }

    /// Inherit properties from another material
    pub fn inherit_from(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Make this a shiny metal material
    pub fn shiny_metal(mut self) -> Self {
        self.metallic = 1.0;
        self.roughness = 0.1;
        self
    }

    /// Make this a rough metal material
    pub fn rough_metal(mut self) -> Self {
        self.metallic = 1.0;
        self.roughness = 0.8;
        self
    }

    /// Make this a plastic material
    pub fn plastic(mut self) -> Self {
        self.metallic = 0.0;
        self.roughness = 0.3;
        self
    }

    /// Make this a glass material
    pub fn glass(mut self) -> Self {
        self.metallic = 0.0;
        self.roughness = 0.0;
        self
    }

    /// Make this a rubber material
    pub fn rubber(mut self) -> Self {
        self.metallic = 0.0;
        self.roughness = 0.9;
        self
    }

    /// Make this an emissive/glowing material
    pub fn glowing(mut self, intensity: f32) -> Self {
        let color = self.base_color * intensity;
        self.emissive = color;
        self
    }

    /// Validate the material configuration
    pub fn validate(&self) -> HaggisResult<()> {
        if self.name.is_empty() {
            return Err(HaggisError::validation("Material name cannot be empty"));
        }

        if self.metallic < 0.0 || self.metallic > 1.0 {
            return Err(HaggisError::validation_field(
                "Metallic factor must be between 0.0 and 1.0",
                "metallic",
                "0.0-1.0",
                &self.metallic.to_string(),
            ));
        }

        if self.roughness < 0.0 || self.roughness > 1.0 {
            return Err(HaggisError::validation_field(
                "Roughness factor must be between 0.0 and 1.0",
                "roughness",
                "0.0-1.0",
                &self.roughness.to_string(),
            ));
        }

        // Validate color components are reasonable
        let max_color_component = self.base_color.x.max(self.base_color.y).max(self.base_color.z);
        if max_color_component < 0.0 || max_color_component > 2.0 {
            return Err(HaggisError::validation(
                "Base color components should be between 0.0 and 2.0"
            ).with_suggestion("Use sRGB values or reasonable HDR values"));
        }

        Ok(())
    }

    /// Build the material (would integrate with existing Material type)
    pub fn build(self) -> HaggisResult<MaterialDefinition> {
        self.validate()?;

        Ok(MaterialDefinition {
            name: self.name,
            base_color: self.base_color,
            metallic: self.metallic,
            roughness: self.roughness,
            emissive: self.emissive,
            albedo_texture: self.albedo_texture,
            normal_texture: self.normal_texture,
            metallic_roughness_texture: self.metallic_roughness_texture,
            emissive_texture: self.emissive_texture,
            parent: self.parent,
        })
    }
}

/// Material definition that can be converted to the engine's Material type
#[derive(Debug, Clone)]
pub struct MaterialDefinition {
    pub name: String,
    pub base_color: Vector3<f32>,
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: Vector3<f32>,
    pub albedo_texture: Option<String>,
    pub normal_texture: Option<String>,
    pub metallic_roughness_texture: Option<String>,
    pub emissive_texture: Option<String>,
    pub parent: Option<String>,
}

/// Material registry for managing materials and inheritance
pub struct MaterialRegistry {
    materials: HashMap<String, MaterialDefinition>,
    presets: HashMap<String, MaterialDefinition>,
}

impl Default for MaterialRegistry {
    fn default() -> Self {
        let mut registry = Self {
            materials: HashMap::new(),
            presets: HashMap::new(),
        };

        // Add common material presets
        registry.add_presets();
        registry
    }
}

impl MaterialRegistry {
    /// Create a new material registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a material
    pub fn register(&mut self, material: MaterialDefinition) -> HaggisResult<()> {
        if self.materials.contains_key(&material.name) {
            return Err(HaggisError::validation(
                format!("Material '{}' already exists", material.name)
            ).with_suggestion("Use a different name or update the existing material"));
        }

        self.materials.insert(material.name.clone(), material);
        Ok(())
    }

    /// Get a material by name
    pub fn get(&self, name: &str) -> Option<&MaterialDefinition> {
        self.materials.get(name).or_else(|| self.presets.get(name))
    }

    /// Get a material with inheritance resolved
    pub fn get_resolved(&self, name: &str) -> HaggisResult<MaterialDefinition> {
        let material = self.get(name)
            .ok_or_else(|| HaggisError::resource(
                format!("Material '{}' not found", name),
                "material"
            ))?;

        // Resolve inheritance
        if let Some(parent_name) = &material.parent {
            let parent = self.get_resolved(parent_name)?;
            Ok(self.merge_materials(&parent, material))
        } else {
            Ok(material.clone())
        }
    }

    /// List all available materials
    pub fn list_materials(&self) -> Vec<&str> {
        self.materials.keys()
            .chain(self.presets.keys())
            .map(|s| s.as_str())
            .collect()
    }

    /// Create a builder for a new material
    pub fn builder(&self, name: impl Into<String>) -> MaterialBuilder {
        MaterialBuilder::new(name)
    }

    /// Quick creation methods for common materials
    pub fn metal(&self, name: impl Into<String>, color: Vector3<f32>) -> MaterialBuilder {
        MaterialBuilder::new(name)
            .base_color(color)
            .shiny_metal()
    }

    pub fn plastic(&self, name: impl Into<String>, color: Vector3<f32>) -> MaterialBuilder {
        MaterialBuilder::new(name)
            .base_color(color)
            .plastic()
    }

    pub fn glass(&self, name: impl Into<String>, color: Vector3<f32>) -> MaterialBuilder {
        MaterialBuilder::new(name)
            .base_color(color)
            .glass()
    }

    /// Add preset materials
    fn add_presets(&mut self) {
        let presets = vec![
            ("gold", Vector3::new(1.0, 0.8, 0.0), 1.0, 0.2),
            ("silver", Vector3::new(0.9, 0.9, 0.9), 1.0, 0.1),
            ("copper", Vector3::new(0.9, 0.4, 0.2), 1.0, 0.3),
            ("iron", Vector3::new(0.5, 0.5, 0.5), 1.0, 0.5),
            ("chrome", Vector3::new(0.8, 0.8, 0.8), 1.0, 0.05),
            ("red_plastic", Vector3::new(0.8, 0.2, 0.2), 0.0, 0.3),
            ("blue_plastic", Vector3::new(0.2, 0.2, 0.8), 0.0, 0.3),
            ("white_plastic", Vector3::new(0.9, 0.9, 0.9), 0.0, 0.3),
            ("black_rubber", Vector3::new(0.1, 0.1, 0.1), 0.0, 0.9),
            ("clear_glass", Vector3::new(0.95, 0.95, 0.95), 0.0, 0.0),
            ("wood", Vector3::new(0.6, 0.4, 0.2), 0.0, 0.7),
            ("concrete", Vector3::new(0.6, 0.6, 0.6), 0.0, 0.8),
        ];

        for (name, color, metallic, roughness) in presets {
            let material = MaterialDefinition {
                name: name.to_string(),
                base_color: color,
                metallic,
                roughness,
                emissive: Vector3::new(0.0, 0.0, 0.0),
                albedo_texture: None,
                normal_texture: None,
                metallic_roughness_texture: None,
                emissive_texture: None,
                parent: None,
            };
            self.presets.insert(name.to_string(), material);
        }
    }

    /// Merge parent material with child, with child properties taking precedence
    fn merge_materials(&self, parent: &MaterialDefinition, child: &MaterialDefinition) -> MaterialDefinition {
        MaterialDefinition {
            name: child.name.clone(),
            base_color: child.base_color,
            metallic: child.metallic,
            roughness: child.roughness,
            emissive: child.emissive,
            albedo_texture: child.albedo_texture.clone().or_else(|| parent.albedo_texture.clone()),
            normal_texture: child.normal_texture.clone().or_else(|| parent.normal_texture.clone()),
            metallic_roughness_texture: child.metallic_roughness_texture.clone()
                .or_else(|| parent.metallic_roughness_texture.clone()),
            emissive_texture: child.emissive_texture.clone().or_else(|| parent.emissive_texture.clone()),
            parent: None, // Don't chain inheritance
        }
    }
}

/// Parse hex color string to Vector3<f32>
fn parse_hex_color(hex: &str) -> HaggisResult<Vector3<f32>> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Err(HaggisError::validation(
            "Hex color must be 6 characters long (e.g., '#FF0000')"
        ));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| HaggisError::validation("Invalid hex color format"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| HaggisError::validation("Invalid hex color format"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| HaggisError::validation("Invalid hex color format"))?;

    Ok(Vector3::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
    ))
}

/// Convenient functions for common material operations
impl MaterialRegistry {
    /// Create a material from a preset and register it
    pub fn from_preset(&mut self, name: impl Into<String>, preset: &str) -> HaggisResult<MaterialBuilder> {
        let preset_material = self.presets.get(preset)
            .ok_or_else(|| HaggisError::resource(
                format!("Preset '{}' not found", preset),
                "material_preset"
            ))?;

        Ok(MaterialBuilder::new(name)
            .base_color(preset_material.base_color)
            .metallic(preset_material.metallic)
            .roughness(preset_material.roughness)
            .emissive(preset_material.emissive))
    }

    /// Quick material creation with automatic registration
    pub fn quick_metal(&mut self, name: impl Into<String>, hex_color: &str) -> HaggisResult<()> {
        let material = self.builder(name)
            .base_color_hex(hex_color)?
            .shiny_metal()
            .build()?;
        self.register(material)
    }

    pub fn quick_plastic(&mut self, name: impl Into<String>, hex_color: &str) -> HaggisResult<()> {
        let material = self.builder(name)
            .base_color_hex(hex_color)?
            .plastic()
            .build()?;
        self.register(material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_builder() {
        let material = MaterialBuilder::new("test_metal")
            .base_color_rgb(0.8, 0.6, 0.2)
            .shiny_metal()
            .build()
            .unwrap();

        assert_eq!(material.name, "test_metal");
        assert_eq!(material.metallic, 1.0);
        assert_eq!(material.roughness, 0.1);
    }

    #[test]
    fn test_hex_color_parsing() {
        let color = parse_hex_color("#FF0000").unwrap();
        assert_eq!(color, Vector3::new(1.0, 0.0, 0.0));

        let color = parse_hex_color("00FF00").unwrap();
        assert_eq!(color, Vector3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_material_registry() {
        let registry = MaterialRegistry::new();
        assert!(registry.get("gold").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_material_inheritance() {
        let mut registry = MaterialRegistry::new();

        // Create a parent material
        let parent = MaterialBuilder::new("parent")
            .base_color_rgb(1.0, 0.0, 0.0)
            .metallic(0.5)
            .albedo_texture("parent_texture.png")
            .build()
            .unwrap();
        registry.register(parent).unwrap();

        // Create a child material that inherits from parent
        let child = MaterialBuilder::new("child")
            .inherit_from("parent")
            .roughness(0.8) // Override just roughness
            .build()
            .unwrap();
        registry.register(child).unwrap();

        // Get resolved child material
        let resolved = registry.get_resolved("child").unwrap();
        assert_eq!(resolved.base_color, Vector3::new(1.0, 0.0, 0.0)); // From parent
        assert_eq!(resolved.metallic, 0.5); // From parent
        assert_eq!(resolved.roughness, 0.8); // From child
        assert_eq!(resolved.albedo_texture, Some("parent_texture.png".to_string())); // From parent
    }

    #[test]
    fn test_validation() {
        let result = MaterialBuilder::new("test")
            .metallic(2.0) // Invalid value
            .build();

        assert!(result.is_err());
    }
}