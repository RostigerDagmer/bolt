#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec3 inNormal;
layout (location = 1) in vec4 inColor;
layout (location = 0) out vec4 outColor;

void main() {
    float light = clamp(dot(normalize(inNormal), vec3(0,0,1)),0,1);
    outColor = light * inColor;
}
