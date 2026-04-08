use crate::realtime_probe::RealtimeProbe;
use godot::classes::{
    geometry_instance_3d::ShadowCastingSetting,
    image::Format,
    Camera3D,
    IMeshInstance3D,
    Image,
    ImageTexture,
    Material,
    MeshInstance3D,
    Node3D,
    RenderingServer,
    Shader,
    ShaderMaterial,
    Texture2D,
};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=MeshInstance3D)]
pub struct FakeWorld {
    #[export]
    probe: Option<Gd<Node3D>>,

    #[export]
    initial_palette: Option<Gd<Texture2D>>,

    player_camera: Option<Gd<Camera3D>>,
    material: Option<Gd<ShaderMaterial>>,
    pal: Option<Gd<Texture2D>>,

    // Settings are now sourced from RenderManager (SSOT)
    // These are read from RenderManager at runtime, not exported here
    _face_resolution: i32,
    _cull_mask: u32,
    _fake_world_cull_mask: u32,

    base: Base<MeshInstance3D>,
}

#[godot_api]
impl IMeshInstance3D for FakeWorld {
    /// Applies settings from RenderManager (SSOT) to this FakeWorld instance.
    ///
    /// This method reads the current settings from RenderManager and updates
    /// the FakeWorld's configuration. Should be called in `ready()`.
    fn _apply_settings_from_render_manager(&mut self) {
        if let Some(render_mgr) = self.base().get_node_or_null("/root/RenderManager") {
            let settings_variant = render_mgr.get("settings");
            if !settings_variant.is_nil() {
                if let Ok(settings) = settings_variant.try_to::<Gd<Object>>() {
                    // Read face_resolution from SSOT
                    let face_res_variant = settings.get("face_resolution");
                    if !face_res_variant.is_nil() {
                        if let Ok(face_res) = face_res_variant.try_to::<i32>() {
                            self._face_resolution = face_res;
                        }
                    }
                        
                    // Read fake_world_cull_mask from SSOT
                    let fake_world_cull_mask_variant = settings.get("fake_world_cull_mask");
                    if !fake_world_cull_mask_variant.is_nil() {
                        if let Ok(fake_world_cull_mask) = fake_world_cull_mask_variant.try_to::<u32>() {
                            self._fake_world_cull_mask = fake_world_cull_mask;
                        }
                    }
                        
                    // Read fake_world_mask from SSOT
                    let fake_world_mask_variant = settings.get("fake_world_mask");
                    if !fake_world_mask_variant.is_nil() {
                        if let Ok(fake_world_mask) = fake_world_mask_variant.try_to::<u32>() {
                            self.base_mut().set_layer_mask(fake_world_mask);
                        }
                    }
                }
            }
        }
    }
    fn init(base: Base<MeshInstance3D>) -> Self {
        Self {
            probe: None,
            initial_palette: None,
            player_camera: None,
            material: None,
            pal: None,
            _face_resolution: 0,
            _cull_mask: 0,
            _fake_world_cull_mask: 0,
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_cast_shadows_setting(ShadowCastingSetting::OFF);

        let mut tree = self.base().get_tree().unwrap();

        let cameras = tree.get_nodes_in_group("player_cameras");
        if !cameras.is_empty() {
            self.player_camera = cameras.at(0).try_cast::<Camera3D>().ok();
        }

        // Get settings from RenderManager (SSOT - Single Source of Truth)
        if let Some(render_mgr) = self.base().get_node_or_null("/root/RenderManager") {
            let settings_variant = render_mgr.get("settings");
            if !settings_variant.is_nil() {
                if let Ok(settings) = settings_variant.try_to::<Gd<Object>>() {
                    // Read face_resolution from SSOT
                    let face_res_variant = settings.get("face_resolution");
                    if !face_res_variant.is_nil() {
                        if let Ok(face_res) = face_res_variant.try_to::<i32>() {
                            self._face_resolution = face_res;
                        }
                    }
                    
                    // Read cull_mask from SSOT
                    let cull_mask_variant = settings.get("cull_mask");
                    if !cull_mask_variant.is_nil() {
                        if let Ok(cull_mask) = cull_mask_variant.try_to::<u32>() {
                            self._cull_mask = cull_mask;
                        }
                    }
                    
                    let mask = settings.get("fake_world_mask").try_to::<u32>().unwrap_or(1);
                    self.base_mut().set_layer_mask(mask);
                }
            }
        }

        if self.pal.is_none() {
            self.pal = self.initial_palette.clone();
        }

        if self.material.is_none() {
            if let Some(mat) = self.base().get_material_override() {
                self.material = mat.try_cast::<ShaderMaterial>().ok();
            } else if let Some(mat) = self.base().get_active_material(0) {
                self.material = mat.try_cast::<ShaderMaterial>().ok();
            }
        }

        if self.material.is_none() {
            self.material = Some(ShaderMaterial::new_gd());
        }

        if let Some(shader) = godot::classes::ResourceLoader::singleton()
            .load("res://Shaders/fake_world.gdshader")
            .map(|res| res.cast::<Shader>())
        {
            let mut mat = self.material.as_ref().unwrap().clone();
            mat.set_shader(&shader);

            if let Some(palette) = &self.pal {
                mat.set_shader_parameter("palette", &palette.to_variant());
            }
            
            RenderingServer::singleton().material_set_param(mat.get_rid(), "env_cubemap", &Rid::Invalid.to_variant());
            
            let mat_gd = mat.upcast::<Material>();
            self.base_mut().set_material_override(&mat_gd);
        }

            if let Ok(mut probe) = probe_node.try_cast::<RealtimeProbe>() {
                let callable = self.base().callable("_on_probe_cycle_complete");
                if !probe.is_connected("probe_updated", &callable) {
                    probe.connect("probe_updated", &callable);
                }
            }
        }
    }


    fn process(&mut self, _delta: f64) {
        if let Some(probe_node) = self.probe.clone() {
            let probe_pos = probe_node.get_global_position();
            if self.base().get_global_position() != probe_pos {
                self.base_mut().set_global_position(probe_pos);
            }
        }
    }
}

#[godot_api]
impl FakeWorld {
    #[func]
    fn _on_probe_cycle_complete(
        &mut self,
        _faces: Array<Gd<Image>>,
        cubemap_rid: Rid,
    ) {
        if let Some(ref mut mat) = self.material {
            RenderingServer::singleton().material_set_param(mat.get_rid(), "env_cubemap", &cubemap_rid.to_variant());
        }
    }

    #[func]
    fn set_palette(&mut self, palette_texture: Gd<Texture2D>) {
        self.initial_palette = Some(palette_texture.clone());
        self.pal = Some(palette_texture.clone());
        if let Some(mut mat) = self.material.clone() {
            mat.set_shader_parameter("palette", &palette_texture.to_variant());
            self.material = Some(mat);
        }
    }

    #[func]
    pub fn get_cubemap_rid(&self) -> Rid {
        if let Some(ref mat) = self.material {
            let variant = RenderingServer::singleton().material_get_param(mat.get_rid(), "env_cubemap");
            variant.try_to::<Rid>().unwrap_or(Rid::Invalid)
        } else {
            Rid::Invalid
        }
    }

    #[func]
    pub fn get_face_resolution(&self) -> i32 {
        self._face_resolution
    }

    #[func]
    pub fn get_cull_mask(&self) -> u32 {
        self._fake_world_cull_mask
    }

    #[func]
    pub fn generate_palette_from_image(&self, source_image: Gd<Image>) -> Gd<Texture2D> {
        let mut palette_image =
            Image::create(16, 1, false, Format::RGBA8).expect("Failed to create palette Image");
        let mut sampled_colors: Vec<Color> = Vec::new();

        let width = source_image.get_width();
        let height = source_image.get_height();

        'outer: for y in 0..height {
            for x in 0..width {
                let color = source_image.get_pixel(x, y);

                let r = (color.r * 15.0).trunc() / 15.0;
                let g = (color.g * 15.0).trunc() / 15.0;
                let b = (color.b * 15.0).trunc() / 15.0;
                let quantized_color = Color::from_rgba(r, g, b, 1.0);

                let mut already_added = false;
                for existing in &sampled_colors {
                    let dr = existing.r - quantized_color.r;
                    let dg = existing.g - quantized_color.g;
                    let db = existing.b - quantized_color.b;
                    let dist = (dr * dr + dg * dg + db * db).sqrt();

                    if dist < 0.05 {
                        already_added = true;
                        break;
                    }
                }

                if !already_added {
                    sampled_colors.push(quantized_color);
                    if sampled_colors.len() >= 16 {
                        break 'outer;
                    }
                }
            }
        }

        for i in 0..16 {
            if i < sampled_colors.len() {
                palette_image.set_pixel(i as i32, 0, sampled_colors[i]);
            } else {
                palette_image.set_pixel(i as i32, 0, Color::from_rgba(1.0, 1.0, 1.0, 1.0));
            }
        }

        palette_image.generate_mipmaps();

        let tex =
            ImageTexture::create_from_image(&palette_image).expect("Failed to create ImageTexture");
        tex.upcast()
    }
}
