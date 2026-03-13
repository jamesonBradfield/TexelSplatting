extends SceneTree

func _init():
    var vp = SubViewport.new()
    vp.size = Vector2i(64, 64)
    vp.render_target_update_mode = SubViewport.UPDATE_ONCE
    
    var cam = Camera3D.new()
    cam.scale = Vector3(-1, 1, 1)
    vp.add_child(cam)
    
    var root = Node.new()
    root.add_child(vp)
    
    print("Camera scale is: ", cam.scale)
    quit()
