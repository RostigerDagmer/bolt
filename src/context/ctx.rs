use crate::*;
//use sim::SimulationManager;

use ash::{
    extensions::khr,
    vk, Device, Entry, Instance,
};
use gpu_allocator::vulkan::Allocator;
use std::sync::{Arc, Mutex};
use super::SharedContext;

#[derive(Clone, Copy)]
pub struct QueueFamiliesIndices {
    pub transfer: u32,
    pub compute: u32,
    pub graphics: u32,
    pub present: u32,
}

impl QueueFamiliesIndices {
    pub fn as_array(&self) -> [u32; 4] {
        [self.transfer, self.compute, self.graphics, self.present]
    }
}

#[derive(Debug)]
pub struct Context {
    shared_context: Arc<Mutex<SharedContext>>,
    queue_manager: QueueManager,
    command_manager: CommandManager,
    frame_command_pools: Vec<CommandPool>,
    transient_command_pool: vk::CommandPool,
}

impl Context {
    pub fn new(shared_context: Arc<Mutex<SharedContext>>, swapchain_image_count: usize) -> Self {
        unsafe {
            let mut frame_command_pools = Vec::<CommandPool>::new();
            let graphics_index = shared_context.lock().unwrap().queue_family_indices.graphics;
            for _ in 0..swapchain_image_count {
                frame_command_pools.push(CommandPool::new(shared_context.clone(), graphics_index));
            }

            let pool_create_info = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(graphics_index);
            let transient_command_pool = shared_context
                .device()
                .create_command_pool(&pool_create_info, None)
                .unwrap();
            Context {
                shared_context,
                frame_command_pools,
                transient_command_pool,
            }
        }
    }

    pub fn entry(&self) -> &Entry {
        self.shared_context.lock().unwrap().entry()
    }

    pub fn instance(&self) -> &Instance {
        self.shared_context.lock().unwrap().instance()
    }

    pub fn device(&self) -> &Device {
        self.shared_context.lock().unwrap().device()
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.shared_context.lock().unwrap().physical_device()
    }

    pub fn get_physical_device_properties(&self) -> vk::PhysicalDeviceProperties {
        self.shared_context.lock().unwrap().get_physical_device_properties()
    }

    pub fn get_physical_device_limits(&self) -> vk::PhysicalDeviceLimits {
        self.shared_context.lock().unwrap().get_physical_device_limits()
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.queue_manager.present_queue()
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.queue_manager.graphics_queue()
    }

    pub fn allocator(&self) -> &Arc<Mutex<Allocator>> {
        self.shared_context.lock().unwrap().allocator()
    }

    pub fn acceleration_structure(&self) -> &khr::AccelerationStructure {
        self.shared_context.lock().unwrap().acceleration_structure()
    }

    pub fn ray_tracing(&self) -> &khr::RayTracingPipeline {
        self.shared_context.lock().unwrap().ray_tracing()
    }

    pub unsafe fn ray_tracing_properties(&self) -> &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR {
        self.shared_context.lock().unwrap().ray_tracing_properties()
    }

    pub fn shared(&self) -> &Arc<Mutex<SharedContext>> {
        &self.shared_context
    }

    pub fn begin_single_time_cmd(&self) -> vk::CommandBuffer {
        let create_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.transient_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        unsafe {
            let command_buffer = self
                .device()
                .allocate_command_buffers(&create_info)
                .unwrap()[0];
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device()
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();

            command_buffer
        }
    }

    pub fn end_single_time_cmd(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device().end_command_buffer(command_buffer).unwrap();

            let command_buffers = vec![command_buffer];
            let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffers);
            self.device()
                .queue_submit(
                    self.graphics_queue(),
                    &[submit_info.build()],
                    vk::Fence::null(),
                )
                .expect("queue submit failed.");

            self.device()
                .queue_wait_idle(self.graphics_queue())
                .unwrap();
            self.device()
                .free_command_buffers(self.transient_command_pool, &command_buffers)
        }
    }

    pub fn request_command_buffer(&self, frame_index: usize) -> vk::CommandBuffer {
        self.frame_command_pools[frame_index].reset();
        self.frame_command_pools[frame_index].request_command_buffer()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.device()
                .destroy_command_pool(self.transient_command_pool, None);
            self.frame_command_pools.clear();
        }
    }
}
