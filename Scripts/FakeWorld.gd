extends MeshInstance3D
class_name FakeWorld

# Reference to the Rust AmortizedProbe node
@export var probe: Node3D
@export var palette: Texture2D
var player_camera: Camera3D
var _cubemap: Cubemap
var _material: ShaderMaterial


func _ready():
	self.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF

	_cubemap = Cubemap.new()
	player_camera = get_tree().get_first_node_in_group("player_cameras")
	# Try to get RenderSettings if RenderManager is autoloaded
	var render_mgr = get_node_or_null("/root/RenderManager")
	if render_mgr and render_mgr.settings:
		# Place this FakeWorld mesh on the Fake World layer so only player cameras see it
		self.layers = render_mgr.settings.fake_world_mask

	# Setup the ShaderMaterial dynamically if it's not set
	if material_override == null:
		_material = ShaderMaterial.new()
		var shader = load("res://Shaders/fake_world.gdshader")
		_material.shader = shader
		_material.set_shader_parameter("palette", palette)
		material_override = _material
	else:
		_material = material_override as ShaderMaterial
		_material.set_shader_parameter("palette", palette)

	if probe != null:
		# Connect to the Rust node's cycle completion signal
		if probe.has_signal("probe_updated"):
			probe.probe_updated.connect(_on_probe_cycle_complete)
		else:
			push_error(
				"FakeWorld: The assigned probe does not have the 'probe_cycle_complete' signal!"
			)
	else:
		push_warning("FakeWorld: No AmortizedProbe assigned!")


# Add this to FakeWorld.gd
func _process(_delta):
	if probe != null:
		# The probe freezes its position while capturing.
		# The FakeWorld MUST freeze with it to maintain the holodeck illusion.
		# Use the Rust method to update position
		probe.update_fake_world_position()


func _on_probe_cycle_complete(faces: Array[Image]):
	var faces_array = faces

	if faces_array.size() == 6:
		var typed_faces: Array[Image] = []
		for i in range(6):
			var img = faces_array[i] as Image
			if img == null or img.is_empty():
				push_warning("FakeWorld: Face image %d is null or empty" % i)
				return

			# Ensure all images are identical in format. Godot's create_from_images requires this.
			# Subviewports often capture with mipmaps disabled and different formats.
			if (
				i > 0
				and (
					img.get_width() != typed_faces[0].get_width()
					or img.get_format() != typed_faces[0].get_format()
				)
			):
				img.convert(typed_faces[0].get_format())
				img.resize(typed_faces[0].get_width(), typed_faces[0].get_height())

			typed_faces.append(img)

		# Godot 4 expects exactly an Array[Image] for Cubemap creation
		var img_array = Image.create_empty(
			typed_faces[0].get_width(),
			typed_faces[0].get_height(),
			false,
			typed_faces[0].get_format()
		)

		var err = _cubemap.create_from_images(typed_faces)
		if err == OK:
			_material.set_shader_parameter("env_cubemap", _cubemap)
		else:
			push_error("FakeWorld: Failed to create cubemap, error code: ", err)


# Update the palette texture dynamically at runtime
func set_palette(palette_texture: Texture2D):
	_material.set_shader_parameter("palette", palette_texture)


# Generate a 16-color palette from a single image for posterization
func generate_palette_from_image(source_image: Image) -> Texture2D:
	# Create a 16x1 texture where each row contains a palette color
	var palette_image = Image.create(16, 1, false, Image.FORMAT_RGBA8)

	# Sample colors from the source image and quantize to find distinct palette colors
	var sampled_colors = []
	var color_count = 0

	# Sample multiple points to find representative colors
	for y in range(source_image.get_height()):
		for x in range(source_image.get_width()):
			var color = source_image.get_pixel(x, y)
			if color_count < 16:
				# Quantize to reduce color variations
				var r = int(color.r * 15.0) / 15.0
				var g = int(color.g * 15.0) / 15.0
				var b = int(color.b * 15.0) / 15.0
				var quantized_color = Color(r, g, b, 1.0)

				var already_added = false
				for i in range(color_count):
					var existing = sampled_colors[i]
					if existing.distance_to(quantized_color) < 0.05:
						already_added = true
						break

				if not already_added:
					sampled_colors.append(quantized_color)
					color_count += 1
					if color_count >= 16:
						break
			if color_count >= 16:
				break
		if color_count >= 16:
			break

# Fill the palette image with the sampled colors
	for i in range(16):
		if i < color_count:
			var color = sampled_colors[i]
			palette_image.set_pixel(i, 0, color)  # Swapped to (x=i, y=0)
		else:
			# Default fallback color
			palette_image.set_pixel(i, 0, Color(1.0, 1.0, 1.0, 1.0))  # Swapped to (x=i, y=0)
	palette_image.mipmaps = false
	palette_image.generate_mipmaps()

	return palette_image.get_texture()
