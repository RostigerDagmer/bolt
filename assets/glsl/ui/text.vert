#version 450

layout(set = 0, binding = 0) uniform Scene {
    mat4 mvp;
    mat4 normal;
    mat4 model;
    mat4 view;
    mat4 projection;
} scene;


layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 inColor;
layout (location = 2) in vec4 inNormal;
layout (location = 3) in vec2 inUv;

layout (location = 4) in vec4 glyph_transform_col1;
layout (location = 5) in vec4 glyph_transform_col2;
layout (location = 6) in vec4 glyph_transform_col3;
layout (location = 7) in vec4 glyph_transform_col4;


layout (location = 8) in vec4 glyph_wh_atlas;
layout (location = 9) in vec4 glyph_color;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec4 outColor;

void main()
{
    mat4 glyph_transform = mat4(
        glyph_transform_col1,
        glyph_transform_col2,
        glyph_transform_col3,
        glyph_transform_col4
    );
    float width = glyph_wh_atlas.x / 1024.0;
    float height = glyph_wh_atlas.y / 1024.0;
    float atlas_x = glyph_wh_atlas.z / 1024.0;
    float atlas_y = glyph_wh_atlas.w / 1024.0;
    gl_Position = scene.projection * scene.view * glyph_transform * pos; // * glyph_transform * pos;
    fragTexCoord = vec2(atlas_x + inUv.x * width, atlas_y + inUv.y * height);
    outColor = inColor;
}