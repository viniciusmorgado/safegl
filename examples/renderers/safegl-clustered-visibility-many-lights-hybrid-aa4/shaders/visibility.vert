// The renderer prepends `#version`; see visibility.frag for SAMPLE_COVERAGE.
uniform samplerBuffer attributes;
uniform mat4 model;
uniform mat4 view_projection;
#ifdef SAMPLE_COVERAGE
sample out vec2 uv;
#else
centroid out vec2 uv;
#endif
void main() {
    vec4 position_uv = texelFetch(attributes, gl_VertexID * 2);
    vec4 normal_uv = texelFetch(attributes, gl_VertexID * 2 + 1);
    uv = vec2(position_uv.w, normal_uv.w);
    vec3 position = texelFetch(attributes, gl_VertexID * 2).xyz;
    gl_Position = view_projection * model * vec4(position, 1.0);
}
