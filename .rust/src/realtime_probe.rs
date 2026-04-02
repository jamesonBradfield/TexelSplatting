use godot::classes::{
    rendering_server::TextureLayeredType, Camera3D, INode3D, Image, Node3D, RenderingServer,
    ShaderMaterial, SubViewport,
};
use godot::prelude::*;

const FACE_RESOLUTION: i32 = 512;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct RealtimeProbe {
    base: Base<Node3D>,

    #[export]
    cameras: Array<Gd<Camera3D>>,

    #[export]
    follow_node: Option<Gd<Node3D>>,

    #[export]
    fake_world_node: Option<Gd<Node3D>>,

    #[export]
    material: Option<Gd<ShaderMaterial>>,

    #[export(range = (1.0, 1000.0, 0.01))]
    tick_rate_ms: f64,

    #[export(range = (1.0, 32.0, 1.0))]
    cull_mask: u32,

    time_accumulator: f64,
    faces: Vec<Gd<Image>>,
    cubemap_rid: Rid,
}

#[godot_api]
impl INode3D for RealtimeProbe {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            cameras: Array::new(),
            follow_node: None,
            fake_world_node: None,
            material: None,
            time_accumulator: 0.0,
            tick_rate_ms: 33.33, // Default to ~30fps capture
            cull_mask: 0xFFFFFFFF,
            faces: Vec::with_capacity(6),
            cubemap_rid: Rid::Invalid,
        }
    }

    fn ready(&mut self) {
        self._spawn_cameras();
    }

    fn process(&mut self, delta: f64) {
        // 1. Sync probe to follow target immediately
        if let Some(target) = self.follow_node.clone() {
            let target_pos = target.get_global_position();
            self.base_mut().set_global_position(target_pos);
        }

        self.time_accumulator += delta * 1000.0;
        if self.time_accumulator >= self.tick_rate_ms {
            self.time_accumulator = 0.0;
            self.trigger_capture();
        }
    }

    fn exit_tree(&mut self) {
        if !self.cubemap_rid.is_invalid() {
            RenderingServer::singleton().free_rid(self.cubemap_rid);
            self.cubemap_rid = Rid::Invalid;
        }
    }
}

#[godot_api]
impl RealtimeProbe {
    #[signal]
    fn probe_updated(images: Array<Gd<Image>>, cubemap_rid: Rid);

    #[func]
    fn trigger_capture(&mut self) {
        if self.cameras.len() != 6 {
            return;
        }

        let origin = self.base().get_global_position();

        // 1. Temporarily hide the fake world so it's not in the capture
        if let Some(mut fw) = self.fake_world_node.clone() {
            fw.set_visible(false);
        }

        for i in 0..6 {
            let camera = self.cameras.at(i);
            let mut cam_mut = camera.clone();
            cam_mut.set_global_position(origin);

            if let Some(mut vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                // Trigger exactly one render this frame
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);
            }
        }

        // 2. Wait for the render to complete before reading (next frame)
        self.base_mut()
            .call_deferred("_deferred_read_and_update", &[]);
    }

    /// Reads captured images from sub-views, reconstructs the cubemap, updates the material,
    // AI! Can we comment the function's content?    /// and emits the probe_updated signal.
    #[func]
    fn _deferred_read_and_update(&mut self) {
        let mut current_capture: Vec<Gd<Image>> = Vec::with_capacity(6);

        for i in 0..6 {
            let camera = self.cameras.at(i);
            if let Some(vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                if let Some(texture) = vp.get_texture() {
                    if let Some(image) = texture.get_image() {
                        let mut img: Gd<Image> =
                            image.duplicate().expect("Failed to duplicate").cast();
                        if i != 3 {
                            img.flip_x();
                        } else {
                            img.flip_y();
                        }
                        current_capture.push(img);
                    }
                }
            }
        }

        // Show the fake world again
        if let Some(mut fw) = self.fake_world_node.clone() {
            fw.set_visible(true);
            // Ensure it's perfectly synced after capture
            fw.set_global_position(self.base().get_global_position());
        }

        if current_capture.len() == 6 {
            self.faces = current_capture;
            let mut rs = RenderingServer::singleton();

            if self.cubemap_rid.is_invalid() {
                let mut image_array = Array::<Gd<Image>>::new();
                for img in &self.faces {
                    image_array.push(img);
                }
                self.cubemap_rid =
                    rs.texture_2d_layered_create(&image_array, TextureLayeredType::CUBEMAP);
            } else {
                for (i, img) in self.faces.iter().enumerate() {
                    rs.texture_2d_update(self.cubemap_rid, img, i as i32);
                }
            }

            let cubemap_rid = self.cubemap_rid;
            let faces_array = self.get_faces_array();
            self.signals()
                .probe_updated()
                .emit(&faces_array, cubemap_rid);

            // Directly update the material parameter
            if let Some(mat) = self.material.clone() {
                RenderingServer::singleton().material_set_param(
                    mat.get_rid(),
                    "env_cubemap",
                    &self.cubemap_rid.to_variant(),
                );
            }
        }
    }

    #[func]
    fn _spawn_cameras(&mut self) {
        self.cameras.clear();
        let children = self.base().get_children();
        for i in 0..children.len() {
            let child = children.at(i);
            if child.get_name().to_string().contains("FaceViewport_") {
                child.clone().free();
            }
        }

        let face_rotations = [
            Vector3::new(0.0, -90.0, 0.0), // +X
            Vector3::new(0.0, 90.0, 0.0),  // -X
            Vector3::new(90.0, 0.0, 0.0),  // +Y
            Vector3::new(-90.0, 0.0, 0.0), // -Y
            Vector3::new(0.0, 180.0, 0.0), // +Z
            Vector3::new(0.0, 0.0, 0.0),   // -Z
        ];

        let world = self.base().get_world_3d();

        for (i, &rotation) in face_rotations.iter().enumerate() {
            let mut vp_gd = SubViewport::new_alloc();
            vp_gd.set_name(&format!("FaceViewport_{}", i));
            vp_gd.set_size(Vector2i::new(FACE_RESOLUTION, FACE_RESOLUTION));
            // Start disabled, only render when triggered
            vp_gd.set_update_mode(godot::classes::sub_viewport::UpdateMode::DISABLED);
            vp_gd.set_clear_mode(godot::classes::sub_viewport::ClearMode::ALWAYS);

            if let Some(w) = &world {
                vp_gd.set_world_3d(w);
            }

            let mut cam_gd = Camera3D::new_alloc();
            cam_gd.set_name(&format!("FaceCamera_{}", i));
            cam_gd.set_fov(90.0);
            cam_gd.set_rotation_degrees(rotation);
            cam_gd.set_cull_mask(self.cull_mask);

            vp_gd.add_child(&cam_gd);
            self.base_mut().add_child(&vp_gd);
            self.cameras.push(&cam_gd);
        }
    }

    #[func]
    pub fn get_faces_array(&self) -> Array<Gd<Image>> {
        self.faces.iter().cloned().collect()
    }
}
