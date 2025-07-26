//! Test file to verify ViewportGizmo compiles and works

#[cfg(test)]
mod tests {
    use super::super::viewport_gizmo::{ViewportGizmo, ViewDirection};
    use crate::gfx::gizmos::Gizmo;
    use cgmath::InnerSpace;

    #[test]
    fn test_viewport_gizmo_creation() {
        let gizmo = ViewportGizmo::new();
        assert!(gizmo.is_enabled());
        assert_eq!(gizmo.name(), "Viewport Gizmo");
    }

    #[test]
    fn test_view_directions() {
        let views = [
            ViewDirection::Front,
            ViewDirection::Back,
            ViewDirection::Right,
            ViewDirection::Left,
            ViewDirection::Top,
            ViewDirection::Bottom,
        ];

        for view in &views {
            let pos = view.get_camera_position(5.0);
            let up = view.get_up_vector();
            let color = view.get_face_color();
            let label = view.get_label();

            // Basic sanity checks
            assert!(pos.magnitude() > 0.0);
            assert!(up.magnitude() > 0.0);
            assert!(color[0] >= 0.0 && color[0] <= 1.0);
            assert!(color[1] >= 0.0 && color[1] <= 1.0);
            assert!(color[2] >= 0.0 && color[2] <= 1.0);
            assert!(!label.is_empty());
        }
    }
}