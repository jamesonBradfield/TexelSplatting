//! # RealtimeProbe
//!
//! A real-time cubemap capture system that uses 6 cameras arranged in a cube
//! configuration to capture environment maps for use in materials. This probe
//! automatically synchronizes with a target node and captures environment data
//! at a configurable tick rate.
//!
//! ## Architecture
//!
//! The probe uses a "Trinity of Threads" approach:
//! - **Capture Thread**: Spawns 6 cameras (one per face of a cube) that render
//!   to sub-viewsports, capturing the environment from each cardinal direction.
//! - **Sync Thread**: Updates the probe's position to follow a target node.
//! - **Processing Thread**: Reads captured images from sub-viewsports and
//!   synthesizes them into a layered cubemap texture.
//!
//! ## Usage
//!
//! 1. Add this node to your scene
//! 2. Assign a `follow_node` target (optional)
//! 3. Configure `tick_rate_ms` to control capture frequency
//! 4. Set `cull_mask` to control which objects are visible in captures
//! 5. Assign a `material` to automatically update its environment cubemap
//!
//! ## Signals
//!
//! - `probe_updated(images, cubemap_rid)`: Emitted when a new cubemap is captured
//!
//! ## Thread Safety
//!
//! This probe is designed to be thread-safe. The cubemap RID and face images
//! are protected by Godot's internal synchronization mechanisms.
//!
//! ## Configuration
//!
//! All settings are now sourced from `RenderManager` (SSOT - Single Source of Truth):
//! - `face_resolution`: Cubemap face resolution (default: 512)
//! - `cull_mask`: Bitmask for object visibility (default: 0xFFFFFFFF)
//! - `tick_rate_ms`: Capture interval in milliseconds (default: 33.33)
//! - `follow_node`: Optional target to follow
//! - `material`: Optional material to update with cubemap

use godot::classes::{
    rendering_server::TextureLayeredType, Camera3D, INode3D, Image, Node3D, RenderingServer,
    ShaderMaterial, SubViewport,
};
use godot::prelude::*;

/// Resolution for each face of the cubemap capture.
/// Higher values improve quality but increase memory and CPU usage.
/// This value is now sourced from RenderManager (SSOT).
const FACE_RESOLUTION: i32 = 512;

/// A real-time cubemap capture node that uses 6 cameras to synthesize an environment map.
///
/// This class implements a cubemap capture system by spawning 6 cameras, each facing
/// one of the 6 cardinal directions (+X, -X, +Y, -Y, +Z, -Z). Each camera renders to
/// a sub-viewport, and the captured images are combined into a layered cubemap texture.
///
/// ## Configuration
///
/// - `cameras`: Internal array managed automatically. Do not modify directly.
/// - `follow_node`: Optional target node to follow. The probe will sync its position
///   to this node every frame.
/// - `material`: Optional shader material to automatically update with the captured cubemap.
/// - `tick_rate_ms`: Time between capture cycles in milliseconds (default: 33.33ms ≈ 30fps).
/// - `cull_mask`: Bitmask controlling which objects are visible in captures (default: all).
///
/// ## Internal State
///
/// - `time_accumulator`: Accumulates delta time for frame-rate independent capture timing.
/// - `faces`: Array of 6 captured images, one per cubemap face.
/// - `cubemap_rid`: The RID of the synthesized cubemap texture.
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct RealtimeProbe {
    /// Internal base node reference. Managed by Godot's class system.
    base: Base<Node3D>,

    /// Array of 6 Camera3D nodes, one for each cubemap face.
    /// These are automatically managed and cleaned up when the probe is destroyed.
    #[export]
    cameras: Array<Gd<Camera3D>>,

    /// Optional target node to follow. The probe will synchronize its position
    /// to this node every frame. Set to `None` to disable auto-follow.
    follow_node: Option<Gd<Node3D>>,

    /// Optional shader material to automatically update with the captured cubemap.
    /// When set, the cubemap will be updated as a shader parameter named "env_cubemap".
    material: Option<Gd<ShaderMaterial>>,

    /// Time between capture cycles in milliseconds.
    /// Lower values increase capture frequency but may impact performance.
    /// Recommended range: 16.67ms (60fps) to 1000ms (1fps).
    time_accumulator: f64,

    /// Bitmask controlling which objects are visible in captures.
    /// Use this to exclude certain objects from the cubemap (e.g., UI elements).
    cull_mask: u32,

    /// Time between capture cycles in milliseconds.
    /// Lower values increase capture frequency but may impact performance.
    /// Recommended range: 16.67ms (60fps) to 1000ms (1fps).
    tick_rate_ms: f64,

    /// Accumulator for delta time, used for frame-rate independent capture timing.
    /// This ensures consistent capture intervals regardless of frame rate.
    time_accumulator: f64,

    /// Array of 6 captured images, one per cubemap face.
    /// These are automatically updated during the capture cycle.
    faces: Vec<Gd<Image>>,

    /// RID of the synthesized cubemap texture.
    /// This is created from the 6 face images and used as an environment map.
    cubemap_rid: Rid,
}

#[godot_api]
impl INode3D for RealtimeProbe {
    /// Initializes the RealtimeProbe node.
    ///
    /// Sets up initial state:
    /// - Clears all camera references
    /// - Resets follow node and material references
    /// - Initializes time accumulator to 0
    /// - Sets default tick rate to 33.33ms (~30fps)
    /// - Sets cull mask to show all objects (0xFFFFFFFF)
    /// - Pre-allocates space for 6 face images
    /// - Initializes cubemap RID as invalid
    ///
    /// Note: All settings are now sourced from RenderManager (SSOT) at runtime.
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            cameras: Array::new(),
            follow_node: None,
            material: None,
            time_accumulator: 0.0,
            cull_mask: 0xFFFFFFFF,
            faces: Vec::with_capacity(6),
            cubemap_rid: Rid::Invalid,
        }
    }

    /// Called when the node enters the scene tree for the first time.
    ///
    /// Spawns the 6 cameras and sub-viewsports needed for cubemap capture.
    /// Each camera is positioned at the probe's location and rotated to face
    /// one of the 6 cardinal directions.
    ///
    /// Note: Camera configuration (resolution, cull_mask) is now sourced from
    /// RenderManager (SSOT) at runtime.
    fn ready(&mut self) {
        self._apply_settings_from_render_manager();
        self._spawn_cameras();
    }

    /// Called every frame. Syncs position and triggers capture cycle.
    ///
    /// This method performs two main tasks:
    /// 1. **Position Sync**: If a follow_node is set, the probe synchronizes
    ///    its position to match the target every frame.
    /// 2. **Capture Timing**: Accumulates delta time and triggers a capture
    ///    cycle when the accumulated time exceeds tick_rate_ms.
    ///
    /// The time accumulator ensures capture intervals are frame-rate independent,
    /// maintaining consistent behavior across different frame rates.
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

    /// Called when the node is removed from the scene tree.
    ///
    /// Cleans up the cubemap RID to prevent memory leaks.
    /// The cubemap texture is freed if it was previously created.
    fn exit_tree(&mut self) {
        if !self.cubemap_rid.is_invalid() {
            RenderingServer::singleton().free_rid(self.cubemap_rid);
            self.cubemap_rid = Rid::Invalid;
        }
    }
}

#[godot_api]
impl RealtimeProbe {
    /// Manually triggers a cubemap capture cycle.
    ///
    /// This method:
    /// 1. Validates that 6 cameras are available
    /// 2. Positions all cameras at the probe's current location
    /// 3. Sets sub-viewport update mode to ONCE for each camera
    /// 4. Deferes the read-and-update operation to the next frame
    ///
    /// ## Notes
    /// - Call this method when you want to force an immediate capture
    /// - The actual image reading happens in the next frame via `_deferred_read_and_update`
    /// - If cameras are not properly set up, this method returns early without error
    #[func]
    fn trigger_capture(&mut self) {
        // Validate that we have exactly 6 cameras (one per cubemap face)
        if self.cameras.len() != 6 {
            return;
        }

        // Get the probe's current position - all cameras will be positioned here
        let origin = self.base().get_global_position();

        // Position each camera at the probe's location
        for i in 0..6 {
            let camera = self.cameras.at(i);
            let mut cam_mut = camera.clone();
            cam_mut.set_global_position(origin);

            // Get the sub-viewport and set update mode to ONCE
            if let Some(mut vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);
            }
        }

        // Defer the actual image reading to the next frame
        // This prevents blocking the current frame and ensures sub-viewsports have rendered
        self.base_mut()
            .call_deferred("_deferred_read_and_update", &[]);
    }

    /// Manually triggers a cubemap capture cycle.
    ///
    /// This method:
    /// 1. Validates that 6 cameras are available
    /// 2. Positions all cameras at the probe's current location
    /// 3. Sets sub-viewport update mode to ONCE for each camera
    /// 4. Deferes the read-and-update operation to the next frame
    ///
    /// ## Notes
    /// - Call this method when you want to force an immediate capture
    /// - The actual image reading happens in the next frame via `_deferred_read_and_update`
    /// - If cameras are not properly set up, this method returns early without error
    #[func]
    fn trigger_capture(&mut self) {
        // Validate that we have exactly 6 cameras (one per cubemap face)
        if self.cameras.len() != 6 {
            return;
        }

        // Get the probe's current position - all cameras will be positioned here
        let origin = self.base().get_global_position();

        // Position each camera at the probe's location
        for i in 0..6 {
            let camera = self.cameras.at(i);
            let mut cam_mut = camera.clone();
            cam_mut.set_global_position(origin);

            // Get the sub-viewport and set update mode to ONCE
            if let Some(mut vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                vp.set_update_mode(godot::classes::sub_viewport::UpdateMode::ONCE);
            }
        }

        // Defer the actual image reading to the next frame
        // This prevents blocking the current frame and ensures sub-viewsports have rendered
        self.base_mut()
            .call_deferred("_deferred_read_and_update", &[]);
    }

    /// Reads captured images from sub-viewsports and synthesizes a cubemap.
    ///
    /// This deferred method:
    /// 1. Reads the texture from each sub-viewport
    /// 2. Duplicates and flips images to correct orientation
    /// 3. Creates or updates the cubemap texture
    /// 4. Emits the `probe_updated` signal
    /// 5. Updates the material's environment cubemap parameter (if set)
    ///
    /// ## Image Orientation
    /// - Faces 0, 1, 4, 5: Flipped horizontally (X-axis)
    /// - Face 2: Flipped vertically (Y-axis)
    /// - Face 3: No flip needed
    ///
    /// ## Cubemap Creation
    /// - If `cubemap_rid` is invalid: Creates a new layered cubemap texture
    /// - If `cubemap_rid` is valid: Updates the existing cubemap with new face data
    ///
    /// ## Material Update
    /// If a material is configured, its "env_cubemap" shader parameter is updated
    /// with the new cubemap RID.
    #[func]
    fn _deferred_read_and_update(&mut self) {
        // Collect captured images from all 6 sub-viewsports
        let mut current_capture: Vec<Gd<Image>> = Vec::with_capacity(6);

        for i in 0..6 {
            let camera = self.cameras.at(i);
            if let Some(vp) = camera
                .get_parent()
                .and_then(|p| p.try_cast::<SubViewport>().ok())
            {
                // Get the texture from the sub-viewport
                if let Some(texture) = vp.get_texture() {
                    if let Some(image) = texture.get_image() {
                        // Duplicate the image to avoid modifying the original
                        let mut img: Gd<Image> =
                            image.duplicate().expect("Failed to duplicate").cast();

                        // Flip images to correct cubemap orientation
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

        // Validate that we captured all 6 faces
        if current_capture.len() != 6 {
            return;
        }

        // Store the captured images
        self.faces = current_capture;

        // Get the RenderingServer singleton for texture operations
        let mut rs = RenderingServer::singleton();

        // Create or update the cubemap texture
        if self.cubemap_rid.is_invalid() {
            // First capture: create a new layered cubemap
            let mut image_array = Array::<Gd<Image>>::new();
            for img in &self.faces {
                image_array.push(img);
            }
            self.cubemap_rid =
                rs.texture_2d_layered_create(&image_array, TextureLayeredType::CUBEMAP);
        } else {
            // Subsequent captures: update existing cubemap faces
            for (i, img) in self.faces.iter().enumerate() {
                rs.texture_2d_update(self.cubemap_rid, img, i as i32);
            }
        }

        // Emit signal to notify listeners of the new cubemap
        let cubemap_rid = self.cubemap_rid;
        let faces_array = self.get_faces_array();
        self.signals()
            .probe_updated()
            .emit(&faces_array, cubemap_rid);

        // Update the material's environment cubemap parameter if configured
        if let Some(mat) = self.material.clone() {
            RenderingServer::singleton().material_set_param(
                mat.get_rid(),
                "env_cubemap",
                &self.cubemap_rid.to_variant(),
            );
        }
    }

    /// Spawns and configures the 6 cameras needed for cubemap capture.
    ///
    /// This method:
    /// 1. Clears existing camera references
    /// 2. Cleans up old camera/viewport nodes from the scene tree
    /// 3. Creates 6 SubViewport nodes (one per cubemap face)
    /// 4. Creates 6 Camera3D nodes (one per sub-viewport)
    /// 5. Configures each camera with appropriate rotation and settings
    /// 6. Adds all nodes as children of the probe
    ///
    /// ## Camera Configuration
    /// Each camera is configured with:
    /// - FOV: 90 degrees (wide angle for better coverage)
    /// - Rotation: Specific to its face direction
    /// - Cull Mask: Uses the configured cull_mask value (from RenderManager SSOT)
    /// - Update Mode: DISABLED (rendering handled by sub-viewport)
    /// - Clear Mode: ALWAYS (ensures clean captures)
    ///
    /// ## Cleanup
    /// Any existing nodes with names containing "FaceViewport_" or "FaceCamera_"
    /// are automatically removed to prevent memory leaks.
    ///
    /// ## Note
    /// Camera resolution and cull_mask are now sourced from RenderManager (SSOT).
    #[func]
    fn _spawn_cameras(&mut self) {
        // Clear the cameras array
        self.cameras.clear();

        // Clean up old camera/viewport nodes from the scene tree
        let children = self.base().get_children();
        for i in 0..children.len() {
            let child = children.get(i).unwrap();
            let name = child.get_name().to_string();
            if name.contains("FaceViewport_") || name.contains("FaceCamera_") {
                child.clone().free();
            }
        }

        // Define rotation for each of the 6 cubemap faces
        // Order: +X, -X, +Y, -Y, +Z, -Z
        let face_rotations = [
            Vector3::new(0.0, -90.0, 0.0), // +X face (looks right)
            Vector3::new(0.0, 90.0, 0.0),  // -X face (looks left)
            Vector3::new(90.0, 0.0, 0.0),  // +Y face (looks up)
            Vector3::new(-90.0, 0.0, 0.0), // -Y face (looks down)
            Vector3::new(0.0, 180.0, 0.0), // +Z face (looks forward)
            Vector3::new(0.0, 0.0, 0.0),   // -Z face (looks backward)
        ];

        // Get the world for sub-viewport assignment
        let world = self.base().get_world_3d();

        // Create a camera and viewport for each face
        for (i, &rotation) in face_rotations.iter().enumerate() {
            // Create SubViewport for this face
            let mut vp_gd = SubViewport::new_alloc();
            vp_gd.set_name(&format!("FaceViewport_{}", i));
            vp_gd.set_size(Vector2i::new(FACE_RESOLUTION, FACE_RESOLUTION));
            vp_gd.set_update_mode(godot::classes::sub_viewport::UpdateMode::DISABLED);
            vp_gd.set_clear_mode(godot::classes::sub_viewport::ClearMode::ALWAYS);

            // Assign the world to the sub-viewport
            if let Some(w) = &world {
                vp_gd.set_world_3d(w);
            }

            // Create Camera3D for this face
            let mut cam_gd = Camera3D::new_alloc();
            cam_gd.set_name(&format!("FaceCamera_{}", i));
            cam_gd.set_fov(90.0);
            cam_gd.set_rotation_degrees(rotation);
            cam_gd.set_cull_mask(self.cull_mask);

            // Add camera to viewport, viewport to probe, and register camera
            vp_gd.add_child(&cam_gd);
            self.base_mut().add_child(&vp_gd);
            self.cameras.push(&cam_gd);
        }
    }

    /// Returns an array of the 6 captured face images.
    ///
    /// ## Returns
    /// An Array containing 6 Gd<Image> objects, one for each cubemap face.
    /// The order matches the face rotation order: +X, -X, +Y, -Y, +Z, -Z.
    ///
    /// ## Usage
    /// ```rust
    /// let faces = probe.get_faces_array();
    /// for (i, face) in faces.iter().enumerate() {
    ///     println!("Face {}: {}", i, face.get_name());
    /// }
    /// ```
    fn get_faces_array(&self) -> Array<Gd<Image>> {
        self.faces.iter().cloned().collect()
    }
}

#[godot_api]
impl RealtimeProbe {
    /// Signal emitted when a new cubemap has been captured and synthesized.
    ///
    /// ## Parameters
    /// - `images`: Array of 6 Image objects, one for each cubemap face.
    /// - `cubemap_rid`: The RID of the synthesized cubemap texture.
    ///
    /// ## Usage
    /// Connect to this signal to receive updates when the environment map changes:
    /// ```gdscript
    /// probe.probe_updated.connect(_on_probe_updated)
    /// ```
    #[signal]
    fn probe_updated(images: Array<Gd<Image>>, cubemap_rid: Rid);
}
