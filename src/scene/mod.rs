mod camera;
pub use camera::*;
use rayon::prelude::*;

// Much of this was directly based on:
// https://github.com/adrien-ben/gltf-viewer-rs/blob/master/model/src/mesh.rs

use crate::resource::{mesh, material};
pub use mesh::*;
pub use material::*;
pub mod daz;

use crate::{Buffer, BufferInfo, Context, Texture2d};

use gltf::{
    buffer::Buffer as GltfBuffer,
    mesh::{Reader, Semantic}, Texture,
};
use std::path::PathBuf;
use std::sync::Arc;

pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub vulkan_meshes: Vec<Box<VulkanMesh>>,
    pub materials: Vec<MaterialInfo>,
    pub material_buffer: Buffer,
    pub camera: Option<Camera>,
    pub textures: Vec<Texture2d>,
}

fn find_mesh(node: &gltf::Node, transforms: &mut Vec<glam::Mat4>, mesh_index: usize) -> bool {
    transforms.push(glam::Mat4::from_cols_array_2d(&node.transform().matrix()));
    let found = match node.mesh() {
        Some(node_mesh) => node_mesh.index() == mesh_index,
        None => false,
    };
    if found {
        return true;
    }
    for ref child in node.children() {
        if find_mesh(child, transforms, mesh_index) {
            return true;
        }
    }
    transforms.pop();
    return false;
}

fn calc_mesh_global_transform(gltf: &gltf::Document, mesh_index: usize) -> glam::Mat4 {
    let mut global_transform = glam::Mat4::IDENTITY;
    let mut transforms = Vec::<glam::Mat4>::new();
    for node in gltf.nodes() {
        if find_mesh(&node, &mut transforms, mesh_index) {
            transforms.iter().for_each(|t| {
                global_transform = global_transform * *t;
            });
            break;
        }
    }
    global_transform
}


fn load_textures_par(document: &gltf::Document, filepath: &PathBuf, context: Arc<Context>) -> Vec<Texture2d> {
    let paths = document.images().map(|im| {
        let source = im.source();
        match source {
            gltf::image::Source::View { view, mime_type } => {
                println!("mime_type: {:?}", mime_type);
                panic!("loading gltf embedded textures not implemented");
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                println!("mime_type: {:?}; uri: {:?}", mime_type, uri);
                return filepath.as_path().parent().unwrap().join(PathBuf::from(uri).as_path()).as_os_str().to_os_string().into_string().unwrap() // PathBuf::from(uri).as_path();
            }
        }
    })
    .collect::<Vec<String>>();

    let mut images = crate::resource::image::load_images_par::<u8>(&paths);
    println!("images loaded");
    images.iter_mut().map(|i| {
        i.set_format(ash::vk::Format::R8G8B8A8_UNORM);
        Texture2d::from_image(i, context.clone())
        }
    ).collect()
}

fn load_textures_lin(document: &gltf::Document, filepath: &PathBuf, context: Arc<Context>) -> Vec<Texture2d> {
    document.images().map(|image| {
        let source = image.source();
        match source {
            gltf::image::Source::View { view, mime_type } => {
                println!("mime_type: {:?}", mime_type);
                panic!("loading gltf embedded textures not implemented");
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                println!("mime_type: {:?}; uri: {:?}", mime_type, uri);
                return Texture2d::new(context.clone(), filepath.as_path().parent().unwrap().join(PathBuf::from(uri).as_path()));
            }
        }
    }).collect()
}


fn load_glts(context: Arc<Context>, filepath: &PathBuf) -> Result<Scene, Box<dyn std::error::Error>> {
    let mut meshes = Vec::<Mesh>::new();
    let res = gltf::import(filepath);
    let (gltf, buffers, _) = match res {
        Ok(s) => s,
        Err(e) => {
            return Err(Box::new(e));
        }
    };
    println!("filepath: {:?}", filepath.as_path().parent());
    
    let textures = load_textures_par(&gltf, filepath, context.clone());
    println!("textures loaded");
    // println!("{:#?}", gltf);
    let materials = gltf
        .materials()
        .map(|m| {
            let mut mat: MaterialInfo = m.into();
            // if mat.color == glam::Vec3::ZERO {
            mat.color = glam::Vec3::splat(1.0);
            // mat.opacity_factor = 0.0;
            // }
            mat
        })
        .collect::<Vec<MaterialInfo>>();
    let material_buffer = Buffer::from_data(
        context.clone(),
        BufferInfo::default().usage_storage().gpu_only(),
        &materials,
    );

    for mesh in gltf.meshes() {
        let mut mesh_indices = Vec::<u32>::new();
        let mut mesh_vertices = Vec::<ModelVertex>::new();
        let mut primitive_sections = Vec::<PrimitiveSection>::new();

        // println!("Mesh #{}", mesh.index());

        for (primitive_index, primitive) in mesh.primitives().enumerate() {
            // println!("- Primitive #{}", primitive.index());

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            let offset = mesh_vertices.len();

            if let Some(_) = primitive.get(&Semantic::Positions) {
                let positions = read_positions(&reader);
                let normals = read_normals(&reader);
                let tex_coords_0 = read_tex_coords(&reader, 0);
                let colors = read_colors(&reader);

                positions.iter().enumerate().for_each(|(index, position)| {
                    let pos = *position;
                    let norm = *normals.get(index).unwrap_or(&[0.0, 1.0, 0.0]);
                    let uv = *tex_coords_0.get(index).unwrap_or(&[0.0, 0.0]);
                    let col = *colors.get(index).unwrap_or(&[1.0, 1.0, 1.0, 1.0]);
                    mesh_vertices.push(ModelVertex {
                        pos: glam::vec4(pos[0], pos[1], pos[2], 1.0),
                        normal: glam::vec4(norm[0], norm[1], norm[2], 1.0),
                        color: glam::vec4(col[0], col[1], col[2], col[3]),
                        uv: glam::vec4(uv[0], uv[1], 0.0, 0.0),
                    });
                });
            };

            primitive_sections.push(PrimitiveSection {
                index: primitive_index,
                vertices: BufferPart {
                    offset,
                    element_count: mesh_vertices.len() - offset,
                },
                indices: None,
                material_index: primitive.material().index(),
            });
            // println!("  Vertices {:?}", (offset, mesh_vertices.len() - offset));

            if let Some(iter) = reader.read_indices() {
                let offset = mesh_indices.len();
                mesh_indices.extend(iter.into_u32());
                primitive_sections.last_mut().unwrap().indices = Some(BufferPart {
                    offset,
                    element_count: mesh_indices.len() - offset,
                });
                // println!("    Indices {:?}", (offset, mesh_indices.len() - offset));
            }
        }

        let global_transform = calc_mesh_global_transform(&gltf, mesh.index());

        let name = match mesh.name() {
            Some(name) => name.to_owned(),
            None => String::new(),
        };
        let mut mesh = Mesh::new(
            name,
            mesh_vertices,
            mesh_indices,
            global_transform,
            primitive_sections,
        );

        meshes.push(mesh);
    }

    let mut camera = None;
    for gltf_camera in gltf.cameras() {
        match gltf_camera.projection() {
            gltf::camera::Projection::Orthographic(_) => {}
            gltf::camera::Projection::Perspective(persp) => {
                for node in gltf.nodes() {
                    let found = match node.camera() {
                        Some(node_camera) => node_camera.index() == gltf_camera.index(),
                        None => false,
                    };
                    if found {
                        let view_matrix =
                            glam::Mat4::from_cols_array_2d(&node.transform().matrix());
                        camera = Some(Camera::from_view(
                            view_matrix,
                            persp.yfov(),
                            persp.znear(),
                            persp.zfar().unwrap_or(100.0),
                        ));
                        break;
                    }
                }
            }
        }
        //Support for the first (default) camera only
        break;
    }

    let vulkan_meshes: Vec<Box<VulkanMesh>> = meshes.iter().map(|mesh| Box::new(mesh.to_vulkan_mesh(context.clone()))).collect();

    Ok(Scene {
        meshes,
        vulkan_meshes,
        materials,
        material_buffer,
        camera,
        textures,
    })
}

fn load_daz(context: Arc<Context>, filepath: &PathBuf) -> Result<Scene, Box<dyn std::error::Error>> {
    let res = daz::import(filepath);
    let dsf = match res {
        Ok(dsf) => dsf,
        Err(err) => {
            return Err(err);
        }
    };
    let meshes: Vec<Mesh> = dsf.geometry_library.iter().map(|geo| {

        // we need to go through the mesh by indices so we can be sure to only handle triangles
        // and not quads
        let index_buffer = geo.polylist.values.iter().fold(Vec::new(), |mut acc, quad| {
            // reorder
            match quad.len() {
                5 => {
                    acc.push(quad[2].clone());
                    acc.push(quad[3].clone());
                    acc.push(quad[4].clone());
                },
                6 => {
                    acc.push(quad[2].clone());
                    acc.push(quad[3].clone());
                    acc.push(quad[4].clone());
                    acc.push(quad[2].clone());
                    acc.push(quad[4].clone());
                    acc.push(quad[5].clone());
                }
                _ => {}
            };
            acc
        });

        let vertices = geo.vertices.values.par_iter().map(|v| {
            ModelVertex {
                pos: glam::vec4(v[0], v[1], v[2], 1.0),
                normal: glam::vec4(0.0, 0.0, 0.0, 0.0),
                color: glam::vec4(1.0, 1.0, 1.0, 1.0),
                uv: glam::vec4(0.0, 0.0, 0.0, 0.0),
            }
        }).collect::<Vec<ModelVertex>>();
        
        let sections = vec![PrimitiveSection {
            index: 0,
            vertices: BufferPart {
                offset: 0,
                element_count: vertices.len() as usize,
            },
            indices: Some(BufferPart {
                offset: 0,
                element_count: index_buffer.len() as usize,
            }),
            material_index: Some(0),
        }];
        let mut mesh = Mesh::new(geo.name.clone(), vertices, index_buffer, glam::Mat4::IDENTITY, sections);
        
        let normals = mesh.vertex_iter()
                .map(|vertex_id| mesh.vertex_normal(vertex_id))
                .collect::<Vec<_>>();
        
        mesh.vertices.iter_mut().zip(normals).for_each(|(vertex, normal)| {
            vertex.normal = glam::vec4(normal.x, normal.y, normal.z, 0.0);
        });

        mesh

    }).collect();

    // "aspectRatio": 1.7777778,
    // "yfov": 22.0,
    // "znear": 0.01,
    // "zfar": 100.0
    let materials = vec![MaterialInfo{
            color: glam::Vec3::new(0.8, 0.5, 0.7),
            metallic_factor: 0.0,
            roughness_factor: 0.4,
            emissive_factor: glam::Vec3::splat(0.0),
            displacement_scale: 0.0,
            displacement_bias: 0.0,
        
            albedo_factor: glam::Vec3::new(1.0, 1.0, 1.0),
            sss_factor: glam::Vec3::new(0.6, 0.4, 0.2),
            normal_factor: 1.0,
            ao_factor: 1.0,
            opacity_factor: 1.0,
            ..Default::default()
        }];
    let material_buffer = Buffer::from_data(
        context.clone(),
        BufferInfo::default().usage_storage().gpu_only(),
        &materials,
    );
    let vulkan_meshes: Vec<Box<VulkanMesh>> = meshes.iter().map(|mesh| Box::new(mesh.to_vulkan_mesh(context.clone()))).collect();

    Ok(Scene {
        meshes: meshes,
        vulkan_meshes,
        materials: materials,
        material_buffer: material_buffer,
        camera: Some(Camera::new(glam::vec2(1280.0, 720.0))),
        textures: Vec::new(),
    })
}

pub fn load_scene(context: Arc<Context>, filepath: &PathBuf) -> Scene {
    // TODO: make this chain of responsibility
    let scene = match filepath.extension().unwrap().to_str().unwrap() {
        "gltf" => load_glts(context.clone(), filepath),
        "glb" => load_glts(context.clone(), filepath),
        "dsf" => load_daz(context.clone(), filepath),
        _ => panic!("Unsupported file format"),
    };
    scene.unwrap()
}

fn read_indices<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Option<Vec<u32>>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_indices()
        .map(|indices| indices.into_u32().collect::<Vec<_>>())
}

fn read_positions<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 3]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_positions()
        .expect("Position primitives should be present")
        .collect()
}

fn read_normals<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 3]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_normals()
        .map_or(vec![], |normals| normals.collect())
}

fn read_tex_coords<'a, 's, F>(reader: &Reader<'a, 's, F>, channel: u32) -> Vec<[f32; 2]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_tex_coords(channel)
        .map_or(vec![], |coords| coords.into_f32().collect())
}

fn read_colors<'a, 's, F>(reader: &Reader<'a, 's, F>) -> Vec<[f32; 4]>
where
    F: Clone + Fn(GltfBuffer<'a>) -> Option<&'s [u8]>,
{
    reader
        .read_colors(0)
        .map_or(vec![], |colors| colors.into_rgba_f32().collect())
}
