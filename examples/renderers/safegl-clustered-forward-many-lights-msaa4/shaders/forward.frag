#version 460 core
centroid in vec3 world_position;
centroid in vec3 world_normal;
centroid in vec2 uv;
layout(std430) readonly buffer LightData { vec4 light_data[]; };
layout(std430) readonly buffer ClusterLists { uint entries[]; };
uniform sampler2D base_color_texture;
uniform vec4 base_color_factor;
uniform vec3 camera_position;
uniform mat4 view;
uniform uvec3 grid;
uniform vec2 near_far;
uniform bool all_lights;
out vec4 fragment_color;

uniform int alpha_mode;
uniform float alpha_cutoff;

void apply_coverage() {
    if (alpha_mode == 0) return;
    float alpha = texture(base_color_texture, uv).a * base_color_factor.a;
    if (alpha_mode == 1) {
        if (alpha < alpha_cutoff) discard;
    } else {
        // Ordered screen-door coverage approximates BLEND in every pipeline.
        const int bayer[16] = int[16](0, 8, 2, 10, 12, 4, 14, 6,
                                     3, 11, 1, 9, 15, 7, 13, 5);
        ivec2 pixel = ivec2(gl_FragCoord.xy) % 4;
        float threshold = (float(bayer[pixel.y * 4 + pixel.x]) + 0.5) / 16.0;
        if (alpha <= threshold) discard;
    }
}

void main() {
    apply_coverage();
    vec3 albedo = texture(base_color_texture, uv).rgb * base_color_factor.rgb;
    vec3 N = normalize(world_normal);
    if (!gl_FrontFacing) N = -N;
    vec3 V = normalize(camera_position - world_position);
    vec3 color = 0.06 * albedo;
    // A weak, fixed directional fill helps retain the shape between local lights.
    vec3 fill = normalize(vec3(0.5, 0.8, 1.0));
    color += 0.18 * albedo * max(dot(N, fill), 0.0);
    float view_depth = -(view * vec4(world_position,1)).z;
    uint slice = uint(clamp(floor(log(max(view_depth, near_far.x)/near_far.x) /
        log(near_far.y/near_far.x) * float(grid.z)), 0.0, float(grid.z-1u)));
    uvec2 tile = uvec2(gl_FragCoord.xy) / 16u;
    uint base = ((slice * grid.y + tile.y) * grid.x + tile.x) * 257u;
    uint count = all_lights ? 256u : entries[base];
    for (uint j = 0u; j < count; ++j) {
        uint index = all_lights ? j : entries[base + 1u + j];
        vec4 light = light_data[index * 2u];
        vec4 emission = light_data[index * 2u + 1u];
        vec3 delta = light.xyz - world_position;
        float distance_squared = dot(delta, delta);
        if (distance_squared >= light.w * light.w) continue;
        vec3 L = delta * inversesqrt(max(distance_squared, 1e-8));
        float diffuse = max(dot(N, L), 0.0);
        vec3 H = normalize(L + V);
        float specular = diffuse > 0.0 ? pow(max(dot(N, H), 0.0), 32.0) : 0.0;
        float fade = max(1.0 - distance_squared / (light.w * light.w), 0.0);
        float attenuation = fade * fade / (1.0 + 12.0 * distance_squared);
        color += emission.rgb * emission.a * attenuation * (0.80 * albedo * diffuse + 0.18 * specular);
    }
    fragment_color = vec4(color, 1.0);
}
