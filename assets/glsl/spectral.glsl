// float rgbToSpectrum(float wavelength, vec3 rgb) {
//     const float red   = 640.0;
//     const float green = 520.0;
//     const float blue  = 460.0;
//     const float width = 200.0;

//     float r = smoothstep(red - width, red + width, wavelength);
//     float g = smoothstep(green - width, green + width, wavelength);
//     float b = smoothstep(blue - width, blue + width, wavelength);

//     return dot(rgb, normalize(vec3(r, g, b)));
// }
const int SAMPLE_COUNT = 16;

// const float solarSpectrum[SAMPLE_COUNT] = float[](
// 0.49751,
// 0.84545,
// 0.9905,
// 1.2791,
// 1.2235,
// 1.3277,
// 1.3096,
// 1.3673,
// 1.3086,
// 1.3299,
// 1.2744,
// 1.276,
// 1.1636,
// 0.98988,
// 1.121,
// 1.0687
// );

const float solarSpectrum[SAMPLE_COUNT] = float[](
    0.044923,
    0.063563,
    0.065953,
    0.077886,
    0.069763,
    0.072766,
    0.070192,
    0.072844,
    0.069880,
    0.066721,
    0.062627,
    0.059156,
    0.055452,
    0.052426,
    0.049329,
    0.046522
);
const float solarSpectrumNorm[SAMPLE_COUNT] = float[](
    0.576779,
    0.816102,
    0.846793,
    1.000000,
    0.895709,
    0.934261,
    0.901217,
    0.935263,
    0.897211,
    0.856656,
    0.804086,
    0.759525,
    0.711961,
    0.673109,
    0.633355,
    0.597306
);

const float d65[SAMPLE_COUNT] = float[](
    49.975500,
    72.587667,
    95.199833,
    117.812000,
    113.343000,
    108.874000,
    104.405000,
    98.836233,
    93.267467,
    87.698700,
    82.335500,
    76.972300,
    71.609100,
    68.867000,
    66.124900,
    63.382800
);

const float skySpectrum[SAMPLE_COUNT] = float[](
    0.497510,
    0.845450,
    0.990500,
    1.279100,
    1.223500,
    1.327700,
    1.309600,
    1.367300,
    1.308600,
    1.329900,
    1.274400,
    1.276000,
    1.163600,
    0.989880,
    1.121000,
    1.068700
);

// const float solarSpectrum[SAMPLE_COUNT] = float[](
//     0.027079,
//     0.046017,
//     0.053911,
//     0.069619,
//     0.066593,
//     0.072265,
//     0.071280,
//     0.074420,
//     0.071225,
//     0.072384,
//     0.069364,
//     0.069451,
//     0.063333,
//     0.053878,
//     0.061014,
//     0.058168
// );

const vec3 cieXYZNorm[SAMPLE_COUNT] = vec3[](
    vec3(0.001368, 0.000040, 0.006450),
    vec3(0.029782, 0.000852, 0.141702),
    vec3(0.313787, 0.015095, 1.542190),
    vec3(0.290800, 0.061909, 1.669200),
    vec3(0.046282, 0.189397, 0.550416),
    vec3(0.019478, 0.583684, 0.128994),
    vec3(0.290400, 0.984346, 0.020300),
    vec3(0.712059, 1.000000, 0.002439),
    vec3(1.047188, 0.742907, 0.001036),
    vec3(0.854450, 0.393119, 0.000190),
    vec3(0.328517, 0.129002, 0.000005),
    vec3(0.072077, 0.027167, 0.000000),
    vec3(0.011359, 0.004232, 0.000000),
    vec3(0.001781, 0.000664, 0.000000),
    vec3(0.000270, 0.000100, 0.000000),
    vec3(0.000042, 0.000015, 0.000000)
);

const vec3 cieXYZ[SAMPLE_COUNT] = vec3[](
    vec3(0.001368, 0.000039, 0.006450),
    vec3(0.029782, 0.000825, 0.141702),
    vec3(0.313787, 0.014630, 1.542190),
    vec3(0.290800, 0.060000, 1.669200),
    vec3(0.046282, 0.183558, 0.550416),
    vec3(0.019478, 0.565690, 0.128994),
    vec3(0.290400, 0.954000, 0.020300),
    vec3(0.712059, 0.969171, 0.002439),
    vec3(1.047188, 0.720004, 0.001036),
    vec3(0.854450, 0.381000, 0.000190),
    vec3(0.328517, 0.125025, 0.000005),
    vec3(0.072077, 0.026329, 0.000000),
    vec3(0.011359, 0.004102, 0.000000),
    vec3(0.001781, 0.000643, 0.000000),
    vec3(0.000270, 0.000097, 0.000000),
    vec3(0.000042, 0.000015, 0.000000)
);

// const vec3 cieXYZ[SAMPLE_COUNT] = vec3[](
//     vec3(0.001368000000,0.0000390000000,0.006450001000), // 380nm
//     vec3(0.014310000000,0.0003960000000,0.067850010000), // 400nm
//     vec3(0.134380000000,0.0040000000000,0.645600000000), // 420nm
//     vec3(0.348280000000,0.0230000000000,1.747060000000), // 440nm
//     vec3(0.290800000000,0.0600000000000,1.669200000000), // 460nm
//     vec3(0.095640000000,0.1390200000000,0.812950100000), // 480nm
//     vec3(0.004900000000,0.3230000000000,0.272000000000), // 500nm
//     vec3(0.063270000000,0.7100000000000,0.078249990000), // 520nm
//     vec3(0.290400000000,0.9540000000000,0.020300000000), // 540nm
//     vec3(0.594500000000,0.9950000000000,0.003900000000), // 560nm
//     vec3(0.916300000000,0.8700000000000,0.001650001000), // 580nm
//     vec3(1.062200000000,0.6310000000000,0.000800000000), // 600nm
//     vec3(0.854449900000,0.3810000000000,0.000190000000), // 620nm
//     vec3(0.447900000000,0.1750000000000,0.000020000000), // 640nm
//     vec3(0.164900000000,0.0610000000000,0.000000000000), // 660nm
//     vec3(0.046770000000,0.0170000000000,0.000000000000) // 680nm
//     // vec3(0.011359160000,0.0041020000000,0.000000000000), // 700nm
// );

const mat3x3 t_xyzlRGB = mat3x3(
    3.240479, -1.537150, -0.498535,
    -0.969256, 1.875991, 0.041556,
    0.055648, -0.204043, 1.057311
);

const mat3x3 t_xyzWideGammutRGB = mat3x3(
    1.4628067, -0.1840623, -0.2743606,
    -0.5217933,  1.4472381,  0.0677227,
    0.0349342, -0.0968930,  1.2884099
);

const mat3x3 t_wideGammutRGBxyz = mat3x3(
     0.7161046,  0.1009296,  0.1471858,
    0.2581874,  0.7249378,  0.0168748,
    0.0000000,  0.0517813,  0.7734287
);

const mat3x3 t_xyzCIERGB = mat3x3(
    2.3706743, -0.9000405, -0.4706338,
    -0.5138850,  1.4253036,  0.0885814,
    0.0052982,-0.0146949,  1.0093968
);

const mat3x3 t_CIERGBxyz = mat3x3(
 0.4887180,  0.3106803,  0.2006017,
 0.1762044,  0.8129847,  0.0108109,
 0.0000000,  0.0102048,  0.9897952
);


const mat3x3 t_lRGBxyz = mat3x3(
    0.412453, 0.357580, 0.180423,
    0.212671, 0.715160, 0.072169,
    0.019334, 0.119193, 0.950227
);

const mat3x3 t_xyzAP1 = mat3x3(
     1.66058,    -0.315295,  -0.24151,
    -0.659926,    1.60839,    0.017298,
    0.00900358, -0.00356713, 0.913644
);

vec3 sRGBtolRGB(vec3 sRGB) {
    return pow(sRGB, vec3(2.2));
}

vec3 xyzToAP1(vec3 xyz) {
    return (xyz * vec3(0.9505, 1.0, 1.0890)) * t_xyzAP1;
}

vec3 xyzTolRGB(vec3 xyz) {
    return (t_xyzlRGB * xyz) / vec3(0.9505, 1.0, 1.0890);
}

vec3 normalizeXYZ (vec3 xyz) {
    vec3 WHITE_POINT_XYZ = vec3(0.9505, 1.0, 1.0890); // white point D65 
    return xyz / WHITE_POINT_XYZ;
}

vec3 lRGBToxyz(vec3 lRGB) {
    return t_wideGammutRGBxyz * lRGB;
}

float rgbToSpectrum(float wavelength, vec3 rgb) {
    int index = int((wavelength - 380.0) / 25.0);
    vec3 c_xyz = lRGBToxyz(rgb);
    vec3 spd = normalize(cieXYZ[index]);
    return dot(c_xyz, spd);
} 

// Cauchy's equation
float getIndexOfRefraction(float wavelength) {
    float A = 1.7280, B = 0.01342;
    return A + B / pow(wavelength * 1e-3, 2.0);
}

float spectralFresnelDielectric(vec3 normal, vec3 direction, float wavelength) {
    float cosTheta = clamp(dot(normal, direction), -1, 1);
    float etaI = 1.0, etaT = getIndexOfRefraction(wavelength);

    if(cosTheta > 0.0) {
        etaI = getIndexOfRefraction(wavelength);
        etaT = 1.0;
    }

    float sinThetaT = etaI / etaT * sqrt(max(0.0, 1 - cosTheta * cosTheta));

    // Handle total internal reflection
    if(sinThetaT >= 1.0) {
        return 1.0;
    }

    float cosThetaT = sqrt(max(0.0, 1 - sinThetaT * sinThetaT));
    float R_parallel = ((etaT * cosTheta) - (etaI * cosThetaT)) / ((etaT * cosTheta) + (etaI * cosThetaT));
    float R_perpendicular = ((etaI * cosTheta) - (etaT * cosThetaT)) / ((etaI * cosTheta) + (etaT * cosThetaT));
    return (R_parallel * R_parallel + R_perpendicular * R_perpendicular) * 0.5;
}