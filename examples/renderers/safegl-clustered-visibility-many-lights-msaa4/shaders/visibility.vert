#version 460 core
uniform samplerBuffer attributes;
uniform mat4 model;
uniform mat4 view_projection;
centroid out vec2 uv;
void main() {
    vec4 position_uv = texelFetch(attributes, gl_VertexID * 2);
    vec4 normal_uv = texelFetch(attributes, gl_VertexID * 2 + 1);
    uv = vec2(position_uv.w, normal_uv.w);
    vec3 position = texelFetch(attributes, gl_VertexID * 2).xyz;
    gl_Position = view_projection * model * vec4(position, 1.0);
}
