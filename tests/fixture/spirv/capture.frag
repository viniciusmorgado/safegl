#version 460

layout(location = 0) in vec4 captured;
layout(location = 0) out vec4 color;

void main() {
    color = captured;
}
