#version 460

layout(set = 0, binding = 0) uniform Scene {
    mat4 mvp;
} scene;

layout(location = 0) in vec4 inPos;

void main() {
    gl_Position = scene.mvp * inPos;
    gl_PointSize = 4.0; //  The size of the point
}