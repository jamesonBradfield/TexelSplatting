use godot::classes::{
    geometry_instance_3d::ShadowCastingSetting,
    image::Format,
    Camera3D,
    Cubemap,
    IMeshInstance3D,
    Image,
    ImageTexture,
    Material, // <-- Add Material here
    MeshInstance3D,
    Node3D,
    Shader,
    ShaderMaterial,
    Texture2D,
};
use godot::global::Error;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=MeshInstance3D)]
pub struct FakeWorld {
    #[export]
    probe: Option<Gd<Node3D>>,

    #[export]
    initial_palette: Option<Gd<Texture2D>>,

    player_camera: Option<Gd<Camera3D>>,
    cubemap: Gd<Cubemap>,
    material: Gd<ShaderMaterial>,

    base: Base<MeshInstance3D>,
}

#[godot_api]
impl IMeshInstance3D for FakeWorld {
    fn init(base: Base<MeshInstance3D>) -> Self {
        Self {
            probe: None,
            initial_palette: None,
            player_camera: None,
            cubemap: Cubemap::new_gd(),
            material: ShaderMaterial::new_gd(),
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut()
            .set_cast_shadows_setting(ShadowCastingSetting::OFF);

        let mut tree = self.base().get_tree().unwrap();

        // Use string literals directly; gdext handles the AsArg<StringName> conversion automatically
        let cameras = tree.get_nodes_in_group("player_cameras");
        if !cameras.is_empty() {
            self.player_camera = cameras.at(0).try_cast::<Camera3D>().ok();
        }

        if let Some(render_mgr) = self.base().get_node_or_null("/root/RenderManager") {
            let settings_variant = render_mgr.get("settings");
            if !settings_variant.is_nil() {
                if let Ok(settings) = settings_variant.try_to::<Gd<Object>>() {
                    let mask = settings.get("fake_world_mask").try_to::<u32>().unwrap_or(1);
                    self.base_mut().set_layer_mask(mask);
                }
            }
        }

        if self.base().get_material_override().is_none() {
            let shader = load::<Shader>("res://Shaders/fake_world.gdshader");
            let mut mat = self.material.clone();
            mat.set_shader(&shader);

            if let Some(pal) = &self.initial_palette {
                mat.set_shader_parameter("palette", &pal.to_variant());
            }
            self.base_mut()
                .set_material_override(&mat.upcast::<Material>());
        } else {
            let mut mat = self
                .base()
                .get_material_override()
                .unwrap()
                .cast::<ShaderMaterial>();
            if let Some(pal) = &self.initial_palette {
                mat.set_shader_parameter("palette", &pal.to_variant());
            }
            self.material = mat;
        }

        if let Some(probe) = self.probe.clone() {
            let base_node = self.base().clone();
            let callable = base_node.callable("_on_probe_cycle_complete");
            probe.connect("probe_updated", &callable);
        } else {
            godot_warn!("FakeWorld: No probe assigned!");
        }
    }

    fn process(&mut self, _delta: f64) {
        if let Some(mut probe) = self.probe.clone() {
            probe.call("update_fake_world_position", &[]);
        }
    }
}

#[godot_api]
impl FakeWorld {
    #[func]
    fn _on_probe_cycle_complete(&mut self, faces: Array<Gd<Image>>, _depth_faces: Array<Gd<Image>>) {
        if faces.len() != 6 {
            godot_error!("FakeWorld: Expected 6 faces, got {}", faces.len());
            return;
        }
        
        godot_print!("FakeWorld: Creating cubemap from {} faces", faces.len());

        let mut typed_faces = Array::new();
        let mut first_format = Format::MAX;
        let mut first_width = 0;
        let mut first_height = 0;

        for i in 0..6 {
            let mut img = faces.at(i);
            if img.is_empty() {
                godot_warn!("FakeWorld: Face image {} is empty", i);
                return;
            }

            if i == 0 {
                first_format = img.get_format();
                first_width = img.get_width();
                first_height = img.get_height();
            } else if img.get_width() != first_width || img.get_format() != first_format {
                img.convert(first_format);
                img.resize(first_width, first_height);
            }

            // Pass by reference to satisfy the AsArg requirement for Gd<T> inside Arrays
            typed_faces.push(&img);
        }

        let err = self.cubemap.create_from_images(&typed_faces);
        if err == Error::OK {
            self.material
                .set_shader_parameter("env_cubemap", &self.cubemap.to_variant());
        } else {
            godot_error!("FakeWorld: Failed to create cubemap, error code: {:?}", err);
        }
    }

    #[func]
    fn set_palette(&mut self, palette_texture: Gd<Texture2D>) {
        self.initial_palette = Some(palette_texture.clone());
        self.material
            .set_shader_parameter("palette", &palette_texture.to_variant());
    }

    #[func]
    fn get_cubemap_rid(&self) -> Rid {
        // Returns the cubemap RID for debugging
        Rid::Invalid
    }

    #[func]
    fn generate_palette_from_image(&self, source_image: Gd<Image>) -> Gd<Texture2D> {
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

impl Drop for FakeWorld {
    fn drop(&mut self) {
        // Clean up cubemap if it exists
        if !self.cubemap.is_nil() {
            // Note: Cubemap doesn't have a free_rid method, but we can log cleanup
            godot_print!("FakeWorld: Cleaning up cubemap resource");
        }
    }
}
