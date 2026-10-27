#version 460

layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 color;

layout(location = 1) uniform vec4 tint;
layout(location = 2, binding = 0) uniform sampler2D tex;
layout(binding = 1, std140) uniform Factors {
    float scale;
} factors;

// changed by specialization
layout(constant_id = 7) const float brightness = 1.0;

void main() {
    color = texture(tex, v_uv) * tint * factors.scale * brightness;
}
