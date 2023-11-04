fn gen_plane(size: f32, resolution: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    let step = size / resolution as f32;
    let half_size = size * 0.5;

    for i in 0..=resolution {
        for j in 0..=resolution {
            let x = j as f32 * step - half_size;
            let z = i as f32 * step - half_size;
            vertices.push(Vec4::new(x, 0.0, z, 1.0));
            normals.push(Vec3::new(0.0, 1.0, 0.0));
            uvs.push(Vec4::new(j as f32 / resolution as f32, i as f32 / resolution as f32, 0.0, 0.0));

            if i < resolution && j < resolution {
                let vertex_index = i * (resolution + 1) + j;
                indices.push(vertex_index as u32);
                indices.push((vertex_index + resolution + 1) as u32);
                indices.push((vertex_index + resolution + 1 + 1) as u32);

                indices.push(vertex_index as u32);
                indices.push((vertex_index + resolution + 1 + 1) as u32);
                indices.push((vertex_index + 1) as u32);
            }
        }
    }

    Mesh {
        vertices,
        indices,
        normals,
        uvs,
    }
}