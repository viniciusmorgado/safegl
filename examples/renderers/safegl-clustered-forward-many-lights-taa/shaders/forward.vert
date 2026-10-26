#version 460 core

in vec3 position;
in vec3 normal;
in vec2 tex_coord;

uniform mat4 model;
uniform mat4 view_projection;
uniform mat3 normal_matrix;

out vec3 world_position;
out vec3 world_normal;
out vec2 uv;

invariant gl_Position;

void main() {
    vec4 world = model * vec4(position, 1.0);
    world_position = world.xyz;
    world_normal = normal_matrix * normal;
    uv = tex_coord;
    gl_Position = view_projection * world;
}
