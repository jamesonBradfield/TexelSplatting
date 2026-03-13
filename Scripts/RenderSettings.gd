extends Resource
class_name RenderSettings

@export var target_fps: int = 24
@export var face_resolution: int = 256

# Layer 1 (bit 0): The Real 3D World (geometry, lights, etc.)
# Layer 2 (bit 1): The Fake World (the inverted cubemap cube, texel splats)

@export_flags_3d_render var real_world_mask: int = 1  # 0001 (Layer 1)
@export_flags_3d_render var fake_world_mask: int = 2  # 0010 (Layer 2)
