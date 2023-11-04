use ash::{
    extensions::{ext::DebugUtils, khr},
    vk, Device, Entry, Instance,
};
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::HasRawDisplayHandle;
use std::mem::ManuallyDrop;
use std::ffi::CString;

use std::sync::{Arc, Mutex};

use crate::{QueueFamiliesIndices, Window, RendererSettings};
use super::*;


#[derive(Debug)]
pub struct QueueManager {
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
}

impl QueueManager {
    pub fn new(
        graphics_queue: vk::Queue,
        present_queue: vk::Queue,
    ) -> Self {
        QueueManager {
            graphics_queue,
            present_queue,
        }
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }
}   

pub struct SharedContext {
    entry: Entry,
    instance: Instance,
    debug_utils_loader: DebugUtils,
    debug_call_back: vk::DebugUtilsMessengerEXT,
    device: Device,
    pdevice: vk::PhysicalDevice,
    allocator: ManuallyDrop<Arc<Mutex<Allocator>>>,
    pub queue_family_indices: QueueFamiliesIndices,
    pub frames_in_flight: usize,
    pub acceleration_structure: khr::AccelerationStructure,
    pub ray_tracing: khr::RayTracingPipeline,
    pub ray_tracing_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
}

impl std::fmt::Debug for SharedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedContext")
            .field("debug_call_back", &self.debug_call_back)
            .field("pdevice", &self.pdevice)
            .field("allocator", &self.allocator)
            .field("ray_tracing_properties", &self.ray_tracing_properties)
            .finish()
    }
}

pub fn create_shared_context_and_queue_manager(
    window: &mut Window,
    settings: &RendererSettings,
) -> (Arc<Mutex<SharedContext>>, QueueManager) {
    unsafe {
        let entry = Entry::load().unwrap();

        let app_name = CString::new("VulkanTriangle").unwrap();

        let mut layer_names = Vec::<CString>::new();
        if cfg!(debug_assertions) {
            layer_names.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
            layer_names.push(CString::new("VK_LAYER_LUNARG_api_dump").unwrap());
        }
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions =
            ash_window::enumerate_required_extensions(window.handle().raw_display_handle()).unwrap();
        
        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|ext| *ext)
            .collect::<Vec<_>>();
        extension_names_raw.push(DebugUtils::name().as_ptr());

        for ext in &settings.extensions {
            let some = ext.as_ref().as_ptr();
            extension_names_raw.push(some);
        }
        
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(vk::API_VERSION_1_3);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance: Instance = entry
            .create_instance(&create_info, None)
            .expect("Instance creation error");

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback));
        let debug_utils_loader = DebugUtils::new(&entry, &instance);
        let debug_call_back = debug_utils_loader
            .create_debug_utils_messenger(&debug_info, None)
            .unwrap();
        // if settings.debug {
        //     setup_debug_utils(&entry, &instance)
        // }

        window.create_surface(&entry, &instance);

        // let pdevices = instance
        //     .enumerate_physical_devices()
        //     .expect("Physical device error");
        
        let (pdevice, _) = pick_physical_device_and_queue_family_indices(
            &instance,
            &[
                ash::extensions::khr::AccelerationStructure::name(),
                ash::extensions::khr::DeferredHostOperations::name(),
                ash::extensions::khr::RayTracingPipeline::name(),
                ash::extensions::nv::DeviceDiagnosticCheckpoints::name(),
            ],
        )
        .unwrap()
        .unwrap();

        let (transfer, graphics, compute, present) = find_queue_families(
            &instance,
            window.surface_loader(),
            window.surface(),
            pdevice,
        );
        let queue_family_indices = QueueFamiliesIndices {
            transfer: transfer.unwrap(),
            compute: compute.unwrap(),
            graphics: graphics.unwrap(),
            present: present.unwrap(),
        };
        let (device, graphics_queue, present_queue) = create_logical_device_with_graphics_queue(
            &instance,
            pdevice,
            queue_family_indices,
            &settings.device_extensions,
        );

        let allocator = Allocator::new(&AllocatorCreateDesc{
            instance: instance.clone(),
            device: device.clone(),
            physical_device: pdevice,
            debug_settings: Default::default(),
            buffer_device_address: true,  // TODO: check the BufferDeviceAddressFeatures struct.
        }).unwrap();

        let acceleration_structure = khr::AccelerationStructure::new(&instance, &device);
        let ray_tracing = khr::RayTracingPipeline::new(&instance, &device);
        let ray_tracing_properties = khr::RayTracingPipeline::get_properties(&instance, pdevice);
        let frames_in_flight = settings.frames_in_flight;

        let shared_context = SharedContext {
            entry,
            instance,
            debug_utils_loader,
            debug_call_back,
            device,
            pdevice,
            allocator: ManuallyDrop::new(Arc::new(Mutex::new(allocator))),
            queue_family_indices,
            frames_in_flight,
            acceleration_structure,
            ray_tracing,
            ray_tracing_properties,
        };

        let queue_manager = QueueManager {
            graphics_queue,
            present_queue,
        };
        (Arc::new(Mutex::new(shared_context)), queue_manager)
    }
}

impl SharedContext {

    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.pdevice
    }

    pub fn get_physical_device_properties(&self) -> vk::PhysicalDeviceProperties {
        unsafe { self.instance.get_physical_device_properties(self.pdevice) }
    }

    pub fn get_physical_device_limits(&self) -> vk::PhysicalDeviceLimits {
        self.get_physical_device_properties().limits
    }

    pub fn allocator(&self) -> &Arc<Mutex<Allocator>> {
        &self.allocator
    }

    pub fn acceleration_structure(&self) -> &khr::AccelerationStructure {
        &self.acceleration_structure
    }

    pub fn ray_tracing(&self) -> &khr::RayTracingPipeline {
        &self.ray_tracing
    }

    pub unsafe fn ray_tracing_properties(&self) -> &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR {
        &self.ray_tracing_properties
    }

    pub fn queue_family_indices(&self) -> &QueueFamiliesIndices {
        &self.queue_family_indices
    }
}

impl Drop for SharedContext {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.allocator); // Explicitly drop before destruction of device and instance.
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}