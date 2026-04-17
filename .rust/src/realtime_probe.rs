//! # RealtimeProbe
//!
//! A real-time cubemap capture system that uses 12 cameras arranged in a cube
//! configuration to capture environment color and depth maps for use in materials.

use godot::classes::{
    rendering_server::TextureLayeredType, Camera3D, INode3D, Image, Node3D, RenderingServer,
    ShaderMaterial, SubViewport, MeshInstance3D, QuadMesh, Mesh, Material, Shader
};
use godot::prelude::*;

/// A real-time cubemap capture node that uses 12 cameras to synthesize color and depth environment maps.
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct RealtimeProbe {
    /// Internal base node reference.
    base: Base<Node3D>,

    /// Array of 6 Camera3D nodes for color.
    #[export]
    color_cameras: Array<Gd<Camera3D>>,

    /// Array of 6 Camera3D nodes for depth.
    #[export]
    depth_cameras: Array<Gd<Camera3D>>,

    /// Optional target node to follow.
    #[export]
    follow_node: Option<Gd<Node3D>>,

    /// Optional shader material to automatically update with the captured cubemap.
    #[export]
    material: Option<Gd<ShaderMaterial>>,

    /// Time between capture cycles in milliseconds.
    tick_rate_ms: f64,

    /// Time accumulator for frame-rate independent capture timing.
    time_accumulator: f64,

    /// Bitmask controlling which objects are visible in captures.
    cull_mask: u32,

    /// Array of 6 captured color images.
    color_faces: Vec<Gd<Image>>,

    /// Array of 6 captured depth images.
    depth_faces: Vec<Gd<Image>>,

    /// RID of the synthesized color cubemap texture.
    color_cubemap_rid: Rid,

    /// RID of the synthesized depth cubemap texture.
    depth_cubemap_rid: Rid,
}

#[godot_api]
impl INode3D for RealtimeProbe {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            color_cameras: Array::new(),
            depth_cameras: Array::new(),
            follow_node: None,
            material: None,
            tick_rate_ms: 33.33,
            time_accumulator: 0.0,
            cull_mask: 0xFFFFFFFF,
            color_faces: Vec::with_capacity(6),
            depth_faces: Vec::with_capacity(6),
            color_cubemap_rid: Rid::Invalid,
            depth_cubemap_rid: Rid::Invalid,
        }
    }

    fn ready(&mut self) {
        self._apply_settings_from_render_manager();
        self._spawn_cameras();
    }

    fn process(&mut self, delta: f64) {
        // Sync probe to follow target
        if let Some(target) = self.follow_node.clone() {
            let target_pos = target.get_global_position();
            self.base_mut().set_global_position(target_pos);
        }

        // Accumulate time for frame-rate independent capture timing
        self.time_accumulator += delta * 1000.0;
        if self.time_accumulator >= self.tick_rate_ms {
            self.time_accumulator = 0.0;
            self.trigger_capture();
        }
    }

    fn exit_tree(&mut self) {
        if !self.color_cubemap_rid.is_invalid() {
            RenderingServer::singleton().free_rid(self.color_cubemap_rid);
            self.color_cubemap_rid = Rid::Invalid;
        }
        if !self.depth_cubemap_rid.is_invalid() {
            RenderingServer::singleton().free_rid(self.depth_cubemap_rid);
            self.depth_cubemap_rid = Rid::Invalid;
        }
    }
}

#[godot_api]
impl RealtimeProbe {
    /// Signal emitted when a new cubemap has been captured and synthesized.
    #[signal]
    fn probe_updated(color_cubemap_rid: Rid, depth_cubemap_rid: Rid);

    /// Applies settings from RenderManager (SSOT) to this probe instance.
    #[func]
    fn _apply_settings_from_render_manager(&mut self) {
        if let Some(render_mgr) = self.base().get_node_or_null("/root/RenderManager") {
            let settings_variant = render_mgr.get("settings");
            if !settings_variant.is_nil() {
                if let Ok(settings) = settings_variant.try_to::<Gd<Object>>() {
                    // Read tick rate from SSOT
                    let tick_rate_variant = settings.get("probe_tick_rate_ms");
                    if !tick_rate_variant.is_nil() {
                        if let Ok(tick_rate) = tick_rate_variant.try_to::<f64>() {
                            self.tick_rate_ms = tick_rate;
                        }
                    }

                    // Read cull mask from SSOT
                    let cull_mask_variant = settings.get("probe_cull_mask");
                    if !cull_mask_variant.is_nil() {
                        if let Ok(cull_mask) = cull_mask_variant.try_to::<u32>() {
                            self.cull_mask = cull_mask;
                            // Update existing cameras
                            for i in 0..self.color_cameras.len() {
                                let mut cam = self.color_cameras.at(i);
                                cam.set_cull_mask(cull_mask);
                            }
                            for i in 0..self.depth_cameras.len() {
                                let mut cam = self.depth_cameras.at(i);
                                cam.set_cull_mask(cull_mask);
                            }
                        }
                    }

                    // Read face resolution from SSOT and update viewports
                    let face_res_variant = settings.get("face_resolution");
                    if !face_res_variant.is_nil() {
                        if let Ok(face_res) = face_res_variant.try_to::<i32>() {
                            let size = Vector2i::new(face_res, face_res);
                            for i in 0..self.color_cameras.len() {
                                let cam = self.color_cameras.at(i);
                                if let Some(mut vp) = cam
                                    .get_parent()
                                    .and_then(|p| p.try_cast::<SubViewport>().ok())
                                {
                                    if vp.get_size() != size {
                                        vp.set_size(size);
                                    }
                                }
                            }
                            for i in 0..self.depth_cameras.len() {
                                let cam = self.depth_cameras.at(i);
                                if let Some(mut vp) = cam
                                    .get_parent()
                                    .and_then(|p| p.try_cast::<SubViewport>().ok())
                                {
                                    if vp.get_size() != size {
                                        vp.set_size(size);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Triggers a capture cycle by enabling sub-viewports for one frame.
    #[func]
    fn trigger_capture(&mut self) {
        for i in 0..self.color_cameras.len() {
            let cam = self.color_cameras.at(i);
            if let Some(mut vp) = cam
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);
            }
        }
        for i in 0..self.depth_cameras.len() {
            let cam = self.depth_cameras.at(i);
            if let Some(mut vp) = cam
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);
            }
        }
        // Defer image reading to next frame to allow viewports to render
        self.base_mut()
            .call_deferred("_deferred_read_and_update", &[]);
    }

    #[func]
    fn _deferred_read_and_update(&mut self) {
        let mut current_color_capture: Vec<Gd<Image>> = Vec::with_capacity(6);
        let mut current_depth_capture: Vec<Gd<Image>> = Vec::with_capacity(6);

        for i in 0..6 {
            if i >= self.color_cameras.len() {
                break;
            }
            let camera = self.color_cameras.at(i);
            if let Some(vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                if let Some(texture) = vp.get_texture() {
                    if let Some(image) = texture.get_image() {
                        let mut img: Gd<Image> =
                            image.duplicate().expect("Failed to duplicate").cast();
                        // Flip images to correct cubemap orientation
                        if i != 3 {
                            img.flip_x();
                        } else {
                            img.flip_y();
                        }
                        current_color_capture.push(img);
                    }
                }
            }
        }

        for i in 0..6 {
            if i >= self.depth_cameras.len() {
                break;
            }
            let camera = self.depth_cameras.at(i);
            if let Some(vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                if let Some(texture) = vp.get_texture() {
                    if let Some(image) = texture.get_image() {
                        let mut img: Gd<Image> =
                            image.duplicate().expect("Failed to duplicate").cast();
                        // Flip images to correct cubemap orientation
                        if i != 3 {
                            img.flip_x();
                        } else {
                            img.flip_y();
                        }
                        current_depth_capture.push(img);
                    }
                }
            }
        }

        if current_color_capture.len() != 6 || current_depth_capture.len() != 6 {
            return;
        }

        self.color_faces = current_color_capture;
        self.depth_faces = current_depth_capture;

        let mut rs = RenderingServer::singleton();

        if self.color_cubemap_rid.is_invalid() {
            let mut image_array = Array::<Gd<Image>>::new();
            for img in &self.color_faces {
                image_array.push(img);
            }
            // Add a fallback if texture creation fails
            match rs.texture_2d_layered_create(&image_array, TextureLayeredType::CUBEMAP) {
                rid => {
                    self.color_cubemap_rid = rid;
                }
            }
        } else {
            for (i, img) in self.color_faces.iter().enumerate() {
                rs.texture_2d_update(self.color_cubemap_rid, img, i as i32);
            }
        }

        if self.depth_cubemap_rid.is_invalid() {
            let mut image_array = Array::<Gd<Image>>::new();
            for img in &self.depth_faces {
                image_array.push(img);
            }
            // Add a fallback if texture creation fails
            match rs.texture_2d_layered_create(&image_array, TextureLayeredType::CUBEMAP) {
                rid => {
                    self.depth_cubemap_rid = rid;
                }
            }
        } else {
            for (i, img) in self.depth_faces.iter().enumerate() {
                rs.texture_2d_update(self.depth_cubemap_rid, img, i as i32);
            }
        }

        let color_rid = self.color_cubemap_rid;
        let depth_rid = self.depth_cubemap_rid;
        self.signals()
            .probe_updated()
            .emit(color_rid, depth_rid);

        if let Some(mat) = self.material.clone() {
            RenderingServer::singleton().material_set_param(
                mat.get_rid(),
                "env_cubemap",
                &self.color_cubemap_rid.to_variant(),
            );
            RenderingServer::singleton().material_set_param(
                mat.get_rid(),
                "depth_cubemap",
                &self.depth_cubemap_rid.to_variant(),
            );
        }
    }

    #[func]
    fn _spawn_cameras(&mut self) {
        self.color_cameras.clear();
        self.depth_cameras.clear();

        let children = self.base().get_children();
        for i in 0..children.len() {
            let child = children.get(i).unwrap();
            let name = child.get_name().to_string();
            if name.contains("FaceViewport_") || name.contains("FaceCamera_") || name.contains("DepthViewport_") || name.contains("DepthCamera_") {
                child.clone().free();
            }
        }

        let face_rotations = [
            Vector3::new(0.0, -90.0, 0.0), // +X face
            Vector3::new(0.0, 90.0, 0.0),  // -X face
            Vector3::new(90.0, 0.0, 0.0),  // +Y face
            Vector3::new(-90.0, 0.0, 0.0), // -Y face
            Vector3::new(0.0, 180.0, 0.0), // +Z face
            Vector3::new(0.0, 0.0, 0.0),   // -Z face
        ];

        let world = self.base().get_world_3d();

        let mut face_res = 512;
        if let Some(render_mgr) = self.base().get_node_or_null("/root/RenderManager") {
            let settings_variant = render_mgr.get("settings");
            if let Ok(settings) = settings_variant.try_to::<Gd<Object>>() {
                face_res = settings
                    .get("face_resolution")
                    .try_to::<i32>()
                    .unwrap_or(512);
            }
        }

        // Spawn Color Cameras
        for (i, &rotation) in face_rotations.iter().enumerate() {
            let mut vp_gd = SubViewport::new_alloc();
            vp_gd.set_name(&format!("FaceViewport_{}", i));
            vp_gd.set_size(Vector2i::new(face_res, face_res));
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
            self.color_cameras.push(&cam_gd);
        }

        // Spawn Depth Cameras
        for (i, &rotation) in face_rotations.iter().enumerate() {
            let mut vp_gd = SubViewport::new_alloc();
            vp_gd.set_name(&format!("DepthViewport_{}", i));
            vp_gd.set_size(Vector2i::new(face_res, face_res));
            vp_gd.set_update_mode(godot::classes::sub_viewport::UpdateMode::DISABLED);
            vp_gd.set_clear_mode(godot::classes::sub_viewport::ClearMode::ALWAYS);

            if let Some(w) = &world {
                vp_gd.set_world_3d(w);
            }

            let mut cam_gd = Camera3D::new_alloc();
            cam_gd.set_name(&format!("DepthCamera_{}", i));
            cam_gd.set_fov(90.0);
            cam_gd.set_rotation_degrees(rotation);
            cam_gd.set_cull_mask(self.cull_mask);

            // Add depth capture quad
            let mut quad = MeshInstance3D::new_alloc();
            let mut quad_mesh = QuadMesh::new_gd();
            quad_mesh.set_flip_faces(true);
            quad_mesh.set_size(Vector2::new(2.0, 2.0));
            quad.set_mesh(&quad_mesh.upcast::<Mesh>());
            
            let mut mat = ShaderMaterial::new_gd();
            if let Some(shader) = godot::classes::ResourceLoader::singleton()
                .load("res://Shaders/depth_capture.gdshader")
                .map(|res| res.cast::<Shader>())
            {
                mat.set_shader(&shader);
            }
            quad.set_material_override(&mat.clone().upcast::<Material>());
            quad.set_extra_cull_margin(10000.0);
            
            cam_gd.add_child(&quad);

            vp_gd.add_child(&cam_gd);
            self.base_mut().add_child(&vp_gd);
            self.depth_cameras.push(&cam_gd);
        }
    }
}