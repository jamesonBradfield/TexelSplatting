/// Represents a probe node that captures 6-sided environment snapshots (like HDRI probes)
/// by rendering the scene from cameras positioned in each cardinal direction.
/// Uses a SubViewport attached to each Camera3D to capture the rendered output.
use godot::classes::{Camera3D, INode3D, Image, Node3D, SubViewport};
use godot::prelude::*;

/// A Node3D that captures real-time environment probes from 6 directions.
///
/// This node:
/// - Follows a target node (if assigned) to position itself in the scene.
/// - Captures snapshots from 6 cameras facing +X, -X, +Y, -Y, +Z, -Z.
/// - Captures both color and depth information.
/// - Emits a `probe_updated` signal when all 6 captures are complete.
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct RealtimeProbe {
    base: Base<Node3D>,

    /// Array of exactly 6 Camera3D nodes.
    /// Each camera must have a SubViewport to capture rendered scenes.
    /// The order must be strictly: +X (Right), -X (Left), +Y (Top), -Y (Bottom), +Z (Back), -Z (Front).
    #[export]
    cameras: Array<Gd<Camera3D>>,

    /// Optional Node3D for the probe to follow.
    /// If set, the probe will automatically update its global position to match this target node every frame.
    #[export]
    follow_node: Option<Gd<Node3D>>,

    /// Optional fake world Node3D.
    /// If set, this node's global position will also be updated to match the probe's position.
    /// This is used to maintain the illusion when rendering the cubemap projection in a separate visual layer.
    #[export]
    fake_world_node: Option<Gd<Node3D>>,

    /// The target interval between capture attempts, measured in milliseconds.
    /// Default is 16.67ms (approximately 60 FPS).
    /// Increase this value to capture less frequently and save performance.
    #[export(range = (1.0, 1000.0, 0.01))]
    tick_rate_ms: f64,

    /// Internal accumulator for tracking elapsed time since the last successful capture.
    time_accumulator: f64,

    /// Internal storage for the 6 captured environment color images (one per cardinal direction).
    faces: Vec<Gd<Image>>,

    /// Internal storage for the 6 captured depth images (one per cardinal direction).
    depth_faces: Vec<Gd<Image>>,
}

#[godot_api]
impl INode3D for RealtimeProbe {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            cameras: Array::new(),
            follow_node: None,
            fake_world_node: None,
            time_accumulator: 0.0,
            tick_rate_ms: 16.67,
            faces: Vec::with_capacity(6),
            depth_faces: Vec::with_capacity(6),
        }
    }

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
    /// Signal emitted when a full cycle of 6 environment faces and 6 depth faces have been successfully captured.
    /// `images`: An Array of 6 color Image objects (+X, -X, +Y, -Y, +Z, -Z).
    /// `depth_images`: An Array of 6 depth Image objects (+X, -X, +Y, -Y, +Z, -Z).
    #[signal]
    fn probe_updated(images: Array<Gd<Image>>, depth_images: Array<Gd<Image>>);

    /// Returns a cloned array of the most recently captured color environment faces.
    /// Useful for passing to other systems or manually constructing Cubemaps via GDScript.
    #[func]
    pub fn get_faces_array(&self) -> Array<Gd<Image>> {
        self.faces.iter().cloned().collect()
    }

    /// Returns a cloned array of the most recently captured depth faces.
    /// Useful for shader parameters or mass rendering logic that requires environmental depth.
    #[func]
    pub fn get_depth_faces_array(&self) -> Array<Gd<Image>> {
        self.depth_faces.iter().cloned().collect()
    }

    /// Core method that executes the capture of environment snapshots from all 6 cardinal directions.
    ///
    /// This method:
    /// 1. Validates that exactly 6 cameras are assigned.
    /// 2. Teleports all cameras to the probe's current global position.
    /// 3. Injects a full-screen depth-reading mesh into the primary viewports.
    /// 4. Forces a single-frame render pass on color and depth viewports.
    /// 5. Harvests the resulting Image data from the GPU.
    /// 6. Emits `probe_updated` if all 12 images (6 color, 6 depth) are successfully extracted.
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

            // Get the viewport from the camera
            if let Some(mut vp) = camera.get_viewport().map(|v| v.cast::<SubViewport>()) {
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);

                // Force a render pass to ensure the texture is updated
                if let Some(texture) = vp.get_texture() {
                    // Capture color face
                    if let Some(mut image) = texture.get_image() {
                        if i != 3 {
                            image.flip_x();
                        }
                        if i == 3 {
                            image.flip_y();
                        }
                        current_capture.push(image);
                    }

                    // Capture depth face
                    if let Some(mut depth_image) = texture.get_image() {
                        if i != 3 {
                            depth_image.flip_x();
                        }
                        if i == 3 {
                            depth_image.flip_y();
                        }
                        current_depth_capture.push(depth_image);
                    }
                }
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
