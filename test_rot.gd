extends SceneTree

func _init():
    var z_fwd = Vector3(0, 0, -1)
    
    var dirs = [
        Vector3(0, -90, 0),  # +X?
        Vector3(0, 90, 0),   # -X?
        Vector3(90, 0, 0),   # +Y?
        Vector3(-90, 0, 0),  # -Y?
        Vector3(0, 180, 0),  # +Z?
        Vector3(0, 0, 0)     # -Z?
    ]
    
    for d in dirs:
        var basis = Basis.from_euler(d * PI / 180.0)
        # Default camera looks down -Z, so multiply basis by -Z to get actual look direction
        var look_dir = basis * Vector3(0, 0, -1)
        var up_dir = basis * Vector3(0, 1, 0)
        print("Rot ", d, " -> Look: ", look_dir.round(), " Up: ", up_dir.round())
        
    quit()
