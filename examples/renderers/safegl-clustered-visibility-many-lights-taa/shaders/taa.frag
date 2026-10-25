#version 330 core

uniform sampler2D current_color;
uniform sampler2D scene_depth;
uniform mat4 inverse_current_vp;
uniform sampler2D history_color_depth;
uniform mat4 current_vp;
uniform mat4 previous_vp;
uniform bool history_valid;
uniform float history_weight;
out vec4 resolved;

vec3 to_ycocg(vec3 c) {
    return vec3(0.25*c.r + 0.5*c.g + 0.25*c.b,
                0.5*c.r - 0.5*c.b, -0.25*c.r + 0.5*c.g - 0.25*c.b);
}
vec3 to_rgb(vec3 c) {
    return vec3(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

void main() {
    ivec2 size = textureSize(current_color, 0);
    ivec2 pixel = ivec2(gl_FragCoord.xy);
    vec2 uv = gl_FragCoord.xy / vec2(size);
    vec3 current = texelFetch(current_color, pixel, 0).rgb;
    vec3 lo = vec3(1e10), hi = vec3(-1e10), mean = vec3(0), moment = vec3(0);
    vec4 closest = vec4(0);
    ivec2 closest_pixel = pixel;
    float current_depth = 1e10;
    // Foreground dilation provides motion/depth at silhouettes and thin hair.
    for (int y = -1; y <= 1; ++y) {
        for (int x = -1; x <= 1; ++x) {
            ivec2 p = clamp(pixel + ivec2(x, y), ivec2(0), size - 1);
            vec3 color = to_ycocg(texelFetch(current_color, p, 0).rgb);
            lo = min(lo, color); hi = max(hi, color);
            mean += color; moment += color * color;
            float device_depth = texelFetch(scene_depth, p, 0).r;
            vec4 world = inverse_current_vp * vec4((vec2(p) + 0.5) / vec2(size) * 2.0 - 1.0,
                                                  device_depth * 2.0 - 1.0, 1.0);
            // Reconstruct transient positions from depth, without an attribute G-buffer.
            vec4 position = device_depth < 1.0 ? vec4(world.xyz / world.w, 1.0) : vec4(0.0);
            float depth = (current_vp * vec4(position.xyz, 1.0)).w;
            if (position.a > 0.0 && depth > 0.0 && depth < current_depth) {
                closest = position; current_depth = depth; closest_pixel = p;
            }
        }
    }
    if (closest.a == 0.0) current_depth = 0.0;
    resolved = vec4(current, current_depth);
    if (!history_valid) return;

    vec2 previous_uv = uv;
    float expected_depth = 0.0;
    if (closest.a > 0.0) {
        vec4 previous_clip = previous_vp * vec4(closest.xyz, 1.0);
        if (previous_clip.w <= 0.0) return;
        expected_depth = previous_clip.w;
        vec2 reprojected = previous_clip.xy / previous_clip.w * 0.5 + 0.5;
        vec2 source_uv = (vec2(closest_pixel) + 0.5) / vec2(size);
        previous_uv += reprojected - source_uv;
    }
    if (any(lessThan(previous_uv, vec2(0))) || any(greaterThanEqual(previous_uv, vec2(1)))) return;

    // Bilinear reconstruction with depth validation per tap avoids blending
    // unrelated foreground/background history at disocclusions.
    vec2 sample_pixel = previous_uv * vec2(size) - 0.5;
    ivec2 origin = ivec2(floor(sample_pixel));
    vec2 fraction = fract(sample_pixel);
    vec3 history = vec3(0);
    float valid_weight = 0.0;
    for (int y = 0; y < 2; ++y) {
        for (int x = 0; x < 2; ++x) {
            ivec2 p = origin + ivec2(x, y);
            if (any(lessThan(p, ivec2(0))) || any(greaterThanEqual(p, size))) continue;
            vec4 old = texelFetch(history_color_depth, p, 0);
            bool valid = expected_depth == 0.0 ? old.a == 0.0 :
                old.a > 0.0 && abs(old.a - expected_depth) < max(0.015, expected_depth * 0.01);
            if (!valid) continue;
            float weight = (x == 0 ? 1.0-fraction.x : fraction.x) *
                           (y == 0 ? 1.0-fraction.y : fraction.y);
            history += old.rgb * weight;
            valid_weight += weight;
        }
    }
    if (valid_weight < 0.1) return;
    history /= valid_weight;

    mean /= 9.0;
    vec3 deviation = sqrt(max(moment / 9.0 - mean * mean, vec3(0)));
    lo = max(lo, mean - 1.25 * deviation);
    hi = min(hi, mean + 1.25 * deviation);
    history = to_rgb(clamp(to_ycocg(history), lo, hi));
    float motion = length((previous_uv - uv) * vec2(size));
    float weight = min(history_weight, mix(0.9, 0.65, clamp(motion / 16.0, 0.0, 1.0)));
    resolved.rgb = mix(current, history, weight * min(valid_weight, 1.0));
}
