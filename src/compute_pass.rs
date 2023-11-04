use ash::vk;
use std::sync::Arc;
use crate::Context;

pub trait ComputePass {
    fn pass(self, cmd: vk::CommandBuffer, context: Arc<Context>);
}