use crate::{Resource, SharedContext};
use ash::{vk};
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

// Based on: https://github.com/KhronosGroup/Vulkan-Samples/blob/master/framework/semaphore_pool.h
pub struct SemaphorePool {
    shared_context: Arc<SharedContext>,
    semaphores: Vec<vk::Semaphore>,
    active_count: usize,
}

impl SemaphorePool {
    pub fn new(shared_context: Arc<SharedContext>) -> Self {
        SemaphorePool {
            shared_context,
            semaphores: Vec::new(),
            active_count: 0,
        }
    }

    pub fn request_semaphore(&mut self) -> vk::Semaphore {
        if self.active_count < self.semaphores.len() {
            let index = self.active_count;
            self.active_count = self.active_count + 1;
            return self.semaphores[index];
        } else {
            unsafe {
                let semaphore_create_info = vk::SemaphoreCreateInfo::default();
                let semaphore = self
                    .shared_context
                    .device()
                    .create_semaphore(&semaphore_create_info, None)
                    .unwrap();

                self.semaphores.push(semaphore.clone());
                return semaphore;
            }
        }
    }

    pub fn get_active_count(&self) -> usize {
        self.active_count
    }

    pub fn reset(&mut self) {
        self.active_count = 0;
    }
}

impl Drop for SemaphorePool {
    fn drop(&mut self) {
        self.reset();
        unsafe {
            self.semaphores.iter().for_each(|s| {
                self.shared_context.device().destroy_semaphore(*s, None);
            });
        }
    }
}