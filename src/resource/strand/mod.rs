use std::sync::Arc;

use ash::vk;

use crate::{Buffer, offset_of, Vertex, Context, BufferInfo, sim};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SimParams {
    pub friction: f32,
    pub volume_preservation: f32,
    pub bending_stiffness: f32,
    pub stretching_stiffness: f32,
    pub local_shape: f32,
    pub global_length: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            friction: 0.2,
            volume_preservation: 0.95,
            bending_stiffness: 0.1,
            stretching_stiffness: 0.1,
            local_shape: 0.0,
            global_length: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Strand {
    pub offset: u32,
    pub count: u32,
    pub uv: glam::Vec2,
    pub barycentric: glam::Vec3,
    pub root_radius: f32,
    pub tip_radius: f32,
    // pub is_simstrand: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StrandVertex {
    pub position: glam::Vec3,
    pub color: glam::Vec4,
}

// what copilot suggested here is actually better than the current trait
// impl Vertex for StrandVertex {
//     fn binding_description() -> vk::VertexInputBindingDescription {
//         vk::VertexInputBindingDescription {
//             binding: 0,
//             stride: std::mem::size_of::<Self>() as u32,
//             input_rate: vk::VertexInputRate::VERTEX,
//         }
//     }
//     fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
//         vec![
//             vk::VertexInputAttributeDescription {
//                 location: 0,
//                 binding: 0,
//                 format: vk::Format::R32G32B32_SFLOAT,
//                 offset: offset_of!(Self, position) as u32,
//             },
//             vk::VertexInputAttributeDescription {
//                 location: 1,
//                 binding: 0,
//                 format: vk::Format::R32G32B32A32_SFLOAT,
//                 offset: offset_of!(Self, color) as u32,
//             },
//         ]
//     }
// }

#[derive(Clone, Debug)]
pub struct Strands {
    pub strands: Vec<Strand>,
    pub sim_params: SimParams,
    pub vertices: Vec<StrandVertex>,
    pub indices: Vec<u32>,
}

pub struct VulkanStrands {
    pub strands: Buffer,
    pub sim_params: Buffer,
    pub vertices: Buffer,
    pub indices: Buffer,
}

impl VulkanStrands {
    pub fn new(context: Arc<Context>, strands: &Strands) -> Self {
        println!("strands: {:?}", strands.strands.len());
        println!("vertices: {:?}", strands.vertices.len());
        println!("indices: {:?}", strands.indices.len());
        println!("sim_params: {:?}", strands.sim_params);
        let info = BufferInfo::default()
        .usage_storage();
        let strand_buffer = Buffer::from_data(
            context.clone(),
            info.name("Strands"),
            &strands.strands,
        );
        
        let sim_params = Buffer::from_data(
            context.clone(),
            info.name("Sim Params"),
            &[strands.sim_params],
        );
        
        let vertices = Buffer::from_data(
            context.clone(),
            info.name("Strand Vertices"),
            &strands.vertices,
        );

        let indices = Buffer::from_data(
            context.clone(),
            info.name("Strand Indices"),
            &strands.indices,
        );
        println!("passed...");
        Self {
            strands: strand_buffer,
            sim_params,
            vertices,
            indices,
        }
    }
}

