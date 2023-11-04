#version 460
#extension GL_EXT_ray_tracing : require
#include "payload.glsl"
#include "spectral.glsl"
#include "postprocess.glsl"

layout (constant_id = 0) const int ENABLE_SKYLIGHT = 0;
layout(location = 0) rayPayloadInEXT SpectralPayload prd;
layout(set = 1, binding = 9) uniform sampler2D skyMap;

vec2 getSkyMapUV(vec3 wI)
{
    float phi = atan(wI.z, wI.x);
    float theta = acos(wI.y);
    float u = 0.5 + phi / (2.0 * 3.14159265358979323846);
    float v = 1.0 - theta / 3.14159265358979323846;
    return vec2(u, v);
}

void skyTexture(out SpectralPayload prd)
{
    vec3 wI = normalize( gl_WorldRayDirectionEXT );
    float t = smoothstep(0.35, 0.65, 0.5*(wI.y + 1));
    vec3 skyColor = texture(skyMap, getSkyMapUV(wI)).xyz;
    prd.hitValue = rgbToSpectrum(prd.wavelength, gammaCorrect(skyColor, 1.0 / 2.2));
}

const float pi = 3.1415926535897932384626433832795;
const float N = 1.0003; // the refractive index of air
const float N2 = N*N;
const float N21 = N2 - 1.0;
const float N22 = N2 + 2.0;
const float rayleigh_const = 8.0 * pi * pi * pi * (N2 - 1.0) * (N2 - 1.0) / (3.0 * N22 * pow(10.0, 25.0));
const float TT = 2.0 * pi;
const float beta_mie = 0.434 * TT * (0.2 * TT);
const float beta_rayleigh = 1.0 / 275.0;

const float sunRadius = 0.51; // around 0.51 degrees
const float sunSolidAngle = 2 * 3.14159 * (1 - cos(sunRadius));
const float sunRadianceScale = sunSolidAngle / 3.14159; // scale to get the perceived radiance

void genSkyLight(out SpectralPayload prd)
{
    // vec3 wI = normalize(gl_WorldRayDirectionEXT);
    // bool isSun = dot(wI, normalize(vec3(0.0,.5,.5))) > 0.9999127335;
    // // scale solar radiance
    int index = int((prd.wavelength - 380.0) / 25.0);
    float irradiance = solarSpectrumNorm[index]; // energy per unit area per unit time. (W/m^-2)
    // float solarRadiance = irradiance / sunSolidAngle; // this is the radiance per unit area. (W/m^-2/sr)
    // solarRadiance = solarRadiance * sunRadianceScale; // scale it based on the solid angle subtended by the sun itself.
    
    // // Compute the index for the sample based on the wavelength
    

    // float lambda =  prd.wavelength; // wavelengths are stored in nm 380.0, ..., 780.0 
    // float lambda2 = lambda * lambda;
    // float lambda4 = lambda2 * lambda2;

    // float p = 2.701603369180368 * lambda4;

    // float beta_rayleigh_lambda = beta_rayleigh / lambda4;
    // float rayleighPhase = 0.75 * (1.0 + cos(prd.wavelength * 2.0)); 
    // float miePhase = 1.5 * ((1.0 - beta_mie * beta_mie) / (2.0 + beta_mie * beta_mie)) * (1.0 + cos(prd.wavelength * prd.wavelength)) / pow((1.0 + beta_mie * beta_mie - 2.0 * beta_mie * cos(prd.wavelength)), 1.5); 

    // float finalRadiance = beta_rayleigh_lambda * rayleighPhase + beta_mie * miePhase;
    
    prd.hitValue = irradiance; // prd.wavelength / 780.0; //clamp(dot(wI, normalize(vec3(0.0,.5,.5))) * p * solarRadiance * 10.0, 0.0, 1.0); // mix(dot(wI, normalize(vec3(0.0,.5,.5))) * beta_rayleigh_lambda * solarRadiance * 100.0, solarRadiance * 100.0, float(isSun));
}

void main()
{
    skyTexture(prd);
    prd.done = 1;
}