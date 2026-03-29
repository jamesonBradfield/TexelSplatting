shader_type spatial;
render_mode unshaded;

uniform samplerCube env_cubemap : filter_nearest, source_color;
uniform sampler2D palette : filter_nearest, source_color, hint_default_black;
uniform float dither_strength : hint_range(0.0, 0.5) = 0.02; // Tweak this in the inspector!

varying vec3 local_pos;

// Smoother 8x8 Bayer Matrix for low resolutions
const float bayer_matrix_8x8[64] = float[](
    0.0/64.0, 48.0/64.0, 12.0/64.0, 60.0/64.0,  3.0/64.0, 51.0/64.0, 15.0/64.0, 63.0/64.0,
   32.0/64.0, 16.0/64.0, 44.0/64.0, 28.0/64.0, 35.0/64.0, 19.0/64.0, 47.0/64.0, 31.0/64.0,
    8.0/64.0, 56.0/64.0,  4.0/64.0, 52.0/64.0, 11.0/64.0, 59.0/64.0,  7.0/64.0, 55.0/64.0,
   40.0/64.0, 24.0/64.0, 36.0/64.0, 20.0/64.0, 43.0/64.0, 27.0/64.0, 39.0/64.0, 23.0/64.0,
    2.0/64.0, 50.0/64.0, 14.0/64.0, 62.0/64.0,  1.0/64.0, 49.0/64.0, 13.0/64.0, 61.0/64.0,
   34.0/64.0, 18.0/64.0, 46.0/64.0, 30.0/64.0, 33.0/64.0, 17.0/64.0, 45.0/64.0, 29.0/64.0,
   10.0/64.0, 58.0/64.0,  6.0/64.0, 54.0/64.0,  9.0/64.0, 57.0/64.0,  5.0/64.0, 53.0/64.0,
   42.0/64.0, 26.0/64.0, 38.0/64.0, 22.0/64.0, 41.0/64.0, 25.0/64.0, 37.0/64.0, 21.0/64.0
);

void vertex() {
    local_pos = VERTEX;
}

float perceptual_color_distance(vec3 c1, vec3 c2) {
    vec3 diff = c1 - c2;
    
    // Calculate how different the brightness is (Luma)
    vec3 luma_weights = vec3(0.299, 0.587, 0.114); 
    float luma_dist = dot(diff * luma_weights, diff * luma_weights);
    
    // Calculate how different the raw colors are (Hue/Saturation)
    float raw_dist = dot(diff, diff); 
    
    // Blend them! 70% focus on brightness for detail, 30% focus on raw color to stop hue-flipping
    return (luma_dist * 0.7) + (raw_dist * 0.3); 
}

void fragment() {
    vec3 sample_dir = normalize(local_pos);
    vec4 base_color = texture(env_cubemap, sample_dir);
    
    // 1. Get the exact number of colors in whatever palette you assigned
    int palette_size = textureSize(palette, 0).x;
    
    // 2. Calculate Screen-Space Dithering using the 8x8 matrix
    int x = int(FRAGCOORD.x) % 8;
    int y = int(FRAGCOORD.y) % 8;
    float dither_value = bayer_matrix_8x8[y * 8 + x] - 0.5; 
    
    // Apply the dither offset to the base color
    vec3 dithered_color = clamp(base_color.rgb + (vec3(dither_value) * dither_strength), 0.0, 1.0);
    
    // 3. Find the closest palette color
    float diff_min = 999999.0;
    vec3 posterized_color = dithered_color; 
    
    // 4. Loop dynamically based on that size
    for (int i = 0; i < palette_size; i++) {
        // Sample horizontally across the exact center of each pixel
        vec3 palette_col = texture(palette, vec2((float(i) + 0.5) / float(palette_size), 0.5)).rgb;
        
        // Use the perceptual distance function against our newly dithered color
        float dist = perceptual_color_distance(dithered_color, palette_col);
        
        if (dist < diff_min) {
            diff_min = dist;
            posterized_color = palette_col;
        }
    }
    
    ALBEDO = posterized_color;
}
