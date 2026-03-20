extends RealtimeProbe
class_name CubemapBuilder

@export var my_cubemap: Cubemap

# Optional: Set defaults here so they exist before RenderManager updates them
@export var default_face_resolution: int = 256

# Reference to the RenderManager singleton for accessing render settings
@export var render_manager: RenderManager


func _ready():
	# Spawn our viewports and cameras
	_spawn_cameras()

	# Connect the Rust signal to our local function
	self.emit_signal("probe_updated", [my_cubemap])




func _on_probe_updated(faces: Array[Image]):
	# Grab the 6 raw images from Rust
	if faces.size() == 6 and faces[0] != null:
		# Create a cubemap from the captured faces using Rust
		var cubemap_rid = probe.create_cubemap_from_faces()
		if cubemap_rid != null:
			# Set the cubemap as a shader parameter or use it directly
			my_cubemap = cubemap_rid.get_cubemap()
			# Now pass my_cubemap to your splatting shader,
			# or send it back into your MassRenderingNode!
