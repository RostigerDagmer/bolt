mod pipeline;
pub use pipeline::*;

mod acceleration;
pub use acceleration::*;

mod sbt;
pub use sbt::*;

use ash::vk;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{Context, Vertex};

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct SceneInstance {
    id: u32,
    texture_offset: u32,
    index_count: u32,
    dynamic: u32,
    transform: glam::Mat4,
    transform_it: glam::Mat4,
}

impl SceneInstance {
    pub fn update_transform(&mut self, transform: glam::Mat4) {
        self.transform = transform;
        self.transform_it = transform.inverse().transpose();
    }

    pub fn get_transform(&self) -> &glam::Mat4 {
        &self.transform
    }
}

// Scene description buffers used by the raytracing hit shader
pub struct SceneDescription {
    blas: Vec<BLAS>,
    tlas: TLAS,
    instances: Vec<SceneInstance>,
    instances_buffer: crate::Buffer,
    vertex_descriptors: Vec<vk::DescriptorBufferInfo>,
    index_descriptors: Vec<vk::DescriptorBufferInfo>,
    mat_descriptors: Vec<vk::DescriptorBufferInfo>,
    phx_descriptors: Vec<vk::DescriptorBufferInfo>,
    half_edges_descriptors: Vec<vk::DescriptorBufferInfo>,
    vertex_to_half_edge_descriptors: Vec<vk::DescriptorBufferInfo>,
    texture_descriptors: Vec<vk::DescriptorImageInfo>,
    blas_to_instances: HashMap<usize, Vec<usize>>,
}

impl SceneDescription {
    pub fn from_scene(context: Arc<Context>, scene: &crate::scene::Scene) -> Self {
        let meshes = scene.meshes.iter().map(|m| m.to_vulkan_mesh(context.clone())).collect::<Vec<_>>();
        let mut transforms = Vec::<glam::Mat4>::new();
        meshes.iter().for_each(|mesh| {
            transforms.push(mesh.transform);
        });
        Self::from_meshes(context, meshes.iter().map(|m| m).collect(), transforms, Some(&scene.material_buffer), Some(&scene.textures))
    }

    pub fn from_meshes(
        context: Arc<Context>,
        meshes: Vec<&crate::scene::VulkanMesh>,
        mesh_transforms: Vec<glam::Mat4>,
        material_buffer: Option<&crate::Buffer>,
        textures: Option<&Vec<crate::Texture2d>>
    ) -> Self {
        let cmd = context.begin_single_time_cmd();
        let mut blas = Vec::<BLAS>::new();
        let mut instances = Vec::<SceneInstance>::new();
        let mut vertex_descriptors = Vec::<vk::DescriptorBufferInfo>::new();
        let mut index_descriptors = Vec::<vk::DescriptorBufferInfo>::new();
        let mut mat_descriptors = Vec::<vk::DescriptorBufferInfo>::new();
        let mut phx_descriptors = Vec::<vk::DescriptorBufferInfo>::new();
        let mut half_edges_descriptors: Vec<vk::DescriptorBufferInfo> = Vec::new();
        let mut vertex_to_half_edge_descriptors: Vec<vk::DescriptorBufferInfo> = Vec::new();
        let mut blas_to_instances = HashMap::<usize, Vec<usize>>::new();

        // let min = context
        //     .get_physical_device_limits()
        //     .min_storage_buffer_offset_alignment;
        // println!("min storage align {:?}", min);

        meshes.iter().enumerate().for_each(|(i, mesh)| {
            for primitive in &mesh.primitive_sections {
                let mut geo_intances = Vec::<GeometryInstance>::new();
                let mut instance_indices = Vec::<usize>::new();
                
                let (index_buffer, index_count, index_offset_size) = match &mesh.index_buffer {
                    Some(buffer) => (
                        Some(buffer.get_device_address()),
                        Some(primitive.get_index_count()),
                        Some(primitive.get_index_offset_size::<u32>()),
                    ),
                    None => (None, None, None),
                };
                geo_intances.push(GeometryInstance {
                    vertex_buffer: mesh.vertex_buffer.get_device_address(),
                    vertex_count: primitive.get_vertex_count(),
                    vertex_offset_size: primitive.get_vertex_offset_size(),
                    vertex_offset: primitive.get_vertex_offset(),
                    index_buffer,
                    index_count,
                    index_offset_size,
                    transform: glam::Mat4::IDENTITY, //TODO: Does this work??
                });

                vertex_descriptors.push(primitive.get_vertex_descriptor(&mesh.vertex_buffer));
                match &mesh.phx {
                    Some(phx) => phx_descriptors.push(primitive.get_physics_descriptor(phx)),
                    None => {}
                }
                match &mesh.index_storage {
                    Some(buffer) => {
                        index_descriptors.push(primitive.get_index_descriptor::<u64>(buffer));
                    }
                    None => {}
                }
                match &material_buffer {
                    Some(buffer) => mat_descriptors.push(primitive.get_material_descriptor(buffer)),
                    None => {}
                };
                // let (half_edge_buffer, vertex_to_half_edge_buffer) = primitive
                //     .get_half_edge_descriptors(mesh.half_edge_buffer.as_ref().unwrap(), mesh.vertices_to_half_edges.as_ref().unwrap());
                // half_edges_descriptors.push(half_edge_buffer);
                // vertex_to_half_edge_descriptors.push(vertex_to_half_edge_buffer);
                
                let instance = SceneInstance {
                    id: instances.len() as u32,
                    transform: mesh_transforms[i],
                    transform_it: mesh_transforms[i].inverse().transpose(),
                    dynamic: 1, // TODO: add this info to the mesh or grab it from materials
                    index_count: index_count.unwrap_or(0) as u32,
                    texture_offset: 0 as u32,
                    ..Default::default()
                };
                instance_indices.push(instance.id as usize);
                instances.push(instance);

                // TODO: support multiple instances per BLAS (move out of primitive loop here)

                // Bottom-level acceleration structure
                blas.push(BLAS::new(
                    context.clone(),
                    cmd,
                    geo_intances,
                    mesh_transforms[i],
                    crate::scene::ModelVertex::stride() as u64,
                    true,
                ));
                blas_to_instances.insert(i as usize, instance_indices);
            }
        });

        let tlas = TLAS::new(context.clone(), cmd, &blas);
        context.end_single_time_cmd(cmd);

        let instances_buffer = crate::Buffer::from_data(
            context.clone(),
            crate::BufferInfo::default().cpu_to_gpu().usage_storage(),
            &instances,
        );
        let texture_descriptors = match textures {
            Some(textures) => {
                textures.iter().map(|t| {
                    t.get_descriptor_info()
                }).collect()
            }
            None => Vec::new()
        };
        
        SceneDescription {
            blas,
            tlas,
            instances,
            instances_buffer,
            vertex_descriptors,
            index_descriptors,
            phx_descriptors,
            half_edges_descriptors,
            vertex_to_half_edge_descriptors,
            mat_descriptors,
            texture_descriptors,
            blas_to_instances,
        }
    }

    pub fn tlas(&self) -> &TLAS {
        &self.tlas
    }

    pub fn blas_transform(&mut self, transform: glam::Mat4, index: usize) {
        self.blas[index].set_transform(transform);
        for instance_index in &self.blas_to_instances[&index] {
            self.instances[*instance_index].update_transform(transform);
        }
    }

    pub fn blas_transforms(&mut self, transforms: &[glam::Mat4]) {
        transforms
            .iter()
            .enumerate()
            .for_each(|(index, transform)| {
                self.blas_transform(transform.clone(), index);
            });
    }

    pub fn tlas_regenerate(&mut self, cmd: vk::CommandBuffer) {
        self.tlas
            .regenerate(cmd, &self.blas);
    }

    pub fn blas_regenerate(&mut self, cmd: vk::CommandBuffer, instances: impl Iterator<Item = (usize, impl Iterator<Item = GeometryInstance>)>) {
        instances.for_each(|(index, geos)| {
            self.blas[index].regenerate(cmd, geos);
        })
    }

    pub fn blas(&self) -> &Vec<BLAS> {
        &self.blas
    }

    pub fn get_instances_buffer(&self) -> &crate::Buffer {
        &self.instances_buffer
    }

    pub fn update(&mut self) {
        self.instances_buffer.update(&self.instances)
    }

    pub fn get_vertex_descriptors(&self) -> &Vec<vk::DescriptorBufferInfo> {
        &self.vertex_descriptors
    }

    pub fn get_index_descriptors(&self) -> &Vec<vk::DescriptorBufferInfo> {
        &self.index_descriptors
    }

    pub fn get_material_descriptors(&self) -> &Vec<vk::DescriptorBufferInfo> {
        &self.mat_descriptors
    }

    pub fn get_physics_descriptors(&self) -> &Vec<vk::DescriptorBufferInfo> {
        &self.phx_descriptors
    }

    pub fn get_half_edge_descriptors(&self) -> (&Vec<vk::DescriptorBufferInfo>, &Vec<vk::DescriptorBufferInfo>) {
        (&self.half_edges_descriptors, &self.vertex_to_half_edge_descriptors)
    }

    pub fn get_texture_descriptors(&self) -> &Vec<vk::DescriptorImageInfo> {
        &self.texture_descriptors
    }
}
