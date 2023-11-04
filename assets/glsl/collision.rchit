#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_scalar_block_layout : enable
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : enable
#include "payload.glsl"
#include "sampling.glsl"

//TODO: https://github.com/nvpro-samples/vk_denoise/blob/master/shaders/pathtrace.rchit

struct ModelVertex {
	vec4 pos;
	vec4 color;
	vec4 normal;
	vec4 uv;
};

    // pub color: glam::Vec3,
    // pub displacement_scale: f32,
    // pub displacement_bias: f32,

    // pub albedo_factor: glam::Vec3,
    // pub sss_factor: glam::Vec3,
    // pub normal_factor: f32,
    // pub roughness_factor: f32,
    // pub metallic_factor: f32,
    // pub ao_factor: f32,
    // pub emissive_factor: glam::Vec3,
    // pub opacity_factor: f32,
    // pub padding0: f32,

struct MaterialInfo {
    vec3 color;
	float displacement_scale;
	float displacement_bias;
	vec3 albedo_factor;
	vec3 sss_factor;
	float normal_factor;
	float roughness_factor;
	float metallic_factor;
	float ao_factor;
    vec3 emissive_factor;
	float opacity_factor;
    float padding0;
};

struct SceneInstance
{
	int  id;
	int  texture_offset;
	int numIndices;
	int dynamic;
	mat4 transform;
	mat4 transform_it;
};

layout(set = 0, binding = 0) uniform Scene {
    mat4 model;
    mat4 view;
    mat4 view_inverse;
    mat4 projection;
    mat4 projection_inverse;
    mat4 model_view_projection;
    uvec3 frame;
} scene;

layout(set = 1, binding = 3, scalar) buffer ScnDesc { SceneInstance i[]; } scnDesc;
layout(set = 1, binding = 4, scalar) buffer Vertices { ModelVertex v[]; } vertices[];
layout(set = 1, binding = 5) buffer Indices { uint64_t i[]; } indices[];
layout(set = 1, binding = 6, scalar) buffer MatBuffer { MaterialInfo mat; } materials[];

layout(location = 0) rayPayloadInEXT PayloadCollision prd;

hitAttributeEXT vec3 attribs;

void main()
{
    // Object of this instance
	uint objId = scnDesc.i[gl_InstanceID].id;
	// Indices of the triangle
	ivec3 ind = ivec3(indices[objId].i[3 * gl_PrimitiveID + 0],
					  indices[objId].i[3 * gl_PrimitiveID + 1],
					  indices[objId].i[3 * gl_PrimitiveID + 2]);
	// Vertex of the triangle
	ModelVertex v0 = vertices[objId].v[ind.x];
	ModelVertex v1 = vertices[objId].v[ind.y];
	ModelVertex v2 = vertices[objId].v[ind.z];

    const vec3 barycentrics = vec3(1.0 - attribs.x - attribs.y, attribs.x, attribs.y);
    vec3 pos = v0.pos.xyz * barycentrics.x + v1.pos.xyz * barycentrics.y + v2.pos.xyz * barycentrics.z;
    
    vec3 normal = v0.normal.xyz * barycentrics.x + v1.normal.xyz * barycentrics.y + v2.normal.xyz * barycentrics.z;
    
    // find out how long the path after intersection would have been
    float samedir = sign(dot(normal, prd.velocity_direction));
    float t = prd.velocity_magnitude - length(pos - prd.origin);
    // reflect the ray along the normal and shorten it using both materials damping factor
    vec3 reflected = reflect(prd.velocity_direction, samedir * normal);
    prd.origin = pos; // <- TODO: look the damping factor up in the material
    prd.velocity_direction = normalize(reflected);
    prd.velocity_magnitude = t;
    prd.next_position = prd.velocity_direction * prd.velocity_magnitude + pos;

}