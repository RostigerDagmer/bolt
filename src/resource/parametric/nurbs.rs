

type Grid2<T> = Vec<Vec<T>>;

struct NURBS {
    degree: (usize, usize), // (p, q) degree in u and v direction
    knots: (Vec<f32>, Vec<f32>), // (u, v)
    control_points: Grid2<Vec3>,
    weights: Grid2<f32>,
}

