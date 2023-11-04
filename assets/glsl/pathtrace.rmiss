#version 460
#extension GL_EXT_ray_tracing : require
#include "payload.glsl"

layout (constant_id = 0) const int ENABLE_SKYLIGHT = 0;
layout(location = 0) rayPayloadInEXT Payload prd;
layout(set = 1, binding = 9) uniform sampler2D skyMap;

vec2 getSkyMapUV(vec3 wI)
{
    float phi = atan(wI.z, wI.x);
    float theta = acos(wI.y);
    float u = 0.5 + phi / (2.0 * 3.14159265358979323846);
    float v = 1.0 - theta / 3.14159265358979323846;
    return vec2(u, v);
}

void main()
{
    // if( bool(ENABLE_SKYLIGHT) ) {
    vec3 wI = normalize( gl_WorldRayDirectionEXT );
    float t = smoothstep(0.35, 0.65, 0.5*(wI.y + 1));
    vec3 skyColor = texture(skyMap, getSkyMapUV(wI)).xyz;
    //vec3 skyColor = mix(vec3(0.58,0.45,0.25), vec3(0.3, 0.4, 0.5), t);
    //bool isSun = dot(wI, normalize(vec3(0.0,.5,.5))) > 0.995;
    //prd.hitValue = mix(skyColor, vec3(120.0, 100.0, 50.0), float(isSun));
    prd.hitValue = skyColor;
    // }
    // else{
    //     prd.hitValue = vec3(0.0);
    // }
    prd.done = 1;
}