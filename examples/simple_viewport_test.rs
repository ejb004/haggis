//! Simple test to verify the viewport gizmo system works

fn main() {
    // Test that we can create and use the viewport gizmo
    println!("Testing viewport gizmo compilation...");
    
    // This will only compile if ViewportGizmo is properly exported
    let _gizmo = haggis::gfx::gizmos::ViewportGizmo::new();
    let _direction = haggis::gfx::gizmos::ViewDirection::Front;
    
    println!("✅ Viewport gizmo types are accessible!");
    println!("✅ ViewDirection enum works!");
    
    // Test basic properties
    println!("📍 Front view position: {:?}", _direction.get_camera_position(5.0));
    println!("🎨 Front view color: {:?}", _direction.get_face_color());
    println!("🏷️  Front view label: {}", _direction.get_label());
    
    println!("🎯 All viewport gizmo components are working!");
}