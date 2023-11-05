use std::{cell::{RefCell, Cell}, sync::{Arc, Mutex}};

use ash::vk;
use std::fmt;
use crate::{SharedContext, Resource};


#[derive(Debug)]
pub struct CommandPool {
    pool: vk::CommandPool,
    command_buffers: RefCell<Vec<vk::CommandBuffer>>,
    active_count: Cell<usize>,
}

impl CommandPool {
    pub fn new(device: Arc<ash::Device>, queue_family_index: u32, ) -> Self {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(queue_family_index);
        unsafe {
            let pool = device
                .create_command_pool(&pool_create_info, None)
                .unwrap();
            CommandPool {
                pool,
                command_buffers: RefCell::new(Vec::new()),
                active_count: Cell::new(0),
            }
        }
    }

    pub fn reset(&self, device: &ash::Device) {
        unsafe {
            device
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::default())
                .expect("Reset command buffer failed.");

            self.active_count.set(0);
        }
    }

    pub fn request_command_buffer(&self, device: &ash::Device) -> vk::CommandBuffer {
        let mut buffers = self.command_buffers.try_borrow_mut().unwrap();
        if self.active_count.get() < buffers.len() {
            let index = self.active_count.get();
            self.active_count.set(index + 1);
            return buffers[index];
        } else {
            unsafe {
                let create_info = vk::CommandBufferAllocateInfo::builder()
                    .command_buffer_count(1)
                    .command_pool(self.pool)
                    .level(vk::CommandBufferLevel::PRIMARY);
                let command_buffer = device
                    .allocate_command_buffers(&create_info)
                    .unwrap()[0];

                buffers.push(command_buffer.clone());
                return command_buffer;
            }
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_command_pool(self.pool, None);
        }
    }
}

impl Resource<vk::CommandPool> for CommandPool {
    fn handle(&self) -> vk::CommandPool {
        self.pool
    }
}


pub struct CommandManager {
    device: Arc<ash::Device>,
    command_pools: Arc<Mutex<Vec<CommandPool>>>,
    transient: vk::CommandPool,
}

impl fmt::Debug for CommandManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandManager")
            .field("device", &self.device.handle())
            .field("command_pools", &self.command_pools)
            .finish()
    }

}

impl CommandManager {
    pub fn new(context: Arc<SharedContext>, queue_family_index: u32) -> Self {

        let graphics_index = context.queue_family_indices.graphics;
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(graphics_index);
        unsafe {
            let transient = context
            .device()
                .create_command_pool(&pool_create_info, None)
                .unwrap();
            CommandManager {
                device: context.device().clone(),
                command_pools: Arc::new(Mutex::new(vec![CommandPool::new(
                    context.device(),
                    queue_family_index,
                )])),
                transient
            }
        }
    }

    pub fn create_pool(&self, queue_family_index: u32) {
        let mut command_pools = self.command_pools.lock().unwrap();
        command_pools.push(CommandPool::new(self.device.clone(), queue_family_index));
    }

    pub fn request_command_buffer(&self, frame_index: usize) -> vk::CommandBuffer {
        let mut command_pools = self.command_pools.lock().unwrap();
        if frame_index >= command_pools.len() {
            panic!("You didn't create enough command pools for the number of frames you're using.")
        }
        command_pools[frame_index].request_command_buffer(&self.device)
    }

    pub fn reset(&self) {
        self.command_pools.lock().unwrap().iter().for_each(|pool| {
            pool.reset(&self.device);
        });
    }

    pub fn transient(&self) -> vk::CommandPool {
        self.transient
    }
}

impl Drop for CommandManager {
    fn drop(&mut self) {
        for pool in self.command_pools.lock().unwrap().iter() {
            pool.reset(&self.device);
            pool.destroy(&self.device);
        }
    }
}


#[test]
fn command_manager_thread_safe() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<CommandManager>();
    assert_sync::<CommandManager>();
}