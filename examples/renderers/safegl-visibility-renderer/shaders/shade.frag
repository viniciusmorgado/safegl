#version 460 core
uniform usampler2D visibility_texture;
uniform usamplerBuffer index_lookup;
uniform samplerBuffer attributes;
uniform sampler2D base_color_texture;
uniform uint triangle_base;
uniform uint triangle_count;
uniform mat4 model;
uniform mat4 view_projection;
uniform mat3 normal_matrix;
uniform vec4 base_color_factor;
uniform vec3 camera_position;
uniform vec3 light_direction;
uniform vec3 light_color;
out vec4 fragment_color;

float cross2(vec2 a, vec2 b) { return a.x*b.y - a.y*b.x; }

void main() {
    uint id = texelFetch(visibility_texture, ivec2(gl_FragCoord.xy), 0).r;
    uint encoded_triangle = id & 0x7fffffffu;
    if (encoded_triangle == 0u) discard;
    uint triangle = encoded_triangle - 1u;
    // One shading draw per material keeps ordinary OpenGL 3.3 texture bindings.
    if (triangle < triangle_base || triangle - triangle_base >= triangle_count) discard;
    int first_index = int((triangle - triangle_base) * 3u);
    vec3 positions[3], normals[3];
    vec2 uvs[3];
    vec4 clip[3];
    for (int i = 0; i < 3; ++i) {
        int vertex = int(texelFetch(index_lookup, first_index + i).r);
        vec4 p = texelFetch(attributes, vertex * 2);
        vec4 n = texelFetch(attributes, vertex * 2 + 1);
        positions[i] = p.xyz; normals[i] = n.xyz; uvs[i] = vec2(p.w, n.w);
        clip[i] = view_projection * model * vec4(p.xyz, 1.0);
    }
    vec3 inv_w = 1.0 / vec3(clip[0].w, clip[1].w, clip[2].w);
    vec2 p0 = clip[0].xy * inv_w.x;
    vec2 e1 = clip[1].xy * inv_w.y - p0;
    vec2 e2 = clip[2].xy * inv_w.z - p0;
    vec2 size = vec2(textureSize(visibility_texture, 0));
    vec2 offset = gl_FragCoord.xy * 2.0 / size - 1.0 - p0;
    float determinant = cross2(e1, e2);
    // Rasterized samples should never come from an exactly degenerate triangle.
    if (abs(determinant) < 1e-20) discard;
    vec3 lambda;
    lambda.y = cross2(offset, e2) / determinant;
    lambda.z = cross2(e1, offset) / determinant;
    lambda.x = 1.0 - lambda.y - lambda.z;
    vec3 q = lambda * inv_w;
    float denominator = q.x + q.y + q.z;
    vec3 barycentric = q / denominator;
    vec2 uv = uvs[0]*barycentric.x + uvs[1]*barycentric.y + uvs[2]*barycentric.z;

    // Analytical UV derivatives stay within this triangle even when adjacent
    // pixels belong to unrelated geometry, unlike implicit fullscreen derivatives.
    vec3 dqdx = vec3(e1.y - e2.y, e2.y, -e1.y) * (2.0 / size.x / determinant) * inv_w;
    vec3 dqdy = vec3(e2.x - e1.x, -e2.x, e1.x) * (2.0 / size.y / determinant) * inv_w;
    vec2 duvdx = (uvs[0]*dqdx.x + uvs[1]*dqdx.y + uvs[2]*dqdx.z - uv*(dqdx.x+dqdx.y+dqdx.z)) / denominator;
    vec2 duvdy = (uvs[0]*dqdy.x + uvs[1]*dqdy.y + uvs[2]*dqdy.z - uv*(dqdy.x+dqdy.y+dqdy.z)) / denominator;

    vec3 world_position = (model * vec4(positions[0]*barycentric.x + positions[1]*barycentric.y + positions[2]*barycentric.z, 1.0)).xyz;
    vec3 N = normalize(normal_matrix * (normals[0]*barycentric.x + normals[1]*barycentric.y + normals[2]*barycentric.z));
    if ((id & 0x80000000u) != 0u) N = -N;
    vec3 albedo = textureGrad(base_color_texture, uv, duvdx, duvdy).rgb * base_color_factor.rgb;
    vec3 L = normalize(light_direction);
    vec3 V = normalize(camera_position - world_position);
    vec3 H = normalize(L + V);
    float diffuse = max(dot(N, L), 0.0);
    float specular = diffuse > 0.0 ? pow(max(dot(N, H), 0.0), 32.0) : 0.0;
    fragment_color = vec4(0.14 * albedo + light_color * (0.80 * albedo * diffuse + 0.18 * specular), 1.0);
}
