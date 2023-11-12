#version 450

layout(set = 1, binding = 1) uniform sampler2D atlas;

layout(location = 0) in vec2 fragTexCoord;  
layout(location = 1) in vec4 inColor;

layout(location = 0) out vec4 outColor;

const float smoothing = 1.0/8.0;

void main()
{
    float sdf = texture(atlas, fragTexCoord).r;

    float opacity = smoothstep(0.5 + smoothing, 0.5 - smoothing, sdf);

    outColor = vec4(vec3(1.0), opacity);  
}

float median(float a, float b, float c) {
    if ((a <= b) && (b <= c)) return b;  // a b c
    if ((a <= c) && (c <= b)) return c;  // a c b
    if ((b <= a) && (a <= c)) return a;  // b a c
    if ((b <= c) && (c <= a)) return c;  // b c a
    if ((c <= a) && (a <= b)) return a;  // c a b
    return b;     
}