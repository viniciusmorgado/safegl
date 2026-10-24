#version 460 core
uniform sampler2D source_color;
out vec4 color;
void main() {
    ivec2 p = ivec2(gl_FragCoord.xy) * 2;
    vec3 sum = texelFetch(source_color,p,0).rgb + texelFetch(source_color,p+ivec2(1,0),0).rgb
             + texelFetch(source_color,p+ivec2(0,1),0).rgb + texelFetch(source_color,p+ivec2(1,1),0).rgb;
    color = vec4(sum * 0.25, 1.0);
}
