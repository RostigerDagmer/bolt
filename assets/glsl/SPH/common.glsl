#define PI 3.1415926538

uint hash3d(ivec3 position)
{
    const uint p1 = 73856093;   
    const uint p2 = 19349663;  
    const uint p3 = 83492791;

    uint xHash = uint(position.x + 2147483648) * p1;
    uint yHash = uint(position.y + 2147483648) * p2;
    uint zHash = uint(position.z + 2147483648) * p3;

    return xHash ^ yHash ^ zHash;
}

// integer offsets to neighbouring cells
const ivec3 cellOffsets[27] = ivec3[](
    ivec3(-1, -1, -1),
    ivec3(-1, -1,  0),
    ivec3(-1, -1,  1),
    ivec3(-1,  0, -1),
    ivec3(-1,  0,  0),
    ivec3(-1,  0,  1),
    ivec3(-1,  1, -1),
    ivec3(-1,  1,  0),
    ivec3(-1,  1,  1),
    ivec3( 0, -1, -1),
    ivec3( 0, -1,  0),
    ivec3( 0, -1,  1),
    ivec3( 0,  0, -1),
    ivec3( 0,  0,  0),
    ivec3( 0,  0,  1),
    ivec3( 0,  1, -1),
    ivec3( 0,  1,  0),
    ivec3( 0,  1,  1),
    ivec3( 1, -1, -1),
    ivec3( 1, -1,  0),
    ivec3( 1, -1,  1),
    ivec3( 1,  0, -1),
    ivec3( 1,  0,  0),
    ivec3( 1,  0,  1),
    ivec3( 1,  1, -1),
    ivec3( 1,  1,  0),
    ivec3( 1,  1,  1)
);

float smoothingKernel(float radius, float dist) {
    float volume = PI * pow(radius, 4) / 2.0;
    float y = max(0, radius - dist);
    return y * y * y / volume;
}

