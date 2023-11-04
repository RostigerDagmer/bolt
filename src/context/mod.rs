pub mod ctx;
pub mod shared;

pub use ctx::*;
pub use shared::*;

use log::*;

use ash::{
    extensions::khr,
    vk, Device, Instance, prelude::VkResult,
};

use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;

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