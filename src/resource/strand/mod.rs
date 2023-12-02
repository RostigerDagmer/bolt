
struct SimParams {
    pub friction: f32,
    pub volume_preservation: f32,
    pub bending_stiffness: f32,
    pub stretching_stiffness: f32,
    pub local_shape: f32,
    pub global_length: f32,
}

struct Strand {
    offset: u32,
    count: u32,
    uv: glam::Vec2,
    barycentric: glam::Vec3,
    root_radius: f32,
    tip_radius: f32,
    is_simstrand: bool,
}

