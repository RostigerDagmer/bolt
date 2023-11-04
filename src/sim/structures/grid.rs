

// build plan for a grid buffer

use ash::vk;
use std::default::Default;

pub struct Grid <T> {
    pub dim: usize,
    pub resolution: f32,
    pub origin: Vec<f32>,
    pub size: Vec<f32>,
    pub data: Vec<T>,
}

impl<T: Default + Clone> Grid<T> {
    pub fn new(dim: usize, resolution: f32, origin: Vec<f32>, size: Vec<f32>) -> Self {
        let len = size.iter().fold(0, |acc, x| acc + (x / resolution) as usize);
        let empty = vec![T::default(); len];
        Grid {
            dim,
            resolution,
            origin,     //.try_into().unwrap_or_else("Expected a vector of <{:?}> got: {:?}", dim, size.len()),
            size,       //.try_into().unwrap_or_else("Failed to write Grid size. Expected a vector of length <{:?}> got: {:?}", dim, size.len()),
            data: empty,      
        }
    }

    pub fn get_descriptor_binding(&self) -> vk::DescriptorSetLayoutBinding {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build()
    }
}