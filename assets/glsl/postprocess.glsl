#ifndef POSTPROCESS_GLSL
#define POSTPROCESS_GLSL

// From http://filmicgames.com/archives/75
vec3 Uncharted2Tonemap(vec3 x)
{
	float A = 0.15;
	float B = 0.50;
	float C = 0.10;
	float D = 0.20;
	float E = 0.02;
	float F = 0.30;
	return ((x*(A*x+C*B)+D*E)/(x*(A*x+B)+D*F))-E/F;
}

vec3 tonemapUncharted2( in vec3 color )
{
	const float W = 11.2;
	const float exposureBias = 2.0;
	vec3 curr = Uncharted2Tonemap(exposureBias * color);
	vec3 whiteScale = 1.0 / Uncharted2Tonemap(vec3(W));
	return curr * whiteScale;
}

vec3 ACESFilm( vec3 x ) {
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return clamp( ( x * ( a * x + b ) ) / ( x * ( c * x + d ) + e ), vec3(0), vec3(1) );
}

vec3 exposure(vec3 color, float fstop) {
   return color * pow(2.0,fstop);
}

vec3 gammaCorrect( in vec3 color, float power )
{
    return pow( color, vec3(1.0f / power) );
}

float sRGB_InvEOTF(float c)
{
    return c > 0.0031308 ? pow(c, 1.0/2.4) * 1.055 - 0.055 : c * 12.92;
}

float sRGB_EOTF(float c)
{
    return c > 0.04045 ? pow(c / 1.055 + 0.055/1.055, 2.4) : c / 12.92;
}

// vec3 sRGB_InvEOTF(vec3 rgb)
// {
//     return If(greaterThan(rgb, vec3(0.0031308)), pow(rgb, vec3(1.0/2.4)) * 1.055 - 0.055, rgb * 12.92);
// }

// vec3 sRGB_EOTF(vec3 rgb)
// {
//     return If(greaterThan(rgb, vec3(0.04045)), pow(rgb / 1.055 + 0.055/1.055, vec3(2.4)), rgb / 12.92);
// }


float ACEScc_from_Linear(float lin) 
{    
    if (lin <= 0.0) 
        return -0.3584474886;
    
    if (lin < exp2(-15.0))
    	return log2(exp2(-16.0) + lin * 0.5) / 17.52 + (9.72/17.52);
    
    return log2(lin) / 17.52 + (9.72/17.52);
}

vec3 ACEScc_from_Linear(vec3 lin) 
{
    return vec3(ACEScc_from_Linear(lin.r),
                ACEScc_from_Linear(lin.g),
                ACEScc_from_Linear(lin.b));
}


float Linear_from_ACEScc(float cc) 
{
    if (cc < -0.3013698630)
    	return exp2(cc * 17.52 - 9.72)*2.0 - exp2(-16.0)*2.0;
    
    return exp2(cc * 17.52 - 9.72);
}

vec3 Linear_from_ACEScc(vec3 cc) 
{
    return vec3(Linear_from_ACEScc(cc.r),
                Linear_from_ACEScc(cc.g),
                Linear_from_ACEScc(cc.b));
}


float ACEScct_from_Linear(float lin)
{
    if(lin > 0.0078125)
        return log2(lin) / 17.52 + (9.72/17.52);
    
	return lin * 10.5402377416545 + 0.0729055341958355;
}

vec3 ACEScct_from_Linear(vec3 lin) 
{
    return vec3(ACEScct_from_Linear(lin.r),
                ACEScct_from_Linear(lin.g),
                ACEScct_from_Linear(lin.b));
}


float Linear_from_ACEScct(float cct)
{
    if(cct > 0.155251141552511)
        return exp2(cct * 17.52 - 9.72);
    
	return cct / 10.5402377416545 - (0.0729055341958355/10.5402377416545);
}

vec3 Linear_from_ACEScct(vec3 cct) 
{
    return vec3(Linear_from_ACEScct(cct.r),
                Linear_from_ACEScct(cct.g),
                Linear_from_ACEScct(cct.b));
}



// ACES fit by Stephen Hill (@self_shadow)
// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl 

// sRGB => XYZ => D65_2_D60 => AP1
const mat3 sRGBtoAP1 = mat3
(
	0.613097, 0.339523, 0.047379,
	0.070194, 0.916354, 0.013452,
	0.020616, 0.109570, 0.869815
);

const mat3 AP1toSRGB = mat3
(
     1.704859, -0.621715, -0.083299,
    -0.130078,  1.140734, -0.010560,
    -0.023964, -0.128975,  1.153013
);

// AP1 => RRT_SAT
const mat3 RRT_SAT = mat3
(
	0.970889, 0.026963, 0.002148,
	0.010889, 0.986963, 0.002148,
	0.010889, 0.026963, 0.962148
);


// sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
const mat3 ACESInputMat = mat3
(
    0.59719, 0.35458, 0.04823,
    0.07600, 0.90834, 0.01566,
    0.02840, 0.13383, 0.83777
);

// ODT_SAT => XYZ => D60_2_D65 => sRGB
const mat3 ACESOutputMat = mat3
(
     1.60475, -0.53108, -0.07367,
    -0.10208,  1.10813, -0.00605,
    -0.00327, -0.07276,  1.07602
);

vec3 RRTAndODTFit(vec3 x)
{
    vec3 a = (x            + 0.0245786) * x;
    vec3 b = (x * 0.983729 + 0.4329510) * x + 0.238081;
    
    return a / b;
}


vec3 ToneTF0(vec3 x)
{
    vec3 a = (x            + 0.0509184) * x;
    vec3 b = (x * 0.973854 + 0.7190130) * x + 0.0778594;
    
    return a / b;
}

vec3 ToneTF1(vec3 x)
{
    vec3 a = (x          + 0.0961727) * x;
    vec3 b = (x * 0.9797 + 0.6157480) * x + 0.213717;
    
    return a / b;
}

vec3 ToneTF2(vec3 x)
{
    vec3 a = (x            + 0.0822192) * x;
    vec3 b = (x * 0.983521 + 0.5001330) * x + 0.274064;
    
    return a / b;
}


// https://twitter.com/jimhejl/status/1137559578030354437
vec3 ToneMapFilmicALU(vec3 x)
{
    x *= 0.665;
    
   #if 0
    x = max(vec3(0.0), x - 0.004f);
    x = (x * (6.2 * x + 0.5)) / (x * (6.2 * x + 1.7) + 0.06);
    
    x = sRGB_EOTF(x);
   #else
    x = max(vec3(0.0), x);
    x = (x * (6.2 * x + 0.5)) / (x * (6.2 * x + 1.7) + 0.06);
    
    x = pow(x, vec3(2.2));// using gamma instead of sRGB_EOTF + without x - 0.004f looks about the same
   #endif
    
    return x;
}


vec3 Tonemap_ACESFitted(vec3 srgb)
{
    vec3 color = srgb * ACESInputMat;
   
    color = ToneTF2(color);

    // color = RRTAndODTFit(color);
    
    color = color * ACESOutputMat;

    return color;
}

vec3 Tonemap_ACESFitted2(vec3 acescg)
{
    vec3 color = acescg * RRT_SAT;
    
    color = ToneTF2(color); 

    // color = RRTAndODTFit(color);
    // color = ToneMapFilmicALU(color);
    
    color = color * ACESOutputMat;
    //color = ToneMapFilmicALU(color);

    return color;
}

vec3 ColorGrade(vec3 col)
{
    col = ACEScct_from_Linear(col);
    {
        vec3 s = vec3(1.1, 1.2, 1.0);
        vec3 o = vec3(0.1, 0.0, 0.1);
        vec3 p = vec3(1.4, 1.3, 1.3);
        
        col = pow(col * s + o, p);
    }
    col = Linear_from_ACEScct(col);
    
    return col;
}

#endif