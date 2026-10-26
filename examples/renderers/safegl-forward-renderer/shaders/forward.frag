#version 330 core

in vec3 world_position;
in vec3 world_normal;
in vec2 uv;

uniform sampler2D base_color_texture;
uniform vec4 base_color_factor;
uniform vec3 camera_position;
uniform vec3 light_direction;
uniform vec3 light_color;

out vec4 fragment_color;

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
    // SrgbTexture2d decodes the base color into linear RGB when sampled.
    vec3 albedo = texture(base_color_texture, uv).rgb * base_color_factor.rgb;
    vec3 N = normalize(world_normal);
    if (!gl_FrontFacing) N = -N;
    vec3 L = normalize(light_direction); // Direction from surface toward the light.
    vec3 V = normalize(camera_position - world_position);
    vec3 H = normalize(L + V);

    float diffuse = max(dot(N, L), 0.0);
    float specular = diffuse > 0.0 ? pow(max(dot(N, H), 0.0), 32.0) : 0.0;
    // Classical Blinn-Phong: ambient approximation + one light's diffuse/specular.
    vec3 ambient = 0.14 * albedo;
    vec3 direct = light_color * (0.80 * albedo * diffuse + 0.18 * specular);
    fragment_color = vec4(ambient + direct, 1.0);
    // SafeGL enables framebuffer sRGB conversion; do not gamma-correct twice.
}
