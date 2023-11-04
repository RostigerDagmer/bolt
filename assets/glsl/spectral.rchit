#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_scalar_block_layout : enable
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : enable
#include "payload.glsl"
#include "sampling.glsl"
#include "spectral.glsl"

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

layout(location = 0) rayPayloadInEXT SpectralPayload prd;

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

    float emissive_spec = rgbToSpectrum(prd.wavelength, mat.emissive_factor);
	
	if(emissive_spec >= 1.0) {
 		prd.hitValue = emissive_spec;
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
	
	if (mat.albedo < 0) { // no albedo texture
		vertex_color = v0.color.xyz * barycentrics.x + v1.color.xyz * barycentrics.y + v2.color.xyz * barycentrics.z;
	} else {
		vertex_color = texture(textures[mat.albedo], uv).xyz;
	}


	vec3 I = -normalize(gl_WorldRayDirectionEXT); // incident direction
	vec3 N = normalize(normal);                   // normal at hit point
	float ior = getIndexOfRefraction(prd.wavelength);                              // index of refraction
											
	// Check if ray is going from inside the material to outside
	float cosi = clamp(dot(I, N), -1.0, 1.0);
	float etai = 1.0, etat = ior;
	vec3 n = N;
	if(cosi < 0.0) { 
		cosi = -cosi;
	} else { 
		float temp = etai; 
		etai = etat;
		etat = temp;
		n = -N; 
	}
	prd.rayOrigin = worldPos + 0.0001 * n;

	// Compute refracted ray using Snell's law
	float eta = etai / etat;
	float k = 1.0 - eta * eta * (1.0 - cosi * cosi);
	vec3 T = k < 0.0 ? vec3(0.0) : eta * I + (eta * cosi - sqrt(k)) * n; // Refracted direction
	float R0 = ((etat - etai) / (etat + etai)) * ((etat - etai) / (etat + etai));

	// Compute reflected ray using Fresnel equations
	vec3 R = reflect(I, N); // Reflected direction
	vec2 Xi = nextRand2(prd.rng);
	float alphaSquared = mat.roughness_factor * mat.roughness_factor;

	vec3 sss = vec3(0.0);
	if (mat.sss_factor.x > 0.0 || mat.sss_factor.y > 0.0 || mat.sss_factor.z > 0.0) {
		vec3 rd = normalize(reflect(gl_WorldRayDirectionEXT, n));
		vec3 sssColor = vertex_color;
		if (mat.sss >= 0) {
			vec3 sssColor = texture(textures[mat.sss], uv).xyz;
		}
		

		// Burley subsurface scattering simulation
		vec3 scatterDist = -log(1.0 - nextRand(prd.rng)) * mat.sss_factor;
		vec3 pos = gl_WorldRayOriginEXT.xyz + scatterDist * rd;
		
		// Query the nearest surface
		sss = (1.0 - exp(-scatterDist)) * sssColor * max(0.0, dot(rd, mat.sss_factor));

	}

	float cosO = abs(dot(I, N));
	float fresnel_reflectance = R0 + (1.0 - R0) * pow(1 - cosO, 5.0);
	float fresnel_transmittance = 1.0 - fresnel_reflectance;

	float BRDF = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color);

	vec3 rayDir_refl = reflect(gl_WorldRayDirectionEXT, sampleGGXDistribution(n, Xi, alphaSquared));
	prd.rayDir = rayDir_refl;
	prd.hitValue = BRDF * fresnel_transmittance + fresnel_reflectance + rgbToSpectrum(prd.wavelength, sss);


	/// REFLECTANCE

	// float R0 = ((etat - etai) / (etat + etai)) * ((etat - etai) / (etat + etai));

	// float cosO = abs(dot(I, N));
	// float fresnel_reflectance = R0 + (1.0 - R0) * pow(1 - cosO, 5.0);
	// float fresnel_transmittance = 1.0 - fresnel_reflectance;

	// float alphaSquared = mat.roughness_factor * mat.roughness_factor;
	// vec2 Xi = nextRand2(prd.rng);

	// float BRDF = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color);
	// vec3 rayDir_refl = reflect(gl_WorldRayDirectionEXT, sampleGGXDistribution(n, Xi, alphaSquared));
	// prd.rayDir = rayDir_refl;
	// prd.hitValue = BRDF * fresnel_transmittance + fresnel_reflectance;


	/// TRANSMITTANCE

	// float BRDF = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color);
	// vec3 rayDir_refl = reflect(gl_WorldRayDirectionEXT, sampleGGXDistribution(n, Xi, alphaSquared));
	// vec3 rayDir_refr = refract(gl_WorldRayDirectionEXT, sampleGGXDistribution(n, Xi, alphaSquared), eta);

	// prd.rayDir = mix(rayDir_refr, rayDir_refl, fresnel_reflectance);

	// prd.hitValue = BRDF * ((1.0 - mat.opacity_factor) * fresnel_transmittance + mat.opacity_factor * fresnel_reflectance);

	// float rand = nextRand(prd.rng);
	// if (rand < mat.opacity_factor) {
	// 	prd.rayDir = reflect(gl_WorldRayDirectionEXT, sampleGGXDistribution(n, Xi, alphaSquared));
	// 	prd.hitValue = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color) * fresnel_reflectance;
	// } else {
	// 	prd.rayDir = refract(gl_WorldRayDirectionEXT, sampleGGXDistribution(n, Xi, alphaSquared), eta);
	// 	prd.hitValue = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color) * fresnel_transmittance;
	// }


	// if( rand < mat.metallic_factor ) {
	// 	prd.rayDir = sampleGGXDistribution(reflect(gl_WorldRayDirectionEXT, nO), Xi, alphaSquared);
	// 	prd.hitValue = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color);
	// }
	// else {
	// 	vec3 m = sampleGGXDistribution(nO, Xi, alphaSquared);
	// 	float diEl = fresnelDielectric(nO, m, 1.0 / 1.5);
	// 	if( rand < diEl ) {
	// 		prd.rayDir = reflect(gl_WorldRayDirectionEXT, m);
	// 		prd.hitValue = rgbToSpectrum(prd.wavelength, vertex_color) * diEl;
	// 	}
	// 	else {
	// 		prd.rayDir = sampleCosineWeightedHemisphere(nO, Xi);
	// 		prd.hitValue = rgbToSpectrum(prd.wavelength, mat.color.xyz) * rgbToSpectrum(prd.wavelength, vertex_color);
	// 	}
	// }
	prd.depth++;
}
