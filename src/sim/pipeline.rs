use crate::context::Context;
use ash::{vk};
use std::{path::PathBuf, sync::Arc};
use super::Format;

#[derive(Debug)]
pub struct PipelineBuildPlan {
    pub layout: vk::PipelineLayout,
    pub shaders: Vec<(PathBuf, vk::ShaderStageFlags)>,
    pub name: String,
    pub specialization_data: Vec<u8>,
    pub specialization_entries: Vec<vk::SpecializationMapEntry>,
    pub input_format: Format,
    pub output_format: Format,
}

impl Default for PipelineBuildPlan {
    fn default() -> Self {
        PipelineBuildPlan {
            layout: vk::PipelineLayout::default(),
            shaders: Vec::new(),
            name: "".to_string(),
            specialization_data: Vec::new(),
            specialization_entries: Vec::new(),
            input_format: Format::Geo,
            output_format: Format::Geo,
        }
    }
}