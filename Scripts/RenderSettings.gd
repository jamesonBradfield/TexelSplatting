extends Resource
class_name RenderSettings

# Core rendering settings
@export var target_fps: int = 24
@export var face_resolution: int = 512

# Layer masks for rendering
# Layer 1 (bit 0): The Real 3D World (geometry, lights, etc.)
# Layer 2 (bit 1): The Fake World (the inverted cubemap cube, texel splats)
@export_flags_3d_render var real_world_mask: int = 1  # 0001 (Layer 1)
@export_flags_3d_render var fake_world_mask: int = 2  # 0010 (Layer 2)

# RealtimeProbe settings (moved from RealtimeProbe exports)
@export var probe_tick_rate_ms: float = 33.33
@export var probe_cull_mask: int = 0xFFFFFFFF

# FakeWorld settings (moved from FakeWorld exports)
@export var fake_world_cull_mask: int = 0xFFFFFFFF
