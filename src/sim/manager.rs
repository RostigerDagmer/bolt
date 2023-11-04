use std::any::Any;
use std::collections::HashMap;

use super::{SimulationPipeline, PipelineBuildPlan};
use super::structures::Structure;


// managment structure that holds a graph of Simulation Pipelines and can construct vulkan objects that run the graph.
pub struct SimulationManager {
    pub pipelines: Vec<SimulationPipeline>,
    pub structures: HashMap<String, Box<dyn Structure>>,
    pub input_format: super::Format,
    pub output_format: super::Format, // usually a imageView that can be presented on the swapchain
}


impl SimulationManager {
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
            structures: HashMap::new(),
            input_format: super::Format::Geo,
            output_format: super::Format::Image,
        }
    }

    pub fn register(&mut self, pipeline_plan: PipelineBuildPlan, structure: Box<dyn Structure>) {
        self.output_format = pipeline_plan.output_format;
        if self.pipelines.len() == 0 {
            self.input_format = pipeline_plan.input_format;
        }
        // construct the pipeline from buildplan
        let pipeline = SimulationPipeline {
            input_buffer_index: 0,
            build_plan: pipeline_plan,
            output_buffer_index: 0,
        };
        let name = pipeline.build_plan.name.clone();
        self.pipelines.push(pipeline);
        self.structures.insert(name, structure);
    }
}

impl std::fmt::Debug for SimulationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SimulationManager")
         .field(&self.pipelines)
         .field(&self.input_format)
         .field(&self.output_format)
         .finish()
    }
}