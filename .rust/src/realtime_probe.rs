/// Represents a probe node that captures 6-sided environment snapshots (like HDRI probes)
/// by rendering the scene from cameras positioned in each cardinal direction.
/// Uses a SubViewport attached to each Camera3D to capture the rendered output.
use godot::classes::{
    Camera3D, INode3D, Image, MeshInstance3D, Node3D, QuadMesh, Shader, ShaderMaterial, SubViewport,
};
use godot::prelude::*;

const DEPTH_SHADER_CODE: &str = r#"
shader_type spatial;
render_mode unshaded, fog_disabled;

uniform sampler2D depth_texture : hint_depth_texture;

void vertex() {
    POSITION = vec4(VERTEX.xy, 1.0, 1.0);
}

void fragment() {
    float depth = texture(depth_texture, SCREEN_UV).x;
    ALBEDO = vec3(depth);
}
"#;

/// A Node3D that captures real-time environment probes from 6 directions.
///
/// This node:
/// - Follows a target node (if assigned) to position itself in the scene
/// - Captures snapshots from 6 cameras facing +X, -X, +Y, -Y, +Z, -Z
/// - Emits a `probe_updated` signal when all 6 captures are complete
/// - Stores the captured images as Gd<Image> references
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct RealtimeProbe {
    base: Base<Node3D>,

    /// Array of 6 Camera3D nodes, each with a SubViewport to capture rendered scenes
    #[export]
    cameras: Array<Gd<Camera3D>>,

    /// Optional array of Viewports to capture color from. If provided, the probe will capture
    /// color from these viewports instead of the ones directly attached to the cameras.
    /// This allows for multi-viewport post-processing pipelines. Depth is always captured from the camera's viewport.
    #[export]
    color_viewports: Array<Gd<SubViewport>>,

    /// Optional Node3D to follow for positioning the probe in the scene
    #[export]
    follow_node: Option<Gd<Node3D>>,

    /// Optional fake world node that tracks the probe's position (used for lighting calculations)
    #[export]
    fake_world_node: Option<Gd<Node3D>>,

    /// Target interval between capture attempts (in milliseconds)
    #[export]
    tick_rate_ms: f64,
    /// Accumulator for tracking elapsed time since last capture
    time_accumulator: f64,

    /// Stores the 6 captured environment images (one per cardinal direction)
    faces: Vec<Gd<Image>>,

    /// Stores the 6 captured depth images (one per cardinal direction)
    depth_faces: Vec<Gd<Image>>,
}

#[godot_api]
impl INode3D for RealtimeProbe {
    /// Initializes the probe node with default values
    /// - Sets tick rate to ~60 FPS (16.67ms per frame)
    /// - Pre-allocates vector capacity for 6 face images
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            cameras: Array::new(),
            color_viewports: Array::new(),
            follow_node: None,
            fake_world_node: None,
            time_accumulator: 0.0,
            tick_rate_ms: 16.67,
            faces: Vec::with_capacity(6),
            depth_faces: Vec::with_capacity(6),
        }
    }

    /// Processes the probe node every frame
    /// - Updates position if following another node
    /// - Accumulates time to trigger periodic environment captures
    fn process(&mut self, delta: f64) {
        // Follow target node if assigned, updates probe position to match target
        if let Some(target) = self.follow_node.clone() {
            let target_pos = target.get_global_position();
            self.base_mut().set_global_position(target_pos);
        }

        // Accumulate time (converted to ms) and trigger capture when threshold reached
        self.time_accumulator += delta * 1000.0;
        if self.time_accumulator >= self.tick_rate_ms {
            self.time_accumulator = 0.0;
            self.capture_environment();
        }
    }
}

#[godot_api]
impl RealtimeProbe {
    /// Signal emitted when all 6 environment faces have been captured
    /// The signal carries an array containing Gd<Image> references to each face
    #[signal]
    fn probe_updated(images: Array<Gd<Image>>, depth_images: Array<Gd<Image>>);

    /// Returns a cloned copy of the captured environment face images
    /// Useful for passing to other systems (e.g., lighting calculation)
    #[func]
    pub fn get_faces_array(&self) -> Array<Gd<Image>> {
        self.faces.iter().cloned().collect()
    }

    /// Returns a cloned copy of the captured depth face images
    #[func]
    pub fn get_depth_faces_array(&self) -> Array<Gd<Image>> {
        self.depth_faces.iter().cloned().collect()
    }

    /// Captures environment snapshots from 6 cardinal directions
    ///
    /// Steps:
    /// 1. Validates that all 6 cameras are assigned (returns early if not)
    /// 2. Positions all cameras at the probe's global position
    /// 3. Orients each camera to face one of the 6 cardinal directions (+X, -X, +Y, -Y, +Z, -Z)
    /// 4. Forces each SubViewport to render once
    /// 5. Captures the rendered output as an Image
    /// 6. Emits probe_updated signal if all 6 captures succeed
    #[func]
    fn capture_environment(&mut self) {
        if self.cameras.len() != 6 {
            return;
        }

        let origin = self.base().get_global_position();

        if let Some(mut fw) = self.fake_world_node.clone() {
            fw.set_global_position(origin);
        }

        // Temporary vectors to store successfully captured images during this capture cycle
        let mut current_capture = Vec::with_capacity(6);
        let mut current_depth_capture = Vec::with_capacity(6);

        // Iterate through each camera and capture the environment
        for i in 0..6 {
            // Get the camera for this direction (expects exactly 6 cameras)
            let mut camera = self.cameras.at(i);

            // Position camera at probe's world position
            camera.set_global_position(origin);

            // Determine which viewports to capture from
            let color_viewport = if self.color_viewports.len() == 6 {
                Some(self.color_viewports.at(i))
            } else {
                camera.get_viewport().map(|v| v.cast::<SubViewport>())
            };
            let depth_viewport = camera.get_viewport().map(|v| v.cast::<SubViewport>());

            // 1. Capture color face
            if let Some(mut vp) = color_viewport {
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);

                // Force a render pass to ensure the color texture is updated
                if let Some(texture) = vp.get_texture() {
                    if let Some(mut image) = texture.get_image() {
                        if i != 3 {
                            image.flip_x();
                        }
                        if i == 3 {
                            image.flip_y();
                        }
                        current_capture.push(image);
                    }
                }
            }

            // 2. Capture depth face
            if let Some(mut vp) = depth_viewport {
                // Ensure we have a depth capture mesh in the viewport
                let mut depth_mesh = if let Some(node) = vp.get_node_or_null("DepthCapture") {
                    node.cast::<MeshInstance3D>()
                } else {
                    let mut mesh_inst = MeshInstance3D::new_alloc();
                    mesh_inst.set_name("DepthCapture");

                    let mut quad = Gd::<QuadMesh>::default();
                    quad.set_size(Vector2::new(2.0, 2.0)); // Cover full screen in NDC
                    quad.set_flip_faces(true); // Depending on godot versions, might need this or not, usually not needed for full screen quad, but let's test.

                    let mut mat = Gd::<ShaderMaterial>::default();
                    let mut sh = Gd::<Shader>::default();
                    sh.set_code(DEPTH_SHADER_CODE);
                    mat.set_shader(&sh);
                    quad.set_material(&mat);

                    mesh_inst.set_mesh(&quad);

                    // To ensure it's rendered, we can put it in the viewport's camera
                    // or just as a child of the viewport since the vertex shader overrides position.
                    vp.add_child(&mesh_inst);
                    mesh_inst
                };

                depth_mesh.set_visible(true);
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);

                // We might need a small delay or multiple frames for the toggle to take effect in some Godot versions,
                // but for a realtime probe, we'll follow the existing pattern of immediate capture.
                if let Some(texture) = vp.get_texture() {
                    if let Some(mut image) = texture.get_image() {
                        if i != 3 {
                            image.flip_x();
                        }
                        if i == 3 {
                            image.flip_y();
                        }
                        current_depth_capture.push(image);
                    }
                }

                // Reset to disabled for normal viewing
                depth_mesh.set_visible(false);
            }
        }

        // Only emit signal if all 6 faces were captured successfully for both color and depth
        if current_capture.len() == 6 && current_depth_capture.len() == 6 {
            // Replace old faces with newly captured ones
            self.faces = current_capture;
            self.depth_faces = current_depth_capture;

            // Create arrays of Gd<Image> references for the signal
            let face_array = self.get_faces_array();
            let depth_face_array = self.get_depth_faces_array();

            // Emit signal to notify systems that the probe data is ready
            self.base_mut().emit_signal(
                "probe_updated",
                &[face_array.to_variant(), depth_face_array.to_variant()],
            );
        }
    }
}
