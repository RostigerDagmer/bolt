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
	int albedo;
	int sss;
	int normal;
	int roughness;
	int metallic;
	int ao;
	int emissive;
	int opacity;
	int displacement;
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
layout(set = 1, binding = 8) uniform sampler2D textures[];

layout(location = 0) rayPayloadInEXT Payload prd;

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

	MaterialInfo mat = materials[gl_InstanceID].mat;
	
	if(mat.emissive_factor.r >= 1.0 || mat.emissive_factor.g >= 1.0 || mat.emissive_factor.b >= 1.0) {
 		prd.hitValue = mat.emissive_factor;
		prd.done     = 1;
		prd.depth++;
 		return;
 	}

	const vec3 barycentrics = vec3(1.0 - attribs.x - attribs.y, attribs.x, attribs.y);
	vec2 uv = v0.uv.xy * barycentrics.x + v1.uv.xy * barycentrics.y + v2.uv.xy * barycentrics.z;
	// Computing the normal at hit position
	vec3 normal;
	if (mat.normal < 0) {
		normal = v0.normal.xyz * barycentrics.x + v1.normal.xyz * barycentrics.y + v2.normal.xyz * barycentrics.z;
	} else {
		normal = texture(textures[mat.normal], uv).xyz;
	}
	
	// Transforming the normal to world space
	normal = normalize(vec3(scnDesc.i[gl_InstanceID].transform_it * vec4(normal, 0.0)));
	// Computing the coordinates of the hit position
	vec3 worldPos = v0.pos.xyz * barycentrics.x + v1.pos.xyz * barycentrics.y + v2.pos.xyz * barycentrics.z;
	// Transforming the position to world space
	worldPos = vec3(scnDesc.i[gl_InstanceID].transform * vec4(worldPos, 1.0));

	vec3 vertex_color;
	
	if (mat.albedo < 0) {
		vertex_color = v0.color.xyz * barycentrics.x + v1.color.xyz * barycentrics.y + v2.color.xyz * barycentrics.z;
	} else {
		vertex_color = texture(textures[mat.albedo], uv).xyz;
	}

	vec3 wI = normalize(gl_WorldRayDirectionEXT);
	vec3 nO = normal * sign( dot(normal, -wI) );
	float alphaSquared = mat.roughness_factor * mat.roughness_factor;
	vec2 Xi = nextRand2(prd.rng);
	float rand = nextRand(prd.rng);

	prd.rayOrigin = worldPos + 0.0001 * nO;
	if( rand < mat.metallic_factor ) {
		prd.rayDir = sampleGGXDistribution(reflect(gl_WorldRayDirectionEXT, nO), Xi, alphaSquared);
		prd.hitValue   = mat.color.xyz * vertex_color;
	}
	else {
		vec3 m = sampleGGXDistribution(nO, Xi, alphaSquared);
		float diEl = fresnelDielectric(nO, m, 1.0/1.5);
		if( rand < diEl ) {
			prd.rayDir = reflect(gl_WorldRayDirectionEXT, m);
			prd.hitValue = vertex_color * diEl;
		}
		else {
			prd.rayDir = sampleCosineWeightedHemisphere(nO, Xi);
			prd.hitValue = mat.color.xyz * vertex_color;
		}
	}
	prd.depth++;
}
