mod pipeline;
pub use pipeline::*;

pub mod manager;
pub use manager::*;

pub mod structures;
pub use structures::*;

#[derive(Debug)]
pub struct SimulationPipeline {
    pub input_buffer_index: usize,
    pub build_plan: PipelineBuildPlan,
    pub output_buffer_index: usize,
}

#[derive(Clone, Copy)]
pub enum Format {
    Image,
    Geo,
}

impl std::fmt::Debug for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Image => write!(f, "Image"),
            Format::Geo => write!(f, "Geometry"),
        }
    }
}