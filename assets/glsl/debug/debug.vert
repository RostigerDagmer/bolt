#version 460
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout(set = 0, binding = 0) uniform Scene {
    mat4 mvp;
    mat4 normal;
    mat4 model;
    mat4 view;
    mat4 projection;
} scene;

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 inColor;
layout (location = 2) in vec2 inUv;

layout (location = 3) in mat4 instanceTransform;

layout (location = 0) out vec3 outNormal;
layout (location = 1) out vec4 outColor;

void main() {
    outColor = inColor;
    outNormal = mat3(scene.normal) * vec3(0, 1, 0);
   gl_Position = scene.projection * scene.view * instanceTransform * pos;
}