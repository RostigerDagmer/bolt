use std::{sync::Arc, hash::Hash, collections::HashMap};

use ash::vk;
use glam::Mat4;
use gpu_allocator::MemoryLocation;

use crate::{Buffer, Context, BufferInfo, scene::daz::format::{RigV1, BoneV1}};
mod rig;
pub use rig::*;

#[derive(Debug, Clone, Copy)]
pub struct SkinJoint {
    pub joint_id: u32,
    pub vertex_id: u32,
    pub weight: f32,
}

pub struct Skin {
    pub name: String,
    pub transforms: Vec<Mat4>,
    pub global_bone_transforms: Vec<Mat4>,
    pub inverse_bind_matrices: Vec<Mat4>,
    pub joints: Vec<SkinJoint>,
    pub joint_id_map: HashMap<String, u32>,
}

impl Skin {
    pub fn transforms_from<R: Rig<S, T>, S: Transform, T: Bone<S>>(&mut self, rig: &R) {
        self.transforms = rig.get_bones().iter().map(|bone| {
            bone.get_local_transform().get_matrix()
        }).collect();

        self.global_bone_transforms = (0..rig.get_bones().len()).into_iter().map(|i| {
            rig.local_to_global(i).get_matrix()
        }).collect();
    }
}

pub struct VulkanSkin {
    pub name: String,
    pub transforms: Buffer,
    pub joints: Buffer,
    pub global_bone_transforms: Buffer,
    pub inverse_bind_matrices: Buffer,
}


impl VulkanSkin {
    // TODO: make this generic over <T: Into<Skin>>
    pub fn from_data(context: Arc<Context>, name: String, skin: &Skin) -> Self {

        let transforms = Buffer::from_data(
                context.clone(),
                BufferInfo {
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                    mem_usage: MemoryLocation::CpuToGpu,
                    memory_type_bits: Some((vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT).as_raw()),
                    ..Default::default()
                },
            &skin.transforms,
        );
        let joints = Buffer::from_data(
                context.clone(),
                BufferInfo {
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                    mem_usage: MemoryLocation::CpuToGpu,
                    memory_type_bits: Some((vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT).as_raw()),
                    ..Default::default()
                },
            &skin.joints,
        );

        let global_bone_transforms = Buffer::from_data(
                context.clone(),
                BufferInfo {
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                    mem_usage: MemoryLocation::CpuToGpu,
                    memory_type_bits: Some((vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT).as_raw()),
                    ..Default::default()
                },
            &skin.global_bone_transforms,
        );

        let inverse_bind_matrices = Buffer::from_data(
                context.clone(),
                BufferInfo {
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER,
                    mem_usage: MemoryLocation::GpuOnly,
                    ..Default::default()
                },
            &skin.inverse_bind_matrices,
        );

        VulkanSkin {
            name,
            transforms,
            joints,
            global_bone_transforms,
            inverse_bind_matrices,
        }
    }

    pub fn update(&self, skin: &Skin) {
        self.transforms.update(&skin.transforms);
        self.global_bone_transforms.update(&skin.global_bone_transforms);
        self.joints.update(&skin.joints);
    }

}
