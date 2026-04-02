shader_type spatial;
render_mode unshaded;

uniform samplerCube env_cubemap : filter_linear, source_color;

varying vec3 local_pos;

void vertex() {
    local_pos = VERTEX;
}

void fragment() {
    vec3 sample_dir = normalize(local_pos);
    vec4 base_color = texture(env_cubemap, sample_dir);
    ALBEDO = base_color.rgb;
}
