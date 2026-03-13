use godot::prelude::*;

fn test(cam: &mut Gd<godot::classes::Camera3D>) {
    cam.look_at(Vector3::ZERO);
}
