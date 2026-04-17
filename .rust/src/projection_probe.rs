use godot::classes::{
    IMultiMeshInstance3D, MultiMeshInstance3D, MultiMesh, QuadMesh, RenderingServer, ShaderMaterial, Shader,
    Camera3D, SubViewport, MeshInstance3D, Mesh, Material,
};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=MultiMeshInstance3D)]
pub struct ProjectionProbe {
    base: Base<MultiMeshInstance3D>,
    
    #[export]
    cubemap_resolution: i32,
    
    #[export]
    quad_size: f32,
    
    #[export]
    cull_mask: u32,
    
    #[export]
    probe: Option<Gd<Node3D>>,
    
    material: Option<Gd<ShaderMaterial>>,
}

#[godot_api]
impl IMultiMeshInstance3D for ProjectionProbe {
    fn init(base: Base<MultiMeshInstance3D>) -> Self {
        Self {
            base,
            cubemap_resolution: 64,
            quad_size: 1.0,
            cull_mask: 1,
            probe: None,
            material: None,
        }
    }

    fn ready(&mut self) {
        self.setup_multimesh();
        self.setup_material();
        
        if let Some(probe_node) = self.probe.clone() {
            if let Ok(mut probe) = probe_node.try_cast::<crate::realtime_probe::RealtimeProbe>() {
                let callable = self.base().callable("_on_probe_cycle_complete");
                if !probe.is_connected("probe_updated", &callable) {
                    probe.connect("probe_updated", &callable);
                }
            }
        }
    }
}

#[godot_api]
impl ProjectionProbe {
    #[func]
    fn setup_multimesh(&mut self) {
        let res = self.cubemap_resolution;
        let total_texels = 6 * res * res;
        
        let mut multimesh = MultiMesh::new_gd();
        multimesh.set_transform_format(godot::classes::multi_mesh::TransformFormat::TRANSFORM_3D);
        multimesh.set_instance_count(total_texels);
        multimesh.set_visible_instance_count(total_texels);
        
        let mut mesh = QuadMesh::new_gd();
        mesh.set_size(Vector2::new(1.0, 1.0));
        multimesh.set_mesh(&mesh.upcast::<godot::classes::Mesh>());
        
        self.base_mut().set_multimesh(&multimesh);
        self.base_mut().set_custom_aabb(Aabb::new(Vector3::new(-10000.0, -10000.0, -10000.0), Vector3::new(20000.0, 20000.0, 20000.0)));
    }

    #[func]
    fn setup_material(&mut self) {
        let mut mat = ShaderMaterial::new_gd();
        if let Some(shader) = godot::classes::ResourceLoader::singleton()
            .load("res://Shaders/projection_probe.gdshader")
            .map(|res| res.cast::<Shader>())
        {
            mat.set_shader(&shader);
        }
        
        mat.set_shader_parameter("cubemap_resolution", &self.cubemap_resolution.to_variant());
        mat.set_shader_parameter("quad_size", &self.quad_size.to_variant());
        
        // Initialize with invalid cubemaps to avoid null texture issues
        let invalid_rid = Rid::Invalid;
        RenderingServer::singleton().material_set_param(mat.get_rid(), "env_cubemap", &invalid_rid.to_variant());
        RenderingServer::singleton().material_set_param(mat.get_rid(), "depth_cubemap", &invalid_rid.to_variant());
        
        self.base_mut().set_material_override(&mat.clone().upcast::<godot::classes::Material>());
        self.material = Some(mat);
    }
    
    #[func]
    fn _on_probe_cycle_complete(
        &mut self,
        color_cubemap_rid: Rid,
        depth_cubemap_rid: Rid,
    ) {
        self.set_env_cubemaps(color_cubemap_rid, depth_cubemap_rid);
    }
    
    #[func]
    pub fn set_env_cubemaps(&mut self, color_cubemap_rid: Rid, depth_cubemap_rid: Rid) {
        if let Some(mat) = self.material.clone() {
            RenderingServer::singleton().material_set_param(mat.get_rid(), "env_cubemap", &color_cubemap_rid.to_variant());
            RenderingServer::singleton().material_set_param(mat.get_rid(), "depth_cubemap", &depth_cubemap_rid.to_variant());
        }
    }
}
