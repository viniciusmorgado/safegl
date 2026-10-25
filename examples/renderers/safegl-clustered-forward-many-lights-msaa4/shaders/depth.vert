#version 460 core
in vec3 position;
in vec2 tex_coord;
centroid out vec2 uv;
uniform mat4 model;
uniform mat4 view_projection;
invariant gl_Position;
void main() { uv = tex_coord; gl_Position = view_projection * (model * vec4(position, 1.0)); }
