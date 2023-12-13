pub mod connectivity;
pub mod indexing;

pub use connectivity::*;
pub use indexing::*;

use crate::{offset_of, Buffer, Context, Resource, Vertex, BufferInfo};
use crate::resource::material::MaterialInfo;
use ash::{vk};
use glam::Vec4Swizzles;
use std::cmp::Reverse;
use std::collections::{HashMap, BinaryHeap, HashSet};
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
pub struct DebugVertex {
    pub pos: glam::Vec4,
    pub color: glam::Vec4,
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

impl Vertex for DebugVertex {
    fn stride() -> u32 {
        std::mem::size_of::<DebugVertex>() as u32
    }
    fn format_offset() -> Vec<(vk::Format, u32)> {
        vec![
            (
                vk::Format::R32G32B32A32_SFLOAT,
                offset_of!(DebugVertex, pos) as u32,
            ),
            (
                vk::Format::R32G32B32A32_SFLOAT,
                offset_of!(DebugVertex, color) as u32,
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
    pub connectivity_info: ConnectivityInfo,
    connectivity_inv_map: HashMap<VertexID, usize> // VertexID -> offset into vertices
}

impl Mesh {

    pub fn new(name: String, vertices: Vec<ModelVertex>, indices: Vec<u32>, transform: glam::Mat4, primitive_sections: Vec<PrimitiveSection>) -> Self {
        
        let no_vertices = vertices.len();
        let no_faces = indices.len() / 3;
        let positions = vertices.iter().map(|v| v.pos.xyz()).collect::<Vec<_>>();
        let connectivity_info: ConnectivityInfo = ConnectivityInfo::new(no_vertices, no_faces);
        let mut inv_map = HashMap::<VertexID, usize>::new();
        
        let mut mesh = Mesh {
            name,
            vertices,
            indices,
            transform,
            primitive_sections,
            connectivity_info,
            connectivity_inv_map: inv_map
        };
        

        // Create vertices
        positions.iter().enumerate().for_each(|(index, pos)| {
            let id = mesh.connectivity_info.new_vertex(*pos);
            mesh.connectivity_inv_map.insert(id, index);
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

    pub fn model_vertex_from_vertex_id(&self, vertex_id: VertexID) -> ModelVertex {
        let offset = self.connectivity_inv_map.get(&vertex_id).unwrap();
        *self.vertices.get(*offset).unwrap()
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

    pub fn faces(&self) -> impl Iterator<Item = FaceID> {
        self.connectivity_info.face_iterator()
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

    pub fn face_area(&self, face_id: FaceID) -> f32 {
        let mut walker = self.walker_from_face(face_id);
        let p0 = self.vertex_position(walker.vertex_id().unwrap());
        walker.as_next();
        let p1 = self.vertex_position(walker.vertex_id().unwrap());
        walker.as_next();
        let p2 = self.vertex_position(walker.vertex_id().unwrap());

        let v0 = p1 - p0;
        let v1 = p2 - p0;

        v0.cross(v1).length() / 2.0
    }

    pub fn lower_lod(&self) -> Self {
        // take a triangle and its neighbouring 3 triangles and collapse it into one triangle (3 vertices)
        // mark all involved outer edges as collapsed and inner edges (part of the taken triangle) as removed
        // project involved points not part of the new face onto the new triangle edges and push them to the front of a queue.
        // should be 3 points since we just collapsed 6 vertices into 3.
        // choose the next point part of a triangle with no collapsed edges and repeat until queue is empty.

        // Create a priority queue based on the triangle's area or error metric.
        // The priority queue will ensure that we process the largest triangles first.
        let mut face_queue = BinaryHeap::<Reverse<(i32, FaceID)>>::new();
        let mut collapsed_set = HashSet::<HalfEdgeID>::new();
        let mut removed_set = HashSet::<HalfEdgeID>::new();
        
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut primitive_sections = Vec::new();
        let mut vertex_map = HashMap::<VertexID, glam::Vec3>::new();
        
        let mut connectivity_info = self.connectivity_info.clone();
        
        // Initialize the priority queue with all faces.
        for face_id in connectivity_info.face_iterator() {
            let area = self.face_area(face_id);
            face_queue.push(Reverse((area as i32, face_id)));
        }

        let get_e1 = |v| {
            let mut w = self.walker_from_vertex(v);
            w.as_previous().as_twin().as_previous().halfedge_id()
        };
        let get_e2 = |v| {
            let mut w = self.walker_from_vertex(v);
            w.as_twin().as_previous().halfedge_id()
        };

        let get_next_face_from_e2 = |e| {
            let mut w = self.walker_from_halfedge(e);
            w.as_previous().as_twin().face_id()
        };
        let get_next_face_from_e1 = |e| {
            let mut w = self.walker_from_halfedge(e);
            w.as_next().as_twin().face_id()
        };

        let mut index_counter = 0;

        // Begin processing faces from the queue.
        while let Some(Reverse((_, face_id))) = face_queue.pop() {
            // Check if current face edges have already been collapsed.
            if should_process_face(&collapsed_set, &removed_set, &self, face_id) {
                // Identify the vertices of the current triangle.
                let mut walker = self.walker_from_face(face_id);
                let vertices_ids = [
                    walker.vertex_id().unwrap(),
                    walker.as_next().vertex_id().unwrap(),
                    walker.as_next().vertex_id().unwrap(),
                ];
                
                // get neighbouring faces from the halfedges
                // get new midpoint vertex positions
                // get new triangle vertices
                /*
                ______v3_____
                 \  h /\  g /
                  \  / f\  /
                 v1\/____\/v2
                    \  j /
                     \  /
                      \/
                 */
                vertices_ids.iter().try_for_each(|v| {
                    let he = self.walker_from_vertex(*v).halfedge_id();
                    let e1 = get_e1(*v);
                    let e2 = get_e2(*v);
                    println!("he: {:?}, e1: {:?}, e2: {:?}", he, e1, e2);
                    match (e1, e2) {
                        (None, _) => return None,
                        (_, None) => return None,
                        _ => ()
                    };
                    let e1 = e1.unwrap();
                    let e2 = e2.unwrap();
                    let he = he.unwrap();
                    let v1 = self.walker_from_halfedge(e1).vertex_id().unwrap();
                    let v2 = self.walker_from_halfedge(e2).as_next().vertex_id().unwrap();
                    let midpoint = (self.connectivity_info.position(v1) +
                    self.connectivity_info.position(v2)) / 2.0;
                    collapsed_set.insert(e1);
                    collapsed_set.insert(e2);
                    removed_set.insert(he);
                    vertex_map.insert(*v, midpoint);
                    match get_next_face_from_e1(e1) {
                        Some(f) => face_queue.push(Reverse((self.face_area(f) as i32, f))),
                        None => (),
                    }
                    println!("face_queue: {:?}", face_queue);
                    vertices.extend_from_slice(&[v1].map(|v| self.model_vertex_from_vertex_id(v)));
                    indices.push(index_counter);
                    index_counter += 1;
                    Some(())
                });

            }
        }

        Mesh::new(self.name.clone(), vertices, indices, self.transform, primitive_sections)
    }

}

fn should_process_face(
    collapsed_set: &HashSet<HalfEdgeID>,
    removed_set: &HashSet<HalfEdgeID>,
    mesh: &Mesh,
    face_id: FaceID,
) -> bool {
    let mut walker = mesh.walker_from_face(face_id);
    for _ in 0..3 {
        let halfedge_id = walker.halfedge_id().unwrap();
        let twin = walker.twin_id().unwrap();
        if collapsed_set.contains(&halfedge_id) || removed_set.contains(&halfedge_id) || collapsed_set.contains(&twin) || removed_set.contains(&twin) {
            println!("skip: {:?}", face_id);
            return false;
        }
        walker.as_next();
    }
    true
}

pub fn test_icosahedron() -> Mesh {
    let vertices = vec![
        ModelVertex {
            pos: glam::Vec4::new(0.000, 0.000, 1.000, 1.0),
            color: glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
            normal: glam::Vec4::new(0.000, 0.000, 1.000, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(0.894, 0.000, 0.447, 1.0),
            color: glam::Vec4::new(0.0, 1.0, 0.0, 1.0),
            normal: glam::Vec4::new(0.894, 0.000, 0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(0.276, 0.851, 0.447, 1.0),
            color: glam::Vec4::new(0.0, 0.0, 1.0, 1.0),
            normal: glam::Vec4::new(0.276, 0.851, 0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(-0.724, 0.526, 0.447, 1.0),
            color: glam::Vec4::new(1.0, 1.0, 0.0, 1.0),
            normal: glam::Vec4::new(-0.724, 0.526, 0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(-0.724, -0.526, 0.447, 1.0),
            color: glam::Vec4::new(1.0, 0.0, 1.0, 1.0),
            normal: glam::Vec4::new(-0.724, -0.526, 0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(0.276, -0.851, 0.447, 1.0),
            color: glam::Vec4::new(0.0, 1.0, 1.0, 1.0),
            normal: glam::Vec4::new(0.276, -0.851, 0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(0.724, 0.526, -0.447, 1.0),
            color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0),
            normal: glam::Vec4::new(0.724, 0.526, -0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(-0.276, 0.851, -0.447, 1.0),
            color: glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            normal: glam::Vec4::new(-0.276, 0.851, -0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
        ModelVertex {
            pos: glam::Vec4::new(-0.894, 0.000, -0.447, 1.0),
            color: glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            normal: glam::Vec4::new(-0.894, 0.000, -0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },

        ModelVertex {
            pos: glam::Vec4::new(-0.276, -0.851, -0.447, 1.0),
            color: glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            normal: glam::Vec4::new(-0.276, -0.851, -0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },

        ModelVertex {
            pos: glam::Vec4::new(0.724, -0.526, -0.447, 1.0),
            color: glam::Vec4::new(0.0, 0.0, 0.0, 1.0),
            normal: glam::Vec4::new(0.724, -0.526, -0.447, 0.0),
            uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0),
        },
    ];

    let indices = vec![
        0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5, // front
        1, 6, 7, 1, 7, 2, 2, 7, 8, 2, 8, 3, // right
        3, 8, 9, 3, 9, 4, 4, 9, 10, 4, 10, 5, // top
        5, 10, 6, 5, 6, 1, 1, 7, 6, 6, 10, 9, 6, 9, 8, 6, 8, 7, // bottom
        5, 1, 0, 5, 0, 4, 4, 0, 3, 3, 0, 2, // left
        10, 9, 8, 10, 8, 7, // back
    ];

    let transform = glam::Mat4::IDENTITY;

    let primitive_sections = vec![PrimitiveSection {
        index: 0,
        vertices: BufferPart {
            offset: 0,
            element_count: vertices.len(),
        },
        indices: Some(BufferPart {
            offset: 0,
            element_count: indices.len(),
        }),
        material_index: None,
    }];

    Mesh::new("Icosahedron".to_string(), vertices, indices, transform, primitive_sections)
}


#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec4;
    use glam::Vec3;


    fn test_cube() -> Mesh {
        let vertices = vec![
            ModelVertex {
                pos: Vec4::new(-1.0, -1.0, -1.0, 1.0),
                color: Vec4::new(1.0, 0.0, 0.0, 1.0),
                normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(1.0, -1.0, -1.0, 1.0),
                color: Vec4::new(0.0, 1.0, 0.0, 1.0),
                normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(1.0, 1.0, -1.0, 1.0),
                color: Vec4::new(0.0, 0.0, 1.0, 1.0),
                normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(-1.0, 1.0, -1.0, 1.0),
                color: Vec4::new(1.0, 1.0, 0.0, 1.0),
                normal: Vec4::new(0.0, 0.0, -1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(-1.0, -1.0, 1.0, 1.0),
                color: Vec4::new(1.0, 0.0, 1.0, 1.0),
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(1.0, -1.0, 1.0, 1.0),
                color: Vec4::new(0.0, 1.0, 1.0, 1.0),
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(1.0, 1.0, 1.0, 1.0),
                color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
            ModelVertex {
                pos: Vec4::new(-1.0, 1.0, 1.0, 1.0),
                color: Vec4::new(0.0, 0.0, 0.0, 1.0),
                normal: Vec4::new(0.0, 0.0, 1.0, 0.0),
                uv: Vec4::new(0.0, 0.0, 0.0, 0.0),
            },
        ];

        let indices = vec![
            0, 1, 2, 2, 3, 0, // front
            1, 5, 6, 6, 2, 1, // right
            3, 2, 6, 6, 7, 3, // top
            4, 5, 1, 1, 0, 4, // bottom
            4, 0, 3, 3, 7, 4, // left
            7, 6, 5, 5, 4, 7, // back
        ];

        let transform = glam::Mat4::IDENTITY;

        let primitive_sections = vec![PrimitiveSection {
            index: 0,
            vertices: BufferPart {
                offset: 0,
                element_count: vertices.len(),
            },
            indices: Some(BufferPart {
                offset: 0,
                element_count: indices.len(),
            }),
            material_index: None,
        }];

        Mesh::new("cube".to_string(), vertices, indices, transform, primitive_sections)
    }

    #[test]
    fn test_vertex_position() {
        let mesh = test_cube(); // You need to implement a new() function for Mesh
        let vertex_id = unsafe {
            VertexID::new(0) // You need to implement a new() function for VertexID
        };
        let position = mesh.vertex_position(vertex_id);
        assert_eq!(position, Vec3::new(0.0, 0.0, 0.0)); // Replace with expected position
    }

    #[test]
    fn test_vertex_normal() {
        let mesh = test_cube();
        let vertex_id = unsafe {
            VertexID::new(0)
        };
        let normal = mesh.vertex_normal(vertex_id);
        assert_eq!(normal, Vec3::new(0.0, 0.0, 0.0)); // Replace with expected normal
    }

    #[test]
    fn test_face_area() {
        let mesh = test_cube();
        let face_id = unsafe {
            FaceID::new(0) // You need to implement a new() function for FaceID
        };
        let area = mesh.face_area(face_id);
        println!("area: {}", area);
        assert_eq!(area, 2.0); // one cube side is 2x2 so one half of a side is 2x1
    }

    #[test]
    fn test_lower_lod() {
        let mesh = test_icosahedron();
        println!("vertices: {:?}", mesh.no_vertices());
        let lower_lod = mesh.lower_lod();
        println!("vertices: {:?}", lower_lod.vertices);
        assert_eq!(lower_lod.no_vertices(), 3); // Replace with expected number of vertices
        assert_eq!(lower_lod.no_faces(), 1); // Replace with expected number of faces
    }
}


/*
                let mut new_positions = Vec::with_capacity(3);
                // collapsed_vertices: vertices of the new larger triangle
                let collapsed_vertices = halfedges.iter().filter_map(|&he| {
                    // twin -> previous is a corner of the larger new triangle
                    // next -> twin -> previous is the the vertex spanning the edge that contains the vertex of next
                    // we need to do this 3 times
                    let walker = self.walker_from_halfedge(he);
                    // store the vertex id of the current vertex
                    let v = walker.vertex_id().unwrap();
                    let v1_id = walker.as_twin().as_previous().vertex_id();
                    let collapsed_edge = walker.halfedge_id().unwrap();
                    // mark the edge as collapsed
                    collapsed_set.insert(collapsed_edge);
                    // go back to initial position
                    walker.as_next().as_twin();
                    let v2_id = walker.as_next().as_twin().as_previous().vertex_id();
                    let collapsed_edge = walker.halfedge_id().unwrap();
                    // mark the edge as collapsed
                    collapsed_set.insert(collapsed_edge);
                    // look up the two vertex positions
                    let v1 = self.vertex_position(v1_id);
                    let v2 = self.vertex_position(v2_id);
                    // calulate the midpoint
                    let midpoint = (v1 + v2) / 2.0;
                    new_positions.push(midpoint);
                    return Some(v);
                }).collect::<Vec<_>>();

*/