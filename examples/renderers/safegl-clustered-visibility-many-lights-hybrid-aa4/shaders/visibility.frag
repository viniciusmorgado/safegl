// The renderer prepends `#version`; SAMPLE_COVERAGE selects the variant for
// alpha-tested and screen-door materials, which runs once per sample.
uniform uint triangle_base;
out uint visibility_id;
#ifdef SAMPLE_COVERAGE
// `sample` interpolation forces per-sample execution, so each sample gets its own
// alpha test: transparency supersampling on top of the MSAA coverage.
sample in vec2 uv;
#else
centroid in vec2 uv;
#endif
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
    // Zero is background; bit 31 records the original triangle facing and bit 30
    // marks alpha-coverage materials, which classification always supersamples.
    visibility_id = triangle_base + uint(gl_PrimitiveID) + 1u;
    if (alpha_mode != 0) visibility_id |= 0x40000000u;
    if (!gl_FrontFacing) visibility_id |= 0x80000000u;
}
