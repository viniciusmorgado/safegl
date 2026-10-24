#version 330 core

in vec3 world_position;
in vec3 world_normal;
in vec2 uv;

uniform sampler2D base_color_texture;
uniform vec4 base_color_factor;

out vec4 g_position;
out vec4 g_normal_specular;
out vec4 g_albedo;

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
    vec3 N = normalize(world_normal);
    if (!gl_FrontFacing) N = -N;
    g_position = vec4(world_position, 1.0);
    g_normal_specular = vec4(N, 0.18);
    // Source sRGB textures decode on sampling; the G-buffer stores linear RGB.
    g_albedo = vec4(texture(base_color_texture, uv).rgb * base_color_factor.rgb, 1.0);
}
