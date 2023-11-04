pub mod connectivity;
pub mod indexing;

pub use connectivity::*;
pub use indexing::*;

use crate::{offset_of, Buffer, Context, Resource, Vertex, BufferInfo};
use crate::resource::material::MaterialInfo;
use ash::{vk};
use glam::Vec4Swizzles;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use rayon::prelude::*;

//TODO: solve non-vec4-aligned issues..
#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct ModelVertex {
    pub pos: glam::Vec4,
    pub color: glam::Vec4,
    pub normal: glam::Vec4,
    pub uv: glam::Vec4,
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct PhysicsPointProperties {
    pub mass: f32,
    pub velocity: glam::Vec3,
    pub radius: f32,
    pub acceleration: glam::Vec3,
}


impl Default for ModelVertex {
    fn default() -> Self {
        ModelVertex {
            pos: glam::vec4(0f32, 0.0, 0.0, 1.0),
            color: glam::Vec4::splat(1.0),
            normal: glam::Vec4::ZERO,
            uv: glam::Vec4::ZERO,
        }
    }
}

impl Default for PhysicsPointProperties {
    fn default() -> Self {
        PhysicsPointProperties {
            mass: 0.1,
            velocity: glam::Vec3::ZERO,
            radius: 0.01,
            acceleration: glam::Vec3::ZERO,
        }
    }
}

impl Vertex for ModelVertex {
    fn stride() -> u32 {
        std::mem::size_of::<ModelVertex>() as u32
    }
    fn format_offset() -> Vec<(vk::Format, u32)> {
        vec![
            (
                vk::Format::R32G32B32A32_SFLOAT,
                offset_of!(ModelVertex, pos) as u32,
            ),
            (
                vk::Format::R32G32B32A32_SFLOAT,
                offset_of!(ModelVertex, color) as u32,
            ),
            (
                vk::Format::R32G32B32A32_SFLOAT,
                offset_of!(ModelVertex, normal) as u32,
            ),
            (
                vk::Format::R32G32B32A32_SFLOAT,
                offset_of!(ModelVertex, uv) as u32,
            ),
        ]
    }
}

pub struct VulkanMesh {
    pub context: Arc<Context>,
    pub name: String,
    pub vertex_buffer: Buffer,
    pub index_buffer: Option<Buffer>,
    pub index_storage: Option<Buffer>,
    pub half_edge_buffer: Option<Buffer>,
    pub vertices_to_half_edges: Option<Buffer>,
    pub skin: Option<Buffer>,
    pub phx: Option<Buffer>,
    pub transform: glam::Mat4,
    pub primitive_sections: Vec<PrimitiveSection>,
}

unsafe impl Send for VulkanMesh {}
unsafe impl Sync for VulkanMesh {}

impl VulkanMesh {

    pub fn new(context: Arc<Context>,
        name: String,
        vertices: Vec<ModelVertex>,
        indices: Vec<u32>,
        half_edges: Vec<glam::UVec2>,
        vertices_to_half_edges: Vec<u32>,
        transform: glam::Mat4,
        primitive_sections: Vec<PrimitiveSection>,
    ) -> Self {
            let mut index_buffer = None;
            let mut index_storage = None;

            if !indices.is_empty() {
                index_buffer = Some(Buffer::from_data(
                    context.clone(),
                    BufferInfo::default().usage_index().gpu_only(),
                    &indices,
                ));
    
                let storage_indices: Vec<u64> = indices.iter().map(|i| *i as u64).collect();
                index_storage = Some(Buffer::from_data(
                    context.clone(),
                    BufferInfo::default().usage_storage().gpu_only(),
                    &storage_indices,
                ));
            }
            let vertex_buffer = Buffer::from_data(
                context.clone(),
                BufferInfo::default()
                    .usage_vertex()
                    .usage_storage()
                    .gpu_only(),
                &vertices,
            );
            // let point_properties = vertices
            //     .iter()
            //     .map(|v| PhysicsPointProperties::default())
            //     .collect::<Vec<_>>();
            // let point_properties = Buffer::from_data(
            //     context.clone(),
            //     BufferInfo::default()
            //         .usage_storage()
            //         .gpu_only(),
            //     &point_properties,
            // );

            // let half_edge_buffer = Buffer::from_data(
            //     context.clone(),
            //     BufferInfo::default()
            //         .usage_storage()
            //         .gpu_only(),
            //     &half_edges,
            // );

            // let vertices_to_half_edges = Buffer::from_data(
            //     context.clone(),
            //     BufferInfo::default()
            //         .usage_storage()
            //         .gpu_only(),
            //     &vertices_to_half_edges,
            // );

            VulkanMesh {
                context,
                name,
                index_buffer,
                index_storage,
                half_edge_buffer: None, // Some(half_edge_buffer),
                vertices_to_half_edges: None, // Some(vertices_to_half_edges),
                vertex_buffer,
                skin:None,
                phx: None, // Some(point_properties),
                transform,
                primitive_sections,
            }

    }

    pub fn cmd_draw(&self, cmd: vk::CommandBuffer) {
        let device = self.context.device();
        unsafe {
            match &self.index_buffer {
                Some(indices) => {
                    for section in &self.primitive_sections {
                        device.cmd_bind_vertex_buffers(
                            cmd,
                            0,
                            &[self.vertex_buffer.handle()],
                            &[section.get_vertex_offset_size()],
                        );
                        device.cmd_bind_index_buffer(
                            cmd,
                            indices.handle(),
                            section.get_index_offset_size::<u32>(),
                            vk::IndexType::UINT32,
                        );
                        device.cmd_draw_indexed(cmd, section.get_index_count(), 1, 0, 0, 1);
                    }
                }
                None => {
                    for section in &self.primitive_sections {
                        device.cmd_bind_vertex_buffers(
                            cmd,
                            0,
                            &[self.vertex_buffer.handle()],
                            &[section.get_vertex_offset_size()],
                        );
                        device.cmd_draw(cmd, section.get_vertex_count(), 1, 0, 1);
                    }
                }
            }
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub struct PrimitiveSection {
    pub index: usize,
    pub vertices: BufferPart,
    pub indices: Option<BufferPart>,
    pub material_index: Option<usize>,
    //aabb: AABB<f32>,
}

impl PrimitiveSection {
    pub fn get_index_descriptor<T>(&self, buffer: &Buffer) -> vk::DescriptorBufferInfo {
        let size = std::mem::size_of::<T>() as u64;
        buffer.get_descriptor_info_offset(
            self.indices.unwrap().offset as u64 * size,
            self.indices.unwrap().element_count as u64 * size,
        )
    }

    pub fn get_vertex_descriptor(&self, buffer: &Buffer) -> vk::DescriptorBufferInfo {
        let size = std::mem::size_of::<ModelVertex>() as u64;
        buffer.get_descriptor_info_offset(
            self.vertices.offset as u64 * size,
            self.vertices.element_count as u64 * size,
        )
    }

    pub fn get_material_descriptor(&self, buffer: &Buffer) -> vk::DescriptorBufferInfo {
        let size = std::mem::size_of::<MaterialInfo>() as u64;
        buffer.get_descriptor_info_offset(self.material_index.unwrap() as u64 * size, size)
    }

    pub fn get_physics_descriptor(&self, buffer: &Buffer) -> vk::DescriptorBufferInfo {
        let size = std::mem::size_of::<PhysicsPointProperties>() as u64;
        buffer.get_descriptor_info_offset(
            self.vertices.offset as u64 * size,
            self.vertices.element_count as u64 * size,
        )
    }
    pub fn get_half_edge_descriptors(&self, he_buff: &Buffer, ve_to_he_buff: &Buffer) -> (vk::DescriptorBufferInfo, vk::DescriptorBufferInfo) {
        let size = std::mem::size_of::<glam::UVec2>() as u64;
        let half_edge_descriptor = he_buff.get_descriptor_info_offset(
            (self.vertices.offset * 2) as u64 * size,
            (self.vertices.element_count * 2) as u64 * size,
        );
        let size = std::mem::size_of::<u32>() as u64;
        let vertices_to_half_edge_descriptor = ve_to_he_buff.get_descriptor_info_offset(
            0 as u64 * size,
            self.vertices.element_count as u64 * size,
        );
        (half_edge_descriptor, vertices_to_half_edge_descriptor)
    }

    pub fn get_vertices(&self) -> &BufferPart {
        &self.vertices
    }

    pub fn get_vertex_count(&self) -> u32 {
        self.vertices.element_count as u32
    }

    pub fn get_vertex_offset(&self) -> u32 {
        self.vertices.offset as u32
    }

    pub fn get_vertex_offset_size(&self) -> vk::DeviceSize {
        let size = std::mem::size_of::<ModelVertex>() as u64;
        self.vertices.offset as u64 * size
    }

    pub fn get_indices(&self) -> &Option<BufferPart> {
        &self.indices
    }

    pub fn get_index_count(&self) -> u32 {
        self.indices.unwrap().element_count as u32
    }

    pub fn get_index_offset_size<T>(&self) -> vk::DeviceSize {
        let size = std::mem::size_of::<T>() as u64;
        self.indices.unwrap().offset as u64 * size
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BufferPart {
    pub offset: usize,
    pub element_count: usize,
}

pub struct Mesh {
    pub name: String,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub transform: glam::Mat4,
    pub primitive_sections: Vec<PrimitiveSection>,
    pub connectivity_info: ConnectivityInfo
}

impl Mesh {

    pub fn new(name: String, vertices: Vec<ModelVertex>, indices: Vec<u32>, transform: glam::Mat4, primitive_sections: Vec<PrimitiveSection>) -> Self {
        
        let no_vertices = vertices.len();
        let no_faces = indices.len() / 3;
        let positions = vertices.iter().map(|v| v.pos.xyz()).collect::<Vec<_>>();
        let connectivity_info: ConnectivityInfo = ConnectivityInfo::new(no_vertices, no_faces);
        
        let mesh = Mesh {
            name,
            vertices,
            indices,
            transform,
            primitive_sections,
            connectivity_info
        };
        

        // Create vertices
        positions.iter().for_each(|pos| {
            mesh.connectivity_info.new_vertex(*pos);
        });

        let mut twins = HashMap::<(VertexID, VertexID), HalfEdgeID>::new();
        fn sort(a: VertexID, b: VertexID) -> (VertexID, VertexID) {
            if a < b {
                (a, b)
            } else {
                (b, a)
            }
        }

        // Create faces and twin connectivity
        for face in mesh.indices.par_iter().collect::<Vec<_>>().chunks(3) {
            let v0 = face[0];
            let v1 = face[1];
            let v2 = face[2];

            let face = mesh.connectivity_info.create_face(
                unsafe { VertexID::new(*v0) },
                unsafe { VertexID::new(*v1) },
                unsafe { VertexID::new(*v2) },
            );

            // mark twin halfedges
            let mut walker = mesh.walker_from_face(face);
            for _ in 0..3 {
                let vertex_id = walker.vertex_id().unwrap();
                walker.as_next();
                let key = sort(vertex_id, walker.vertex_id().unwrap());
                if let Some(twin) = twins.get(&key) {
                    mesh.connectivity_info
                        .set_halfedge_twin(walker.halfedge_id().unwrap(), *twin);
                } else {
                    twins.insert(key, walker.halfedge_id().unwrap());
                }
            }
        }
        mesh.connectivity_info.halfedge_iterator().for_each(|halfedge| {
            if mesh
                .connectivity_info
                .halfedge(halfedge)
                .unwrap()
                .twin
                .is_none()
            {
                let vertex = mesh
                    .walker_from_halfedge(halfedge)
                    .as_previous()
                    .vertex_id()
                    .unwrap();
                mesh.connectivity_info.set_halfedge_twin(
                    halfedge,
                    mesh.connectivity_info
                        .new_halfedge(Some(vertex), None, None),
                );
            }
        });
        mesh
    }

    pub fn to_vulkan_mesh(&self, context: Arc<Context>) -> VulkanMesh {
        let first_id: VertexID;
        unsafe { first_id = VertexID::new(*self.indices.first().unwrap()); };
        let half_edges = self.vertex_halfedge_iter(first_id).filter_map(|id| {
            let he = self.connectivity_info.halfedge(id).unwrap();
            match (he.next, he.twin) {
                (Some(he1), Some(he2)) => Some(glam::UVec2::new(*he1.deref(), *he2.deref())),
                (None, None) => None,
                (None, Some(twin)) => Some(glam::UVec2::new(0, *twin.deref())),
                (Some(next), None) => panic!("impossibruh")
            }
        }).collect::<Vec<glam::UVec2>>();
        let vertices_to_half_edges = self.vertex_halfedge_iter(first_id).map(|id| {
            *id.deref()
        }).collect::<Vec<u32>>();
        VulkanMesh::new(
            context.clone(),
            self.name.clone(),
            self.vertices.clone(),
            self.indices.clone(),
            half_edges,
            vertices_to_half_edges,
            self.transform,
            self.primitive_sections.clone(),
        )
    }

    pub fn vertex_position(&self, vertex_id: VertexID) -> glam::Vec3 {
        self.connectivity_info.position(vertex_id)
    }

    /// Returns the number of vertices in the mesh.
    pub fn no_vertices(&self) -> usize {
        self.connectivity_info.no_vertices()
    }

    /// Returns the number of edges in the mesh.
    pub fn no_edges(&self) -> usize {
        self.connectivity_info.no_halfedges() / 2
    }

    /// Returns the number of half-edges in the mesh.
    pub fn no_halfedges(&self) -> usize {
        self.connectivity_info.no_halfedges()
    }

    /// Returns the number of faces in the mesh.
    pub fn no_faces(&self) -> usize {
        self.connectivity_info.no_faces()
    }

    pub fn face_direction(&self, face_id: FaceID) -> glam::Vec3 {
        let mut walker = self.walker_from_face(face_id);
        let p0 = self.vertex_position(walker.vertex_id().unwrap());
        walker.as_next();
        let v0 = self.vertex_position(walker.vertex_id().unwrap()) - p0;
        walker.as_next();
        let v1 = self.vertex_position(walker.vertex_id().unwrap()) - p0;

        v0.cross(v1)
    }

    pub fn half_edges(&self) -> impl Iterator<Item = HalfEdgeID> {
        self.connectivity_info.halfedge_iterator()
    }

    /// Returns the normal of the face.
    pub fn face_normal(&self, face_id: FaceID) -> glam::Vec3 {
        self.face_direction(face_id).normalize()
    }

    pub fn vertex_normal(&self, vertex_id: VertexID) -> glam::Vec3 {
        let mut normal = glam::Vec3::ZERO;
        for halfedge_id in self.vertex_halfedge_iter(vertex_id) {
            if let Some(face_id) = self.walker_from_halfedge(halfedge_id).face_id() {
                normal += self.face_normal(face_id)
            }
        }
        normal.normalize()
    }

}