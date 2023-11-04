#version 460
#extension GL_EXT_ray_tracing : require
#include "payload.glsl"

layout(location = 0) rayPayloadInEXT PayloadCollision prd;

void main() {
    // velocity direction * velocity magnitude + intersection position
    //prd.hitValue = prd.rayDir * prd.rayRange.y + prd.rayOrigin;
    prd.next_position = prd.origin + prd.velocity_direction * prd.velocity_magnitude;
}