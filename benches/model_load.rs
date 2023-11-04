// use bolt::AppSettings;
// use bolt::SharedContext;
// use bolt::Window;
// use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use ash::vk;
// use bolt::scene::*;
// use bolt::resource::ResourceManager;
// use rayon::prelude::*;

// // Much of this was directly based on:
// // https://github.com/adrien-ben/gltf-viewer-rs/blob/master/model/src/mesh.rs

// use bolt::resource::mesh;
// use bolt::resource::material;
// use glam::Vec4Swizzles;
// pub use mesh::*;
// pub use material::*;
// // use bolt::scene::{Scene, daz};

// use bolt::{Buffer, BufferInfo, Context};
// use winit::event_loop;
// use winit::event_loop::EventLoop;
// use std::path::PathBuf;
// use std::sync::Arc;

// pub struct PerFrameData {
//     pub ubo: bolt::Buffer,
//     pub desc_set: bolt::DescriptorSet,
// }
// pub struct AppData {
//     pub scene: Scene,
// }

// fn load_daz(filepath: &PathBuf) {
//     let dsf = daz::import(filepath).unwrap();
//     dsf.geometry_library.iter().for_each(|geo| {

//         // we need to go through the mesh by indices so we can be sure to only handle triangles
//         // and not quads
//         let index_buffer = geo.polylist.values.iter().fold(Vec::new(), |mut acc, quad| {
//             // reorder
//             match quad.len() {
//                 5 => {
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                 },
//                 6 => {
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[2].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[5].clone());
//                 }
//                 _ => {}
//             };
//             acc
//         });

//         let mut vertices = geo.vertices.values.iter().map(|v| {
//             ModelVertex {
//                 pos: glam::vec4(v[0], v[1], v[2], 1.0),
//                 normal: glam::vec4(0.0, 0.0, 0.0, 0.0),
//                 color: glam::vec4(1.0, 1.0, 1.0, 1.0),
//                 uv: glam::vec4(0.0, 0.0, 0.0, 0.0),
//             }
//         }).collect::<Vec<ModelVertex>>();

//         index_buffer.iter()
//         .collect::<Vec<_>>()
//         .windows(3)
//         .for_each(|i| {
//             let v_prev = vertices[*i[0] as usize].pos;
//             let v = vertices[*i[1] as usize].pos;
//             let v_next = vertices[*i[2] as usize].pos;
//             let A  = v - v_prev;
//             let B = v_next - v;
//             let normal = glam::Vec4::from((A.xyz().cross(B.xyz()).normalize(), 1.0));
//             vertices[*i[0] as usize].normal = normal;
//             vertices[*i[1] as usize].normal = normal;
//             vertices[*i[2] as usize].normal = normal;
//         });
//     });
// }

// fn load_daz_par(filepath: &PathBuf) {
//     let dsf = daz::import(filepath).unwrap();
//     dsf.geometry_library.iter().map(|geo| {

//         let index_buffer = geo.polylist.values.iter().fold(Vec::new(), |mut acc, quad| {
//             // reorder
//             match quad.len() {
//                 5 => {
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                 },
//                 6 => {
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[2].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[5].clone());
//                 }
//                 _ => {}
//             };
//             acc
//         });

//         let mut vertices = geo.vertices.values.par_iter().map(|v| {
//             ModelVertex {
//                 pos: glam::vec4(v[0], v[1], v[2], 1.0),
//                 normal: glam::vec4(0.0, 0.0, 0.0, 0.0),
//                 color: glam::vec4(1.0, 1.0, 1.0, 1.0),
//                 uv: glam::vec4(0.0, 0.0, 0.0, 0.0),
//             }
//         }).collect::<Vec<ModelVertex>>();
        
//         let sections = vec![PrimitiveSection {
//             index: 0,
//             vertices: BufferPart {
//                 offset: 0,
//                 element_count: vertices.len() as usize,
//             },
//             indices: Some(BufferPart {
//                 offset: 0,
//                 element_count: index_buffer.len() as usize,
//             }),
//             material_index: Some(0),
//         }];
//         let mut mesh = Mesh::new(geo.name.clone(), vertices, index_buffer, glam::Mat4::IDENTITY, sections);
        
//         let normals = mesh.vertex_iter()
//                 .map(|vertex_id| mesh.vertex_normal(vertex_id))
//                 .collect::<Vec<_>>();
        
//         mesh.vertices.iter_mut().zip(normals).for_each(|(vertex, normal)| {
//             vertex.normal = glam::vec4(normal.x, normal.y, normal.z, 0.0);
//         });
//         mesh
//     });
// }


// fn load_some(filepath: &PathBuf) -> Vec<Mesh> {
//     let dsf = daz::import(filepath).unwrap();
//     let meshes: Vec<Mesh> = dsf.geometry_library.iter().map(|geo| {

//         let index_buffer = geo.polylist.values.iter().fold(Vec::new(), |mut acc, quad| {
//             // reorder
//             match quad.len() {
//                 5 => {
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                 },
//                 6 => {
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[2].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[5].clone());
//                 }
//                 _ => {}
//             };
//             acc
//         });

//         let mut vertices = geo.vertices.values.par_iter().map(|v| {
//             ModelVertex {
//                 pos: glam::vec4(v[0], v[1], v[2], 1.0),
//                 normal: glam::vec4(0.0, 0.0, 0.0, 0.0),
//                 color: glam::vec4(1.0, 1.0, 1.0, 1.0),
//                 uv: glam::vec4(0.0, 0.0, 0.0, 0.0),
//             }
//         }).collect::<Vec<ModelVertex>>();
        
//         let sections = vec![PrimitiveSection {
//             index: 0,
//             vertices: BufferPart {
//                 offset: 0,
//                 element_count: vertices.len() as usize,
//             },
//             indices: Some(BufferPart {
//                 offset: 0,
//                 element_count: index_buffer.len() as usize,
//             }),
//             material_index: Some(0),
//         }];
//         let mut mesh = Mesh::new(geo.name.clone(), vertices, index_buffer, glam::Mat4::IDENTITY, sections);
        
//         let normals = mesh.vertex_iter()
//                 .map(|vertex_id| mesh.vertex_normal(vertex_id))
//                 .collect::<Vec<_>>();
        
//         mesh.vertices.iter_mut().zip(normals).for_each(|(vertex, normal)| {
//             vertex.normal = glam::vec4(normal.x, normal.y, normal.z, 0.0);
//         });
//         mesh
//     }).collect();
//     meshes
// }



// fn conversion(meshes: &Vec<Mesh>, context: Arc<Context>) {

//     let converted = meshes.iter().map(|m| m.to_vulkan_mesh(context.clone())).collect::<Vec<_>>();
//     //let x = converted[0].name.clone();
// }

// fn prepare_context(settings: AppSettings, event_loop: &EventLoop<()>) -> Arc<Context> {
//     let mut window = Window::new(
//         settings.resolution[0],
//         settings.resolution[1],
//         settings.name.clone(),
//         &event_loop,
//     );
//     let manager = bolt::resource::BindlessManager::new(&mut window, &settings.clone().manager, Some(settings.render));
//     let renderer = bolt::AppRenderer::new(&mut window, &manager);
//     renderer.context.clone()
    
// }

// fn criterion_benchmark(c: &mut Criterion) {
//     let mut group = c.benchmark_group("model_load");
//     group.sample_size(10);
//     let context = prepare_context(AppSettings::default(), &EventLoop::new());
//     let path = bolt::util::find_asset("models/Genesis9.dsf").unwrap();
//     let models = load_some(&path);
//     // c.bench_function("load_seq", |b| b.iter(|| {
//     //     load_daz(black_box(&path));
//     // }));
//     // c.bench_function("load_par", |b| b.iter(|| {
//     //     load_daz_par(black_box(&path));
//     // }));
//     group.bench_function("load_seq", |b| b.iter(|| {
//         conversion(black_box(&models), context.clone());
//     }));
// }

// criterion_group!(benches, criterion_benchmark);
// criterion_main!(benches);