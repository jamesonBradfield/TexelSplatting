shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_opaque, skip_vertex_transform;

uniform samplerCube env_cubemap : filter_nearest, source_color;
uniform samplerCube depth_cubemap : filter_nearest;
uniform int cubemap_resolution = 64;
uniform float quad_size = 1.0;

varying vec3 v_color;

void vertex() {
    int res = cubemap_resolution;
    int face = INSTANCE_ID / (res * res);
    int texel_index = INSTANCE_ID % (res * res);
    
    int u_idx = texel_index % res;
    int v_idx = texel_index / res;
    
    // Map to [-1, 1]
    float u = (float(u_idx) + 0.5) / float(res) * 2.0 - 1.0;
    float v = (float(v_idx) + 0.5) / float(res) * 2.0 - 1.0;
    
    vec3 dir;
    // Cubemap face mapping (OpenGL standard)
    if (face == 0)      { dir = vec3( 1.0,   -v,   -u); } // +X
    else if (face == 1) { dir = vec3(-1.0,   -v,    u); } // -X
    else if (face == 2) { dir = vec3(   u,  1.0,    v); } // +Y
    else if (face == 3) { dir = vec3(   u, -1.0,   -v); } // -Y
    else if (face == 4) { dir = vec3(   u,   -v,  1.0); } // +Z
    else                { dir = vec3(  -u,   -v, -1.0); } // -Z
    
    // We need the unnormalized direction for the Chebyshev distance formula
    vec3 unnormalized_dir = dir;
    dir = normalize(dir);
    
    // Sample color and depth from cubemaps
    v_color = texture(env_cubemap, dir).rgb;
    float raw_depth = texture(depth_cubemap, dir).r;
    
    // The depth captured by the camera is non-linear (z-buffer depth).
    // We need to linearize it to get the actual distance from the probe.
    // For Vulkan [0, 1] depth buffer:
    float near = 0.05;
    float far = 4000.0;
    float linear_depth = near * far / (far - raw_depth * (far - near));
    
    // The paper says: p = o + d * ||r||_inf * r
    // Where d is the Chebyshev distance (which is the linear depth along the principal axis).
    // Since our unnormalized_dir has a max component of 1.0, ||r||_inf is 1.0.
    // So p = o + linear_depth * unnormalized_dir
    
    // The probe origin 'o' is the position of the ProjectionProbe node (which should be at the same place as the RealtimeProbe).
    // In the vertex shader, VERTEX is initially the local quad vertex.
    // We want to position the quad at `linear_depth * unnormalized_dir` in local space.
    vec3 center_pos = linear_depth * unnormalized_dir;
    
    // Calculate texel size based on depth and resolution
    // A 90-degree FOV face has width 2.0 at depth 1.0.
    float texel_size = (2.0 / float(res)) * linear_depth;
    
    // Billboard the quad to face the camera
    // We can use the inverse view matrix to get the camera's right and up vectors
    vec3 cam_right = INV_VIEW_MATRIX[0].xyz;
    vec3 cam_up = INV_VIEW_MATRIX[1].xyz;
    
    // VERTEX.xy contains the quad's local coordinates (-0.5 to 0.5 if size is 1.0)
    vec3 vertex_offset = (cam_right * VERTEX.x + cam_up * VERTEX.y) * texel_size * quad_size;
    
    // Final local position
    vec3 final_local_pos = center_pos + vertex_offset;
    
    // Transform to view space for Godot's standard pipeline
    VERTEX = (MODELVIEW_MATRIX * vec4(final_local_pos, 1.0)).xyz;
}

void fragment() {
    ALBEDO = v_color;
}
