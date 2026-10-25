#version 330 core

uniform sampler2D position_texture;
uniform sampler2D normal_specular_texture;
uniform sampler2D albedo_texture;
uniform vec3 camera_position;
uniform vec3 light_direction;
uniform vec3 light_color;

out vec4 fragment_color;

void main() {
    ivec2 pixel = ivec2(gl_FragCoord.xy);
    vec4 position = texelFetch(position_texture, pixel, 0);
    if (position.a == 0.0) {
        fragment_color = vec4(0.018, 0.024, 0.037, 1.0);
        return;
    }
    vec4 normal_specular = texelFetch(normal_specular_texture, pixel, 0);
    vec3 N = normalize(normal_specular.xyz);
    vec3 albedo = texelFetch(albedo_texture, pixel, 0).rgb;
    vec3 L = normalize(light_direction);
    vec3 V = normalize(camera_position - position.xyz);
    vec3 H = normalize(L + V);
    float diffuse = max(dot(N, L), 0.0);
    float specular = diffuse > 0.0 ? pow(max(dot(N, H), 0.0), 32.0) : 0.0;
    vec3 ambient = 0.14 * albedo;
    vec3 direct = light_color * (0.80 * albedo * diffuse + normal_specular.a * specular);
    fragment_color = vec4(ambient + direct, 1.0);
    // SafeGL performs sRGB conversion only on the final sRGB framebuffer.
}
