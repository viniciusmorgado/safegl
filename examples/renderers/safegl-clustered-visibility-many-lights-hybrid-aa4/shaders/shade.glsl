// Compiled three times; the renderer prepends `#version` and one SHADING_RATE:
//   SHADING_RATE_PIXEL    once per pixel at its centre (samples on one continuous surface)
//   SHADING_RATE_SAMPLE   once per sample via gl_SampleID (edges and alpha coverage)
//   SHADING_RATE_CLASSIFY no shading: keeps pixels whose specular lobe would alias
uniform usampler2DMS visibility_texture;
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
layout(std430) readonly buffer LightData { vec4 light_data[]; };
layout(std430) readonly buffer ClusterLists { uint entries[]; };
uniform mat4 view;
uniform uvec3 grid;
uniform vec2 near_far;
uniform bool all_lights;
uniform bool highlight_samples;
uniform float specular_threshold;
#ifndef SHADING_RATE_CLASSIFY
out vec4 fragment_color;
#endif

const uint TRIANGLE_MASK = 0x3fffffffu;
const uint BACK_FACING = 0x80000000u;

float cross2(vec2 a, vec2 b) { return a.x*b.y - a.y*b.x; }

void main() {
#ifdef SHADING_RATE_SAMPLE
    uint id = texelFetch(visibility_texture, ivec2(gl_FragCoord.xy), gl_SampleID).r;
    vec2 sample_offset = gl_SamplePosition;
#else
    // Classification left this pixel on one continuous surface: either all four
    // samples hold this ID, or they hold neighbours sharing a vertex with it. Sample
    // 0's triangle is evaluated at the pixel centre, extrapolating at most a fraction
    // of a pixel across a shared, attribute-continuous edge.
    uint id = texelFetch(visibility_texture, ivec2(gl_FragCoord.xy), 0).r;
    vec2 sample_offset = vec2(0.5);
#endif
    uint encoded_triangle = id & TRIANGLE_MASK;
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
    vec2 size = vec2(textureSize(visibility_texture));
    vec2 sample_pixel = floor(gl_FragCoord.xy) + sample_offset;
    vec2 offset = sample_pixel * 2.0 / size - 1.0 - p0;
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
    vec3 dqdx = vec3(e1.y - e2.y, e2.y, -e1.y) * (2.0 / size.x / determinant) * inv_w;
    vec3 dqdy = vec3(e2.x - e1.x, -e2.x, e1.x) * (2.0 / size.y / determinant) * inv_w;
    vec3 normal = normal_matrix * (normals[0]*barycentric.x + normals[1]*barycentric.y + normals[2]*barycentric.z);

#ifdef SHADING_RATE_CLASSIFY
    // Screen-space derivatives of the perspective-correct barycentrics, then of the
    // unit normal. Their length is how far (in radians) the normal turns per pixel.
    vec3 dbdx = (dqdx - barycentric * (dqdx.x + dqdx.y + dqdx.z)) / denominator;
    vec3 dbdy = (dqdy - barycentric * (dqdy.x + dqdy.y + dqdy.z)) / denominator;
    vec3 dndx = normal_matrix * (normals[0]*dbdx.x + normals[1]*dbdx.y + normals[2]*dbdx.z);
    vec3 dndy = normal_matrix * (normals[0]*dbdy.x + normals[1]*dbdy.y + normals[2]*dbdy.z);
    float length_n = max(length(normal), 1e-8);
    vec3 n = normal / length_n;
    dndx = (dndx - n * dot(n, dndx)) / length_n;
    dndy = (dndy - n * dot(n, dndy)) / length_n;
    float turn_per_pixel = max(length(dndx), length(dndy));
    // Keep (and stencil-mark) only pixels that need supersampling.
    if (turn_per_pixel < specular_threshold) discard;
#else
    vec2 uv = uvs[0]*barycentric.x + uvs[1]*barycentric.y + uvs[2]*barycentric.z;

    // Analytical UV derivatives stay within this triangle even when adjacent
    // pixels belong to unrelated geometry, unlike implicit fullscreen derivatives.
    vec2 duvdx = (uvs[0]*dqdx.x + uvs[1]*dqdx.y + uvs[2]*dqdx.z - uv*(dqdx.x+dqdx.y+dqdx.z)) / denominator;
    vec2 duvdy = (uvs[0]*dqdy.x + uvs[1]*dqdy.y + uvs[2]*dqdy.z - uv*(dqdy.x+dqdy.y+dqdy.z)) / denominator;

    vec3 world_position = (model * vec4(positions[0]*barycentric.x + positions[1]*barycentric.y + positions[2]*barycentric.z, 1.0)).xyz;
    vec3 N = normalize(normal);
    if ((id & BACK_FACING) != 0u) N = -N;
    vec3 albedo = textureGrad(base_color_texture, uv, duvdx, duvdy).rgb * base_color_factor.rgb;
    vec3 V = normalize(camera_position - world_position);
    vec3 color = 0.06 * albedo;
    vec3 fill = normalize(vec3(0.5, 0.8, 1.0));
    color += 0.18 * albedo * max(dot(N, fill), 0.0);
    float view_depth = -(view * vec4(world_position,1)).z;
    uint slice = uint(clamp(floor(log(max(view_depth, near_far.x)/near_far.x) /
        log(near_far.y/near_far.x) * float(grid.z)), 0.0, float(grid.z-1u)));
    uvec2 tile = uvec2(gl_FragCoord.xy) / 16u;
    uint base = ((slice * grid.y + tile.y) * grid.x + tile.x) * 257u;
    uint count = all_lights ? 256u : entries[base];
    for (uint j=0u; j<count; ++j) {
        uint index = all_lights ? j : entries[base+1u+j];
        vec4 light = light_data[index*2u];
        vec4 emission = light_data[index*2u+1u];
        vec3 delta = light.xyz - world_position;
        float distance_squared = dot(delta,delta);
        if (distance_squared >= light.w*light.w) continue;
        vec3 L = delta * inversesqrt(max(distance_squared,1e-8));
        float diffuse = max(dot(N,L),0.0);
        vec3 H = normalize(L+V);
        float specular = diffuse > 0.0 ? pow(max(dot(N,H),0.0),32.0) : 0.0;
        float fade = max(1.0-distance_squared/(light.w*light.w),0.0);
        float attenuation = fade*fade/(1.0+12.0*distance_squared);
        color += emission.rgb*emission.a*attenuation*(0.80*albedo*diffuse+0.18*specular);
    }
#ifdef SHADING_RATE_SAMPLE
    // Classification overlay: tint the samples that paid for supersampling.
    if (highlight_samples) color = mix(color, vec3(1.0, 0.05, 0.6), 0.55);
#endif
    fragment_color = vec4(color,1.0);
#endif
}
