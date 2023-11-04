use ash::vk;
use std::sync::Arc;
use std::path::PathBuf;

use crate::context::Context;
//use crate::shader::Shader;
use crate::pipeline::Shader;
use crate::Resource;
use std::ffi::CString;

pub struct ComputePipeline {
    context: Arc<Context>,
    shaders: Vec<Shader>,
    info: ComputePipelineInfo,
    pipelines: Vec<vk::Pipeline>,
    create_infos: Vec<vk::ComputePipelineCreateInfo>,
}

impl ComputePipeline {
    pub fn new(context: Arc<Context>, info: ComputePipelineInfo) -> Self {

        let shader_entry_name = CString::new("main").unwrap();

        let shaders = info.shaders.iter().cloned().map(|(path, stage_flags)| {
            Shader::new(context.clone(), path, stage_flags)
        })
        .collect::<Vec<_>>();

        let create_infos: Vec<vk::ComputePipelineCreateInfo> = shaders.iter().map(| shader | {
            let shader_stage_create_info = if info.specialization_entries.is_empty() {
                shader.get_create_info(&shader_entry_name)
            } else {
                shader.get_create_info_with_specialization(
                    &shader_entry_name,
                    &vk::SpecializationInfo::builder()
                        .map_entries(&info.specialization_entries)
                        .data(&info.specialization_data),
                )
            };
            println!("shader create info: {:?}", shader_stage_create_info);
            vk::ComputePipelineCreateInfo::builder()
            .stage(shader_stage_create_info)
            .layout(info.layout)
            .build()
        })
        .collect::<Vec<_>>();

        let pipelines = unsafe {
            context
                .device()
                .create_compute_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Failed to create compute pipeline")
        };
    
        ComputePipeline {
            shaders,
            create_infos,
            context,
            info,
            pipelines,
        }
    }
}

impl Resource<vk::Pipeline> for ComputePipeline {
    fn handle(&self) -> vk::Pipeline {
        panic!("ComputePipeline::handle() should not be called because they intrinsically have multiple pipelines");
    }
    fn handles(&self) -> Option<&Vec<vk::Pipeline>> {
        Some(&self.pipelines)
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.pipelines.iter().for_each(|&pipeline| {
                self.context.device().destroy_pipeline(pipeline, None);
            });
        }
    }
}

pub struct ComputePipelineInfo {
    pub layout: vk::PipelineLayout,
    pub shaders: Vec<(PathBuf, vk::ShaderStageFlags)>,
    pub specialization_data: Vec<u8>,
    pub specialization_entries: Vec<vk::SpecializationMapEntry>,
}

impl Default for ComputePipelineInfo {
    fn default() -> Self {
        ComputePipelineInfo {
            layout: vk::PipelineLayout::default(),
            shaders: vec![], // ("assets/glsl/id.comp".to_string().into(), vk::ShaderStageFlags::COMPUTE)
            specialization_data: Vec::new(),
            specialization_entries: Vec::new(),
        }
    }
}

impl ComputePipelineInfo {
    pub fn layout(mut self, layout: vk::PipelineLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn shader(mut self, path: PathBuf, stage_flags: vk::ShaderStageFlags) -> Self {
        self.shaders.push((path, stage_flags));
        self
    }

    pub fn comp(mut self, path: PathBuf) -> Self {
        self.shaders.push((path, vk::ShaderStageFlags::COMPUTE));
        self
    }

    pub fn specialization<T>(mut self, data: &T, constant_id: u32) -> Self {
        let slice = unsafe {
            std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of_val(data))
        };
        self.specialization_data = slice.to_vec();
        self.specialization_entries.push(
            vk::SpecializationMapEntry::builder()
                .constant_id(constant_id)
                .offset(0)
                .size(self.specialization_data.len())
                .build(),
        );
        self
    }
}