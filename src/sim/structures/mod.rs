pub mod grid;
pub use grid::*;
use ash::vk::{DescriptorType, ShaderStageFlags, DescriptorSetLayout, DescriptorSetLayoutCreateInfo};

use crate::context;


pub struct StructureBuildPlan {
    pub name: String,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: usize,
    pub shader_stage: ShaderStageFlags,
    pub descriptor_set_layout: DescriptorSetLayoutCreateInfo,
}


pub trait Structure {
    fn get_build_plan(&self) -> StructureBuildPlan;
    fn stride(&self) -> usize;
}


impl <T: Default + Clone> Structure for Grid<T> {
    fn get_build_plan(&self) -> StructureBuildPlan {
        let create_info = DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[self.get_descriptor_binding()])
            .build();
        StructureBuildPlan {
            name: "Grid".to_string(),
            descriptor_type: DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
            shader_stage: ShaderStageFlags::COMPUTE,
            descriptor_set_layout: create_info,
        }
    }
    fn stride(&self) -> usize {
        std::mem::size_of::<T>()
    }
}

