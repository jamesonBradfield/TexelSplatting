extends RealtimeProbe
class_name CubemapBuilder

@export var my_cubemap: Cubemap

# Optional: Set defaults here so they exist before RenderManager updates them
@export var default_face_resolution: int = 256

# Reference to the RenderManager singleton for accessing render settings
@export var render_manager: RenderManager


func _ready():
	my_cubemap = Cubemap.new()

	# Spawn our viewports and cameras before connecting the signal
	_spawn_cameras()

	# Connect the Rust signal to our local function
	probe_updated.connect(_on_probe_updated)


func _spawn_cameras():
	# Ensure the Rust array is empty before we populate it
	cameras.clear()

	# Define rotations for the 6 cubemap faces: +X, -X, +Y, -Y, +Z, -Z
	var face_rotations = [
		Vector3(0, -90, 0),  # 0: +X (Right)
		Vector3(0, 90, 0),  # 1: -X (Left)
		Vector3(90, 0, 0),  # 2: +Y (Top)
		Vector3(-90, 0, 0),  # 3: -Y (Bottom)
		Vector3(0, 180, 0),  # 4: +Z (Back)
		Vector3(0, 0, 0)  # 5: -Z (Front)
	]

	for i in range(6):
		# 1. Create the SubViewport
		var vp = SubViewport.new()
		vp.name = "FaceViewport_" + str(i)
		vp.size = Vector2i(default_face_resolution, default_face_resolution)
		vp.render_target_update_mode = SubViewport.UPDATE_ONCE

		# ADD THIS LINE: Tell the viewport to look at the main 3D world!
		vp.world_3d = get_world_3d()

		# 2. Create the Camera3D
		var cam = Camera3D.new()
		cam.name = "FaceCamera_" + str(i)
		cam.fov = 90.0  # Essential for perfect cubemap stitching
		cam.environment = get_viewport().get_camera_3d().environment

		# Set the rotation to face the correct cubemap direction
		cam.rotation_degrees = face_rotations[i]

		# Apply cull_mask from RenderSettings to match layer configuration
		# This ensures cameras only render the real world layer, not the fake world
		if render_manager:
			var settings = render_manager.settings
			if settings:
				cam.cull_mask = settings.real_world_mask
			else:
				cam.cull_mask = 1  # real_world_mask fallback
		else:
			cam.cull_mask = 1  # real_world_mask fallback

		# 3. Build the node tree
		vp.add_child(cam)
		add_child(vp)

		# 4. Push the camera reference to the Rust array
		cameras.append(cam)


func _on_probe_updated(faces: Array[Image]):
	# Grab the 6 raw images from Rust
	if faces.size() == 6 and faces[0] != null:
		# Create a texture from the 6 faces!
		my_cubemap.create_from_images(faces)

		# Now pass my_cubemap to your splatting shader,
		# or send it back into your MassRenderingNode!
