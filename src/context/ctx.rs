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

const COMMAND_POOL_COUNT: usize = 2;

#[derive(Debug)]
pub struct Context {
    shared_context: Arc<SharedContext>,
    queue_manager: QueueManager,
    command_manager: CommandManager,
}

impl Context {
    pub fn new(shared_context: Arc<SharedContext>, queue_manager: QueueManager, swapchain_image_count: usize) -> Self {

        let graphics_index = shared_context.queue_family_indices.graphics;
        let command_manager = CommandManager::new(shared_context.clone(), graphics_index);
        
        for _ in 0..swapchain_image_count {
            command_manager.create_pool(graphics_index);
        }

        unsafe {
            Context {
                shared_context,
                queue_manager,
                command_manager,
            }
        }
    }

    pub fn entry(&self) -> &Entry {
        self.shared_context.entry()
    }

    pub fn instance(&self) -> &Instance {
        self.shared_context.instance()
    }

    pub fn device(&self) -> Arc<Device> {
        self.shared_context.device()
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.shared_context.physical_device()
    }

    pub fn get_physical_device_properties(&self) -> vk::PhysicalDeviceProperties {
        self.shared_context.get_physical_device_properties()
    }

    pub fn get_physical_device_limits(&self) -> vk::PhysicalDeviceLimits {
        self.shared_context.get_physical_device_limits()
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.queue_manager.present_queue()
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.queue_manager.graphics_queue()
    }

    pub fn allocator(&self) -> &Arc<Mutex<Allocator>> {
        self.shared_context.allocator()
    }

    pub fn acceleration_structure(&self) -> &khr::AccelerationStructure {
        self.shared_context.acceleration_structure()
    }

    pub fn ray_tracing(&self) -> &khr::RayTracingPipeline {
        self.shared_context.ray_tracing()
    }

    pub unsafe fn ray_tracing_properties(&self) -> &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR {
        self.shared_context.ray_tracing_properties()
    }

    pub fn shared(&self) -> &Arc<SharedContext> {
        &self.shared_context
    }

    pub fn begin_single_time_cmd(&self) -> vk::CommandBuffer {
        let create_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_manager.transient())
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
                .free_command_buffers(self.command_manager.transient(), &command_buffers)
        }
    }

    pub fn request_command_buffer(&self, frame_index: usize) -> vk::CommandBuffer {
        // self.frame_command_pools[frame_index].reset();
        // self.frame_command_pools[frame_index].request_command_buffer()
        self.command_manager.request_command_buffer(frame_index)
    }
}

// impl Drop for Context {
//     fn drop(&mut self) {
//         unsafe {
//             self.device()
//                 .destroy_command_pool(self.transient_command_pool, None);
//             self.frame_command_pools.clear();
//         }
//     }
// }


// #[test]
// fn command_manager_thread_safe() {
//     fn assert_send<T: Send>() {}
//     fn assert_sync<T: Sync>() {}

//     assert_send::<Context>();
//     assert_sync::<Context>();
// }