#version 460

layout(location = 0) in vec2 position;
layout(location = 0, xfb_buffer = 0, xfb_stride = 16, xfb_offset = 0) out vec4 captured;

void main() {
    captured = vec4(position, 1.0, 2.0);
    gl_Position = vec4(position, 0.0, 1.0);
}
