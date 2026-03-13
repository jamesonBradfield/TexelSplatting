extends Node3D


func _ready():
	var layer = 2
	var render_mgr = get_node_or_null("/root/RenderManager")
	if render_mgr and render_mgr.settings:
		layer = render_mgr.settings.real_world_mask

	var create_axis = func(color: Color, size: Vector3, pos: Vector3):
		var mesh_inst = MeshInstance3D.new()
		var box = BoxMesh.new()
		box.size = size
		mesh_inst.mesh = box

		var mat = StandardMaterial3D.new()
		mat.albedo_color = color
		mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		mesh_inst.material_override = mat
		mesh_inst.position = pos
		mesh_inst.layers = layer

		add_child(mesh_inst)

	# +X: Right (Red)
	create_axis.call(Color(1.0, 0.0, 0.0), Vector3(1.0, 0.1, 0.1), Vector3(0.5, 0.0, 0.0))
	# -X: Left (Cyan)
	create_axis.call(Color(0.0, 1.0, 1.0), Vector3(1.0, 0.1, 0.1), Vector3(-0.5, 0.0, 0.0))

	# +Y: Top (Green)
	create_axis.call(Color(0.0, 1.0, 0.0), Vector3(0.1, 1.0, 0.1), Vector3(0.0, 0.5, 0.0))
	# -Y: Bottom (Magenta)
	create_axis.call(Color(1.0, 0.0, 1.0), Vector3(0.1, 1.0, 0.1), Vector3(0.0, -0.5, 0.0))

	# +Z: Back (Blue)
	create_axis.call(Color(0.0, 0.0, 1.0), Vector3(0.1, 0.1, 1.0), Vector3(0.0, 0.0, 0.5))
	# -Z: Forward (Yellow)
	create_axis.call(Color(1.0, 1.0, 0.0), Vector3(0.1, 0.1, 1.0), Vector3(0.0, 0.0, -0.5))

	# The Asymmetry Block: Placed Right (+X), Up (+Y), and Forward (-Z)
	create_axis.call(Color.WHITE, Vector3(0.2, 0.2, 0.2), Vector3(1.0, 1.0, -1.0))
