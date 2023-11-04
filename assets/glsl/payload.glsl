#ifndef PAYLOAD_GLSL
#define PAYLOAD_GLSL

struct Payload
{
    vec3 hitValue;
    uint  depth;
    uint  sampleId;
    uint  done;
    vec3 rayOrigin;
    vec3 rayDir;
    vec2 rayRange;
    float roughness;
    uint rng;
};

struct SpectralPayload
{
    float hitValue;
    float wavelength;
    uint depth;
    uint sampleId;
    uint done;
    vec3 rayOrigin;
    vec3 rayDir;
    vec2 rayRange;
    uint rng;
};


struct PayloadCollision {
    vec3 origin;
    vec3 next_position;
    vec3 velocity_direction;
    vec3 normal;
    float velocity_magnitude;
    float position_eps;
    float mass;
    float energy_dissipated;
};
#endif
