#version 450

layout(set = 1, binding = 1) uniform sampler2D atlas;

layout(location = 0) in smooth vec2 fragTexCoord;  
layout(location = 1) in vec4 inColor;
layout(location = 2) in vec2 inUv;

layout(location = 0) out vec4 outColor;

const float smoothing = 1.0/8.0;

void main()
{
    float sdf = texture(atlas, fragTexCoord).r;
    float aaf = fwidth(sdf);
    // vec2 halfPixel = vec2(0.5 / 128.0);
    // float aaf = abs(dFdx(fragTexCoord.x) + dFdy(fragTexCoord.y));
    float opacity = smoothstep(0.5 - aaf, 0.5 + aaf, -sdf + 1.0);

    outColor = vec4(vec3(1.0), opacity);
    // outColor = vec4(inUv, 0.0, 1.0);
}

float median(float r, float g, float b) {
    return max(min(r, g), min(max(r, g), b));
}