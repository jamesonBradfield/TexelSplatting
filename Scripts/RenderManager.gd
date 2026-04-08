class_name RenderManager
extends Node

@export var settings: RenderSettings

# Single Source of Truth for all RealtimeProbe configuration
# All probe-related settings are defined in RenderSettings resource
# This node only applies those settings consistently across the system.


func _ready():
	if settings == null:
		push_warning("RenderManager: No RenderSettings assigned. Creating default settings.")
		settings = RenderSettings.new()

	_apply_settings()


func _apply_settings():
	# All settings come from RenderSettings (SSOT)
	print("RenderManager: Applying settings from SSOT")
	print("RenderManager: Target FPS = ", settings.target_fps)
	print("RenderManager: Face Resolution = ", settings.face_resolution)
	print("RenderManager: Probe Tick Rate = ", settings.probe_tick_rate_ms)

	# Apply settings to all RealtimeProbe instances
	var probes = get_tree().get_nodes_in_group("realtime_probes")
	for probe in probes:
		if probe.has_method("set_tick_rate_ms") or "tick_rate_ms" in probe:
			# Apply all settings from SSOT
			probe.tick_rate_ms = settings.probe_tick_rate_ms
			probe.cull_mask = settings.probe_cull_mask

			# Access the array of cameras defined in Rust
			var cameras = probe.get("cameras")
			if cameras:
				for camera in cameras:
					if is_instance_valid(camera):
						# Each probe camera only sees the "Real World"
						camera.cull_mask = settings.probe_cull_mask
						camera.fov = 90.0

						# Find the SubViewport parent of this specific camera
						var parent = camera.get_parent()
						while parent != null:
							if parent is SubViewport:
								# Apply face resolution from SSOT
								parent.size = Vector2i(
									settings.face_resolution, settings.face_resolution
								)
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

	# Apply settings to FakeWorld instances
	var fake_worlds = get_tree().get_nodes_in_group("fake_worlds")
	for fake_world in fake_worlds:
		if fake_world.has_method("set_palette") or "set_palette" in fake_world:
			# FakeWorld will apply its own settings from RenderManager in ready()
			pass


# Update settings at runtime - all changes go through RenderSettings (SSOT)
func update_target_fps(new_fps: int):
	settings.target_fps = new_fps
	_apply_settings()


# Update all probe settings from the single source of truth
func update_all_probe_settings():
	_apply_settings()
