use crate::*;
//use sim::SimulationManager;

use ash::{
    extensions::{ext::DebugUtils, khr},
    vk, Device, Entry, Instance, prelude::VkResult,
};
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::ffi::{CStr, CString};
use std::{
    collections::{HashSet},
    os::raw::c_char
};
use raw_window_handle::HasRawDisplayHandle;
use std::sync::{Arc, Mutex};
use log::*;

fn pick_physical_device_and_queue_family_indices(
    instance: &ash::Instance,
    extensions: &[&CStr],
) -> VkResult<Option<(vk::PhysicalDevice, u32)>> {
    Ok(unsafe { instance.enumerate_physical_devices() }?
        .into_iter()
        .find_map(|physical_device| {
            if unsafe { instance.enumerate_device_extension_properties(physical_device) }.map(
                |exts| {
                    let set: HashSet<&CStr> = exts
                        .iter()
                        .map(|ext| unsafe { CStr::from_ptr(&ext.extension_name as *const c_char) })
                        .collect();

                    extensions.iter().all(|ext| set.contains(ext))
                },
            ) != Ok(true)
            {
                return None;
            }

            let graphics_family =
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) }
                    .into_iter()
                    .enumerate()
                    .find(|(_, device_properties)| {
                        device_properties.queue_count > 0
                            && device_properties
                                .queue_flags
                                .contains(vk::QueueFlags::GRAPHICS)
                    });

            graphics_family.map(|(i, _)| (physical_device, i as u32))
        }))
}

fn find_queue_families(
    instance: &Instance,
    surface: &khr::Surface,
    surface_khr: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> (Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
    let mut graphics = None;
    let mut present = None;
    let mut compute = None;
    let mut transfer = None;

    let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
    for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
        let index = index as u32;

        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            && family.queue_flags.contains(vk::QueueFlags::COMPUTE)
            && graphics.is_none()
        {
            graphics = Some(index);
            compute = Some(index);
        }

        // if family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
        //     compute = Some(index)
        // }

        if family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
            transfer = Some(index)
        }

        let present_support = unsafe {
            surface
                .get_physical_device_surface_support(device, index, surface_khr)
                .expect("Failed to get surface support")
        };
        if present_support && present.is_none() {
            present = Some(index);
        }

        if graphics.is_some() && present.is_some() && compute.is_some() && transfer.is_some() {
            break;
        }
    }
    println!("transfer_idx: {:?}", transfer);
    println!("compute_idx: {:?}", compute);
    println!("graphics_idx: {:?}", graphics);
    println!("present_idx: {:?}", present);

    (transfer, graphics, compute, present)
}

fn create_logical_device_with_graphics_queue(
    instance: &Instance,
    device: vk::PhysicalDevice,
    queue_families_indices: QueueFamiliesIndices,
    device_extensions: &Vec<&'static CStr>,
) -> (Device, vk::Queue, vk::Queue) {
    let graphics_family_index = queue_families_indices.graphics;
    let present_family_index = queue_families_indices.present;
    let compute_family_index = queue_families_indices.compute;
    let transfer_family_index = queue_families_indices.transfer;
    let queue_priorities = [1.0f32];

    let queue_create_infos = {
        // Vulkan specs does not allow passing an array containing duplicated family indices.
        // And since the family for graphics and presentation could be the same we need to
        // deduplicate it.
        let mut indices = vec![graphics_family_index, present_family_index, compute_family_index, transfer_family_index];
        indices.dedup();

        // Now we build an array of `DeviceQueueCreateInfo`.
        // One for each different family index.
        indices
            .iter()
            .map(|index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*index)
                    .queue_priorities(&queue_priorities)
                    .build()
            })
            .collect::<Vec<_>>()
    };

    let supported_extensions: HashSet<String> = unsafe {
        let extension_properties = instance
            .enumerate_device_extension_properties(device).unwrap();
        extension_properties
            .iter()
            .map(|ext| {
                CStr::from_ptr(ext.extension_name.as_ptr() as *const c_char)
                    .to_string_lossy()
                    .as_ref()
                    .to_owned()
            })
            .collect()
    };

    let mut device_extensions_ptrs = vec![
        vk::ExtDescriptorIndexingFn::name().as_ptr(),
        vk::ExtScalarBlockLayoutFn::name().as_ptr(),
        vk::KhrMaintenance1Fn::name().as_ptr(),
        vk::KhrMaintenance2Fn::name().as_ptr(),
        vk::KhrMaintenance3Fn::name().as_ptr(),
        vk::KhrGetMemoryRequirements2Fn::name().as_ptr(),
        vk::KhrImagelessFramebufferFn::name().as_ptr(),
        vk::KhrImageFormatListFn::name().as_ptr(),
        vk::KhrDescriptorUpdateTemplateFn::name().as_ptr(),
        // Rust-GPU
        vk::KhrShaderFloat16Int8Fn::name().as_ptr(),
        // DLSS
        #[cfg(feature = "dlss")]
        {
            b"VK_NVX_binary_import\0".as_ptr() as *const i8
        },
        #[cfg(feature = "dlss")]
        {
            b"VK_KHR_push_descriptor\0".as_ptr() as *const i8
        },
        #[cfg(feature = "dlss")]
        vk::NvxImageViewHandleFn::name().as_ptr(),
    ];

    device_extensions_ptrs.push(ash::extensions::khr::Swapchain::name().as_ptr());

    let ray_tracing_extensions = [
        vk::KhrVulkanMemoryModelFn::name().as_ptr(), // used in ray tracing shaders
        vk::KhrPipelineLibraryFn::name().as_ptr(),   // rt dep
        vk::KhrDeferredHostOperationsFn::name().as_ptr(), // rt dep
        vk::KhrBufferDeviceAddressFn::name().as_ptr(), // rt dep
        vk::KhrAccelerationStructureFn::name().as_ptr(),
        vk::KhrRayTracingPipelineFn::name().as_ptr(),
    ];

    let ray_tracing_enabled = unsafe {
        ray_tracing_extensions.iter().all(|ext| {
            let ext = CStr::from_ptr(*ext).to_string_lossy();

            let supported = supported_extensions.contains(ext.as_ref());

            if !supported {
                dbg!("Ray tracing extension not supported: {}", ext);
            }

            supported
        })
    };

    if ray_tracing_enabled {
        dbg!("All ray tracing extensions are supported");
        device_extensions_ptrs.extend(ray_tracing_extensions.iter());
    }

    for ext in device_extensions {
        device_extensions_ptrs.push((*ext).as_ptr());
    }

    let device_features = vk::PhysicalDeviceFeatures::builder()
        .sampler_anisotropy(true)
        .shader_int64(true);

    let mut vulkan12_features = vk::PhysicalDeviceVulkan12Features::builder()
        .shader_subgroup_extended_types(true)
        .descriptor_indexing(true);

    let mut indexing_info = vk::PhysicalDeviceDescriptorIndexingFeatures::builder()
        .descriptor_binding_partially_bound(true)
        .runtime_descriptor_array(true)
        .build();

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions_ptrs)
        .enabled_features(&device_features)
        .push_next(&mut indexing_info)
        .push_next(&mut vulkan12_features);

    // Build device and queues
    let device = unsafe {
        instance
            .create_device(device, &device_create_info, None)
            .expect("Failed to create logical device.")
    };
    let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
    let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };

    (device, graphics_queue, present_queue)
}

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

pub struct SharedContext {
    entry: Entry,
    instance: Instance,
    debug_utils_loader: DebugUtils,
    debug_call_back: vk::DebugUtilsMessengerEXT,
    device: Device,
    pdevice: vk::PhysicalDevice,
    allocator: ManuallyDrop<Arc<Mutex<Allocator>>>,
    pub queue_family_indices: QueueFamiliesIndices,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
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
            .field("graphics_queue", &self.graphics_queue)
            .field("present_queue", &self.present_queue)
            .field("ray_tracing_properties", &self.ray_tracing_properties)
            .finish()
    }
}

// Debug Callback
extern "system" fn vulkan_debug_utils_callback(
    message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> u32 {
    let severity = if message_severity
        .contains(ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO)
    {
        log::Level::Info
    } else if message_severity
        .contains(ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING)
    {
        log::Level::Warn
    } else if message_severity
        .contains(ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
    {
        log::Level::Error
    } else {
        log::Level::Debug
    };
    let types = if message_types
        .contains(ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL)
    {
        "General"
    } else if message_types
        .contains(ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION)
    {
        "Validation"
    } else if message_types
        .contains(ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
    {
        "Performance"
    } else {
        "Unknown"
    };
    let message = unsafe {
        let data = *p_callback_data;
        format!(
            "[{}]: {:?}",
            types,
            CStr::from_ptr(data.p_message)
        )
    };
    log!(severity, "{}", &message);
    ash::vk::FALSE
}

impl SharedContext {
    pub fn new(window: &mut Window, settings: &RendererSettings) -> Self {

        // println!("RendererSettings: {:#?}", settings);
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
                        // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                )
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    // | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
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

            SharedContext {
                entry,
                instance,
                debug_utils_loader,
                debug_call_back,
                device,
                pdevice,
                allocator: ManuallyDrop::new(Arc::new(Mutex::new(allocator))),
                queue_family_indices,
                graphics_queue,
                present_queue,
                frames_in_flight,
                acceleration_structure,
                ray_tracing,
                ray_tracing_properties,
            }
        }
    }

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

    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
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

#[derive(Debug)]
pub struct Context {
    shared_context: Arc<SharedContext>,
    frame_command_pools: Vec<CommandPool>,
    transient_command_pool: vk::CommandPool,
}

impl Context {
    pub fn new(shared_context: Arc<SharedContext>, swapchain_image_count: usize) -> Self {
        unsafe {
            let mut frame_command_pools = Vec::<CommandPool>::new();
            let graphics_index = shared_context.queue_family_indices.graphics;
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
        self.shared_context.entry()
    }

    pub fn instance(&self) -> &Instance {
        self.shared_context.instance()
    }

    pub fn device(&self) -> &Device {
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
        self.shared_context.present_queue()
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.shared_context.graphics_queue()
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
