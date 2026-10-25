#version 330 core
uniform sampler2D resolved_color;
out vec4 fragment_color;
void main() {
    fragment_color = vec4(texelFetch(resolved_color, ivec2(gl_FragCoord.xy), 0).rgb, 1.0);
    // Program outputs_srgb=false enables final linear-to-sRGB encoding.
}
