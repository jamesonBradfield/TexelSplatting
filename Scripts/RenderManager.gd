class_name RenderManager
extends Node

@export var settings: RenderSettings


func _ready():
	if settings == null:
		push_warning("RenderManager: No RenderSettings assigned. Creating default settings.")
		settings = RenderSettings.new()

	_apply_settings()


func _apply_settings():
	# No more dividing by 6.0! All cameras fire in one tick.
	var target_ms_per_frame = 1000.0 / float(settings.target_fps)

	print("RenderManager: Target FPS is ", settings.target_fps)
	print("RenderManager: Calculated RealtimeProbe tick_rate_ms = ", target_ms_per_frame)

	# Make sure you rename your node group in the editor to "realtime_probes"
	var probes = get_tree().get_nodes_in_group("realtime_probes")
	for probe in probes:
		if probe.has_method("set_tick_rate_ms") or "tick_rate_ms" in probe:
			probe.tick_rate_ms = target_ms_per_frame

		# Access the array of cameras defined in Rust
		var cameras = probe.get("cameras")
		if cameras:
			for camera in cameras:
				if is_instance_valid(camera):
					# Each probe camera only sees the "Real World"
					camera.cull_mask = settings.real_world_mask
					camera.fov = 90.0

					# Find the SubViewport parent of this specific camera
					var parent = camera.get_parent()
					while parent != null:
						if parent is SubViewport:
							parent.size = Vector2i(
								settings.face_resolution, settings.face_resolution
							)
							# parent.render_target_update_mode = SubViewport.UPDATE_ALWAYS
							break
						parent = parent.get_parent()

	# Configure Player Cameras
	var player_cameras = get_tree().get_nodes_in_group("player_cameras")
	if player_cameras.is_empty():
		var main_cam = get_viewport().get_camera_3d()
		if main_cam:
			main_cam.cull_mask = settings.fake_world_mask
	else:
		for cam in player_cameras:
			if cam is Camera3D:
				cam.cull_mask = settings.fake_world_mask


# If you change the settings at runtime, call this to update everything
func update_target_fps(new_fps: int):
	settings.target_fps = new_fps
	_apply_settings()
