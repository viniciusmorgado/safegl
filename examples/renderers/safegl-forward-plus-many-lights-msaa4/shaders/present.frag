#version 460 core
uniform sampler2DMS color_texture;
out vec4 color;
void main() {
    ivec2 pixel = ivec2(gl_FragCoord.xy);
    vec3 linear_color = vec3(0.0);
    int samples = textureSamples(color_texture);
    for (int i = 0; i < samples; ++i) {
        linear_color += texelFetch(color_texture, pixel, i).rgb;
    }
    color = vec4(linear_color / float(samples), 1.0);
    // Average linear HDR samples first; framebuffer sRGB encoding happens once.
}
