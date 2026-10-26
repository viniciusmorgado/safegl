#version 460 core
uniform sampler2D color_texture;
out vec4 color;
void main() { color = texelFetch(color_texture, ivec2(gl_FragCoord.xy), 0); }
