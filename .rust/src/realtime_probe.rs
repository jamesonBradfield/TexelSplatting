/// Represents a probe node that captures 6-sided environment snapshots (like HDRI probes)
/// by rendering the scene from cameras positioned in each cardinal direction.
/// Uses a SubViewport attached to each Camera3D to capture the rendered output.
use godot::classes::{Camera3D, INode3D, Image, Node3D, SubViewport};
use godot::prelude::*;

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
            follow_node: None,
            fake_world_node: None,
            time_accumulator: 0.0,
            tick_rate_ms: 16.67,
            faces: Vec::with_capacity(6),
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
    fn probe_updated(images: Array<Gd<Image>>);

    /// Returns a cloned copy of the captured environment face images
    /// Useful for passing to other systems (e.g., lighting calculation)
    #[func]
    pub fn get_faces_array(&self) -> Array<Gd<Image>> {
        self.faces.iter().cloned().collect()
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

        // Temporary vector to store successfully captured images during this capture cycle
        let mut current_capture = Vec::with_capacity(6);

        // Iterate through each camera and capture the environment
        for i in 0..6 {
            // Get the camera for this direction (expects exactly 6 cameras)
            let mut camera = self.cameras.at(i);

            // Position camera at probe's world position
            camera.set_global_position(origin);

            // Extract and force render the SubViewport attached to this camera
            if let Some(viewport) = camera.get_viewport() {
                let mut vp = viewport.cast::<SubViewport>();

                // Force the viewport to render once (triggers actual scene rendering)
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);

                // Capture the rendered output as an image
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
        }

        // Only emit signal if all 6 faces were captured successfully
        if current_capture.len() == 6 {
            // Replace old faces with newly captured ones
            self.faces = current_capture;

            // Create array of Gd<Image> references for the signal
            let face_array = self.get_faces_array();

            // Emit signal to notify systems that the probe data is ready
            self.base_mut()
                .emit_signal("probe_updated", &[face_array.to_variant()]);
        }
    }
}
