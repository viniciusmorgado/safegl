#version 330 core
uniform uint triangle_base;
out uint visibility_id;
in vec2 uv;
uniform sampler2D base_color_texture;
uniform vec4 base_color_factor;
uniform int alpha_mode;
uniform float alpha_cutoff;

void apply_coverage() {
    if (alpha_mode == 0) return;
    float alpha = texture(base_color_texture, uv).a * base_color_factor.a;
    if (alpha_mode == 1) {
        if (alpha < alpha_cutoff) discard;
    } else {
        // Ordered screen-door coverage approximates BLEND in every pipeline.
        const int bayer[16] = int[16](0, 8, 2, 10, 12, 4, 14, 6,
                                     3, 11, 1, 9, 15, 7, 13, 5);
        ivec2 pixel = ivec2(gl_FragCoord.xy) % 4;
        float threshold = (float(bayer[pixel.y * 4 + pixel.x]) + 0.5) / 16.0;
        if (alpha <= threshold) discard;
    }
}

void main() {
    apply_coverage();
    // Zero is background; the high bit records the original triangle facing.
    visibility_id = triangle_base + uint(gl_PrimitiveID) + 1u;
    if (!gl_FrontFacing) visibility_id |= 0x80000000u;
}
