//! # Materials Showcase Example
//!
//! This example demonstrates the Haggis material system with various PBR materials,
//! presets, and lighting setups. Shows the difference between metallic and
//! non-metallic materials, roughness values, and how lighting affects appearance.
//!
//! ## What this example shows:
//! - Material presets (gold, silver, plastic, ceramic, etc.)
//! - Custom material creation with PBR parameters
//! - Different lighting setups
//! - Real-time material parameter adjustment
//! - Material inheritance and variations
//!
//! ## Usage:
//! ```bash
//! cargo run --example materials_showcase
//! ```

use haggis::prelude::*;

/// Material showcase simulation that rotates objects for better viewing
struct MaterialShowcase {
    rotation_speed: f32,
    time: f32,
    auto_rotate: bool,
    light_intensity: f32,
    light_position: Vector3<f32>,
}

impl MaterialShowcase {
    fn new() -> Self {
        Self {
            rotation_speed: 30.0, // degrees per second
            time: 0.0,
            auto_rotate: true,
            light_intensity: 2.0,
            light_position: Vector3::new(5.0, 5.0, 8.0),
        }
    }

    fn update_rotations(&self, scene: &mut Scene) {
        if !self.auto_rotate {
            return;
        }

        // Rotate each object at slightly different speeds for variety
        for (i, object) in scene.objects.iter_mut().enumerate() {
            let speed_multiplier = 1.0 + (i as f32) * 0.2;
            object.ui_transform.rotation[1] = self.time * self.rotation_speed * speed_multiplier;
            object.ui_transform.rotation[2] = self.time * self.rotation_speed * speed_multiplier * 0.5;
            object.apply_ui_transform();
        }
    }
}

impl Simulation for MaterialShowcase {
    fn initialize(&mut self, _scene: &mut Scene) {
        // No initialization needed for this showcase
    }

    fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        self.time += delta_time;
        self.update_rotations(scene);
    }

    fn render_ui(&mut self, ui: &Ui) {
        ui.window("Material Controls")
            .size([320.0, 400.0], imgui::Condition::FirstUseEver)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🎨 Material Showcase");
                ui.separator();

                ui.text("Animation:");
                ui.checkbox("Auto Rotate Objects", &mut self.auto_rotate);
                if self.auto_rotate {
                    ui.slider("Rotation Speed", 0.0, 100.0, &mut self.rotation_speed);
                }
                ui.spacing();

                ui.text("Lighting:");
                ui.slider("Light Intensity", 0.1, 5.0, &mut self.light_intensity);

                let mut light_pos = [self.light_position.x, self.light_position.y, self.light_position.z];
                if ui.input_float3("Light Position", &mut light_pos).build() {
                    self.light_position = Vector3::new(light_pos[0], light_pos[1], light_pos[2]);
                }
                ui.spacing();

                ui.text("Materials on Display:");
                ui.bullet_text("🥇 Gold (high metallic, low roughness)");
                ui.bullet_text("🥈 Silver (high metallic, very low roughness)");
                ui.bullet_text("🔴 Red Plastic (low metallic, medium roughness)");
                ui.bullet_text("🟢 Green Ceramic (low metallic, low roughness)");
                ui.bullet_text("🔵 Blue Metal (high metallic, medium roughness)");
                ui.bullet_text("⚫ Rubber (low metallic, high roughness)");
                ui.bullet_text("💎 Crystal (low metallic, very low roughness)");
                ui.bullet_text("🪨 Stone (low metallic, high roughness)");
            });

        ui.window("Material Science")
            .size([300.0, 300.0], imgui::Condition::FirstUseEver)
            .position([340.0, 10.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("🔬 PBR Material Theory");
                ui.separator();

                ui.text("Metallic Factor:");
                ui.bullet_text("0.0 = Dielectric (plastic, wood, skin)");
                ui.bullet_text("1.0 = Metallic (gold, silver, iron)");
                ui.bullet_text("Controls base reflectance");
                ui.spacing();

                ui.text("Roughness Factor:");
                ui.bullet_text("0.0 = Mirror-like surface");
                ui.bullet_text("0.5 = Moderate surface roughness");
                ui.bullet_text("1.0 = Very rough, matte surface");
                ui.bullet_text("Controls light scattering");
                ui.spacing();

                ui.text("Base Color:");
                ui.bullet_text("Diffuse color for dielectrics");
                ui.bullet_text("Reflectance color for metals");
                ui.bullet_text("Should be dark for metals");
            });

        // Material editor placeholder
        ui.window("Material Editor")
            .size([280.0, 200.0], imgui::Condition::FirstUseEver)
            .position([10.0, 420.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("Material Editor");
                ui.separator();
                ui.text("Select an object to edit its material");
                ui.text("(Material editing interface)");
                ui.spacing();

                if ui.button("Apply Gold Preset") {
                    // Apply gold material preset
                }
                ui.same_line();
                if ui.button("Apply Plastic Preset") {
                    // Apply plastic material preset
                }
            });
    }

    fn name(&self) -> &str {
        "Materials Showcase"
    }

    fn is_running(&self) -> bool {
        true
    }

    fn set_running(&mut self, _running: bool) {
        // Always running
    }

    fn reset(&mut self, _scene: &mut Scene) {
        // Reset to initial state
        self.rotation_speed = 30.0;
        self.time = 0.0;
        self.auto_rotate = true;
        self.light_intensity = 2.0;
        self.light_position = Vector3::new(5.0, 5.0, 8.0);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn main() -> HaggisResult<()> {
    println!("🎨 Haggis Materials Showcase");
    println!("===========================");
    println!("Demonstrating PBR materials, presets, and lighting effects.");
    println!();

    // Create application with material showcase
    let mut app = haggis::default();

    // Add multiple objects with different materials
    app.add_object("examples/test/sphere.obj")
        .with_transform([-6.0, 2.0, 2.0], 0.8, 0.0)
        .with_material("gold")
        .with_name("gold_sphere");

    app.add_object("examples/test/cube.obj")
        .with_transform([-2.0, 2.0, 2.0], 0.7, 0.0)
        .with_material("silver")
        .with_name("silver_cube");

    app.add_object("examples/test/sphere.obj")
        .with_transform([2.0, 2.0, 2.0], 0.8, 0.0)
        .with_material("copper")
        .with_name("copper_sphere");

    app.add_object("examples/test/cube.obj")
        .with_transform([6.0, 2.0, 2.0], 0.7, 0.0)
        .with_material("blue_metal")
        .with_name("blue_metal_cube");

    // Middle row: Dielectrics
    app.add_object("examples/test/cube.obj")
        .with_transform([-6.0, 0.0, 2.0], 0.7, 0.0)
        .with_material("red_plastic")
        .with_name("red_plastic_cube");

    app.add_object("examples/test/sphere.obj")
        .with_transform([-2.0, 0.0, 2.0], 0.8, 0.0)
        .with_material("green_ceramic")
        .with_name("green_ceramic_sphere");

    app.add_object("examples/test/cube.obj")
        .with_transform([2.0, 0.0, 2.0], 0.7, 0.0)
        .with_material("blue_ceramic")
        .with_name("blue_ceramic_cube");

    app.add_object("examples/test/sphere.obj")
        .with_transform([6.0, 0.0, 2.0], 0.8, 0.0)
        .with_material("white_plastic")
        .with_name("white_plastic_sphere");

    // Bottom row: Special materials
    app.add_object("examples/test/sphere.obj")
        .with_transform([-6.0, -2.0, 2.0], 0.8, 0.0)
        .with_material("rubber")
        .with_name("rubber_sphere");

    app.add_object("examples/test/cube.obj")
        .with_transform([-2.0, -2.0, 2.0], 0.7, 0.0)
        .with_material("glass")
        .with_name("glass_cube");

    app.add_object("examples/test/sphere.obj")
        .with_transform([2.0, -2.0, 2.0], 0.8, 0.0)
        .with_material("stone")
        .with_name("stone_sphere");

    app.add_object("examples/test/cube.obj")
        .with_transform([6.0, -2.0, 2.0], 0.7, 0.0)
        .with_material("wood")
        .with_name("wood_cube");

    // Add a ground plane
    app.add_object("examples/test/plane.obj")
        .with_transform([0.0, 0.0, 0.0], 15.0, 0.0)
        .with_material("ground")
        .with_name("ground_plane");

    app.attach_simulation(MaterialShowcase::new());

    app.set_ui(|ui, scene, selected| {
        default_transform_panel(ui, scene, selected);
    });

    app.run();
    Ok(())
}