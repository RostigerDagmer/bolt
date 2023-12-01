use std::{path::PathBuf, sync::Arc, ffi::CString};

use ash::vk;

use crate::{Context, Shader};

pub struct MeshPipelineInfo {
    pub layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
    pub mesh_shader: Option<PathBuf>,
    pub task_shader: Option<PathBuf>,
    pub name: String,
    pub primitive_topology: vk::PrimitiveTopology,
    pub specialization_data: Vec<u8>,
    pub specialization_entries: Vec<vk::SpecializationMapEntry>,
    pub samples: vk::SampleCountFlags,
    // Add more fields specific to mesh pipeline setup if necessary...
}

impl Default for MeshPipelineInfo {
    fn default() -> Self {
        Self {
            layout: vk::PipelineLayout::default(),
            render_pass: vk::RenderPass::null(),
            mesh_shader: None,
            task_shader: None,
            name: "".to_string(),
            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            specialization_data: Vec::new(),
            specialization_entries: Vec::new(),
            samples: vk::SampleCountFlags::TYPE_1,
        }
    }
}

impl MeshPipelineInfo {
    // Builder-style methods to set up the MeshPipelineInfo...
    // For example:
    pub fn layout(mut self, layout: vk::PipelineLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn render_pass(mut self, render_pass: vk::RenderPass) -> Self {
        self.render_pass = render_pass;
        self
    }   


    pub fn mesh_shader(mut self, mesh_shader: PathBuf) -> Self {
        self.mesh_shader = Some(mesh_shader);
        self
    }

    pub fn task_shader(mut self, task_shader: PathBuf) -> Self {
        self.task_shader = Some(task_shader);
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn primitive_topology(mut self, primitive_topology: vk::PrimitiveTopology) -> Self {
        self.primitive_topology = primitive_topology;
        self
    }

    pub fn specialization_data(mut self, specialization_data: Vec<u8>) -> Self {
        self.specialization_data = specialization_data;
        self
    }

    pub fn specialization_entries(mut self, specialization_entries: Vec<vk::SpecializationMapEntry>) -> Self {
        self.specialization_entries = specialization_entries;
        self
    }

    pub fn samples(mut self, samples: vk::SampleCountFlags) -> Self {
        self.samples = samples;
        self
    }

    pub fn build(self, context: Arc<Context>) -> MeshPipeline {
        MeshPipeline::new(context, self)
    }

}

pub struct MeshPipeline {
    context: Arc<Context>,
    info: MeshPipelineInfo,
    pipeline: vk::Pipeline,
}

impl MeshPipeline {
    pub fn new(context: Arc<Context>, mut info: MeshPipelineInfo) -> Self {
        // Compile shaders and get their modules.
        let mesh_shader_module = info.mesh_shader.as_ref().map(|path| {
            Shader::new(context.clone(), path.clone(), vk::ShaderStageFlags::MESH_NV)
        });
        let task_shader_module = info.task_shader.as_ref().map(|path| {
            Shader::new(context.clone(), path.clone(), vk::ShaderStageFlags::TASK_NV)
        });

        // Create shader stages.
        let mut shader_stage_create_infos = Vec::new();
        let shader_entry_name = CString::new("main").unwrap();

        if let Some(ref shader) = task_shader_module {
            shader_stage_create_infos.push(shader.get_create_info(&shader_entry_name));
        }
        
        if let Some(ref shader) = mesh_shader_module {
            shader_stage_create_infos.push(shader.get_create_info(&shader_entry_name));
        }

        // Set up other pipeline info including blend modes, rasterization and multisample state.
        // ...

        let create_infos = [vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            // Additional settings specific to mesh pipeline...
            .layout(info.layout)
            .render_pass(info.render_pass)
            .build()];

        let graphics_pipelines = unsafe {
            context
                .device()
                .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("Unable to create graphics pipeline")
        };

        // Return the new MeshPipeline object
        MeshPipeline {
            context,
            info,
            pipeline: graphics_pipelines[0],
        }
    }

    // ... other MeshPipeline methods ...

    // You will also want to implement update methods, disposal, etc., as needed.
}

// ... `Resource` trait implementation, `Drop` implementation, etc. ...