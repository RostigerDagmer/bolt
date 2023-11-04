// use std::{collections::HashMap, path::PathBuf, any::Any, sync::Arc, ffi::CStr};
// use ash::{vk::{self, Handle}, extensions::khr};
// use image::{ImageError, error::DecodingError};
// use crate::{context::Context, Window, SharedContext, RendererSettings, Resource};
// use crate::texture::{Image2d, Texture2d};
// use crate::resource::{material::{Material, MaterialError, MaterialInfo}, mesh::Mesh};
// use crate::resource::image::{Image, ToRawData};
// use crate::resource::mesh::VulkanMesh;
// use crate::buffer::*;
// use safe_transmute::{transmute_to_bytes, TriviallyTransmutable};
// use std::convert::AsRef;

// extern crate num_cpus;

// #[derive(Debug, Clone)]
// struct AllocationInfo<'a> {
//     buffer_info: BufferInfo<'a>, // buffer instantiation settings
//     size: u64, // size in byte
// }

// #[derive(Debug, Clone)]
// pub struct ManagerBaseSettings {
//     pub parallel_scaling: f32,
//     pub frames_in_flight: usize,
//     pub extensions: Vec<&'static CStr>,
//     pub device_extensions: Vec<&'static CStr>,
//     default_allocation: Vec<AllocationInfo<'static>>,
// }

// impl Default for ManagerBaseSettings {
//     fn default() -> Self {
//         ManagerBaseSettings {
//             parallel_scaling: 0.5,
//             frames_in_flight: 2,
//             extensions: vec![],
//             device_extensions: vec![],
//             default_allocation: vec![AllocationInfo {
//                 buffer_info: BufferInfo::std_storage(),
//                 size: 1024 * 1024 * 128, // 128MB
//             }]
//         }
//     }
// }

// /* allows for the creation of any resource that any sim
//  * will need to run. This includes buffers, textures, etc.
//  * There can be multiple ResourceManagers but it is preferred to use
//  * only one because it can facilitate optimization.
//  */
// pub struct BindlessManager {
//     pub shared_context: Arc<SharedContext>,
//     pub render_settings: RendererSettings,

//     pub command: Command,
//     pub resource_pools: HashMap<vk::MemoryPropertyFlags, Vec<vk::DescriptorPool>>,

//     textures_buffer: Vec<Buffer>, // frames_in_flight * large_buffer get suballocation info in one call from get_descriptors
//     materials_buffer: Vec<Buffer>, 
//     per_draw_buffer: Vec<Buffer>,
//     skin_buffer: Vec<Buffer>,
//     transforms_buffer: Vec<Buffer>,
//     vertex_buffer: Vec<Buffer>,

//     pub textures: HashMap<PathBuf, (u64, u64, u64)>,  // map from texture path to (buffer offset, size, image_hash)
//     pub materials: HashMap<String, (u64, u64)>,       // map from material name to (buffer offset, size)
//     pub per_draw: HashMap<String, (u64, u64)>,        // map from per_draw_info name to (buffer offset, size)
//     pub skins: HashMap<String, (u64, u64)>,           // map from mesh to (buffer offset, size) 
//     pub transforms: HashMap<String, (u64, u64)>,      // map from mesh to (buffer offset, size)
//     pub vertex: HashMap<String, (u64, u64)>,          // map from mesh to (buffer offset, size)

//     pub acceleration_structure: khr::AccelerationStructure,

//     pub reallocate_flags: usize, // 0: none, 1: textures, 2: buffers
//     pub last_buffer: usize,
// }

// pub trait ResourceManager {
//     fn new(window: &mut Window, settings: &ManagerBaseSettings, render_settings: Option<RendererSettings>) -> Self;
//     // buffers, textures and materials can have different precisions (T)
//     //fn buffer_insert<T>(&mut self, buffer: &Buffer, data:&[T]) -> usize;
//     fn next_slot(&self, kind: ResourceKind) -> u64;
//     fn register_texture<T: num::Num + TriviallyTransmutable>(&mut self, texture: &mut Image<T>) -> Result<(), ImageError> where Image<T>: ToRawData<T>; 
//     fn register_material<T: num::Num + TriviallyTransmutable>(&mut self, material: &Material<T>) -> Result<(), MaterialError> where Image<T>: ToRawData<T>;
//     fn register_vulkan_mesh(&mut self, name: String, mesh: &VulkanMesh);

//     fn get_descriptors(&self) -> Vec<vk::DescriptorSet>;
// }

// pub enum ResourceKind {
//     Texture,
//     Material,
//     PerDraw,
//     Skins,
//     Transforms,
//     Vertex,
//     AccelerationStructure
// }

// struct CommandBuffers {
//     texture: Vec<vk::CommandBuffer>,
//     material: Vec<vk::CommandBuffer>,
//     vertex: Vec<vk::CommandBuffer>,
//     index: Vec<vk::CommandBuffer>
// }

// struct CommandPools {
//     texture: Vec<vk::CommandPool>,
//     material: Vec<vk::CommandPool>,
//     vertex: Vec<vk::CommandPool>,
//     index: Vec<vk::CommandPool>
// }

// struct Command {
//     buffers: CommandBuffers,
//     pools: CommandPools,
// }

// impl ResourceManager for BindlessManager {
//     fn new(window: &mut Window, settings: &ManagerBaseSettings, render_settings: Option<RendererSettings>) -> Self {

//         let render_settings = match render_settings {
//             Some(x) => x,
//             None => {
//                 let render_settings = RendererSettings {
//                     samples: 8,
//                     depth: true,
//                     clear_color: glam::Vec4::ZERO,
//                     present_mode: vk::PresentModeKHR::IMMEDIATE,
//                     frames_in_flight: settings.frames_in_flight,
//                     extensions: settings.extensions.clone(),
//                     device_extensions: settings.device_extensions.clone(),
//                 };
//                 render_settings
//             }
            
//         };
//         // create the context
//         let shared_context = Arc::new(SharedContext::new(window, &render_settings));
    
//         let mut command_pools = Vec::new();
//         let mut command_buffers = Vec::new();
//         let mut resource_pools = HashMap::new();

//         let mut textures_buffer = Vec::new();
//         let materials_buffer = Vec::new();
//         let per_draw_buffer = Vec::new();
//         let skin_buffer = Vec::new();
//         let transforms_buffer = Vec::new();
//         let vertex_buffer = Vec::new();

//         for _ in 0..settings.frames_in_flight {
//             let buffer = Buffer::new(shared_context.clone(), settings.default_allocation[0].buffer_info, settings.default_allocation[0].size, 1);
//             textures_buffer.push(buffer);
//         }


//         let textures = HashMap::new();
//         let materials = HashMap::new();
//         let per_draw = HashMap::new();
//         let skins = HashMap::new();
//         let transforms = HashMap::new();
//         let vertex = HashMap::new();
//         let reallocate_flags = 0;

//         let command_record_scaling = 0.5;
//         let core_count = num_cpus::get();
//         let n_command_pools = (core_count as f32 * command_record_scaling * settings.frames_in_flight as f32) as usize;
//         let device = shared_context.device();
//         let queue_family_indices = shared_context.queue_family_indices();

//         for _ in 0..n_command_pools {
//             for queue_family_index in queue_family_indices.as_array().iter().fold(Vec::new(), |mut acc, x| {
//                 if !acc.contains(x) {
//                     acc.insert(0,*x);
//                     acc
//                 } else {
//                     acc
//                 }
//             }) {
//                 let pool_create_info = vk::CommandPoolCreateInfo::builder()
//                     .flags(vk::CommandPoolCreateFlags::TRANSIENT)
//                     .queue_family_index(queue_family_index);

//                 unsafe {
//                     let pool = device
//                         .create_command_pool(&pool_create_info, None)
//                         .unwrap();
//                     command_pools.push(pool);
//                     let c_buffers = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder()
//                         .command_pool(pool)
//                         .level(vk::CommandBufferLevel::PRIMARY)
//                         .command_buffer_count(1)
//                         .build(),
//                     ).expect("Failed to allocate command buffers!");
//                     command_buffers.push(c_buffers[0]);
//                 }
//             }
//         }

//         resource_pools.insert(vk::MemoryPropertyFlags::DEVICE_LOCAL, Vec::new());
//         resource_pools.insert(vk::MemoryPropertyFlags::HOST_VISIBLE, Vec::new());
//         resource_pools.insert(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT, Vec::new());
//         resource_pools.insert(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_CACHED, Vec::new());

//         let acceleration_structure = khr::AccelerationStructure::new(&shared_context.instance(), &device);

//         BindlessManager {
//             shared_context,
//             render_settings,
//             command_pools,
//             command_buffers,
//             resource_pools,
//             textures_buffer,
//             materials_buffer,
//             per_draw_buffer,
//             skin_buffer,
//             transforms_buffer,
//             vertex_buffer,
//             textures,
//             materials,
//             per_draw,
//             skins,
//             transforms,
//             vertex,
//             acceleration_structure,
//             last_buffer: 0,
//             reallocate_flags,
//         }
//     }

//     fn next_slot(&self, kind: ResourceKind) -> u64 {
//         match kind {
//             ResourceKind::Texture => {
//                 // sum up sizes of self.textures
//                 self.textures.iter().fold(0, |acc, x| acc + x.1.1)
//             }
//             ResourceKind::Material => {
//                 // sum up sizes of self.materials
//                 self.materials.iter().fold(0, |acc, x| acc + x.1.1)
//             }
//             ResourceKind::PerDraw => {
//                 // sum up sizes of self.per_draw
//                 self.per_draw.iter().fold(0, |acc, x| acc + x.1.1)
//             }
//             ResourceKind::Skins => {
//                 // sum up sizes of self.skins
//                 self.skins.iter().fold(0, |acc, x| acc + x.1.1)
//             }
//             ResourceKind::Transforms => {
//                 // sum up sizes of self.transforms
//                 self.transforms.iter().fold(0, |acc, x| acc + x.1.1)
//             }
//             ResourceKind::Vertex => {
//                 // sum up sizes of self.vertex
//                 self.vertex.iter().fold(0, |acc, x| acc + x.1.1)
//             }
//             _ => 0,
//         }
//     }
    

//     fn register_texture<T: num::Num + TriviallyTransmutable>(&mut self, texture: &mut Image<T>) -> Result<(), ImageError> where Image<T>: ToRawData<T>{
//         // check if texture is already registered
//         let entry = self.textures.get(&texture.path);
//         match entry {
//             // check if texture has changed
//             Some((index, _, hash)) => {
//                 if texture.hash() == *hash {
//                     return Ok(());
//                 }
//             }
//             None => {}
//         }
//         let next_slot = self.next_slot(ResourceKind::Texture);

//         // we have to get the raw data from the image and insert it into a buffer
//         let next_buffer = (self.last_buffer + 1) % self.textures_buffer.len();
        
//         let buffer = &mut self.textures_buffer[next_buffer];
//         //self.buffer_insert(buffer, &[texture.raw_data()]);
        
//         let (offset, size) = buffer.insert(transmute_to_bytes(texture.raw_data().clone().as_slice()), next_slot);
        
//         // record commands
//         unsafe {
//             self.shared_context.device().reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
//                 .expect("Failed to reset command buffer");
    
//             self.shared_context.device().begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::builder()
//                 .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
//                 .build()
//             )
//             .expect("Failed to begin command buffer");
    
//             self.shared_context.device().cmd_copy_buffer(command_buffer, buffer[self.last_buffer].handle(), buffer[next_idx].handle(), &[vk::BufferCopy::builder()
//                 .src_offset(offset)
//                 .dst_offset(offset)
//                 .size(size as vk::DeviceSize)
//                 .build()
//             ]);
//         }
        
        
//         // then update the map of textures with the returned index
//         self.textures.insert(texture.path.clone(), (offset, size, texture.hash()));
//         self.reallocate_flags |= 1;

//         // then update the map of textures with the returned index

//         Ok(())
//     }

//     fn register_material<T: num::Num + TriviallyTransmutable>(&mut self, material: &Material<T>) -> Result<(), MaterialError> where Image<T>: ToRawData<T> {
//         // check if material is already registered
//         if self.materials.contains_key(&material.id) {
//             // check if materials are the same
//             return Err(MaterialError::AlreadyRegistered(material.id.clone() + &" is already registered. You can update using update_material".to_string()));
//         }
//         // check if Images used by material are already registered
//         for path in material.texture_paths().iter() {
//             if !self.textures.contains_key(path) {
//                 let texture = Image::<T>::new(path.clone());
//                 match texture {
//                     Ok(mut texture) => {
//                         let res = self.register_texture(&mut texture);
//                         match res {
//                             Ok(_) => {}
//                             Err(e) => {
//                                 return Err(MaterialError::InvalidTexture(e.to_string()));
//                             }
//                         }
//                     }
//                     Err(e) => {
//                         return Err(MaterialError::MissingTexture(e.to_string()));
//                     }
//                 }
//             }
//         };
//         // pick the next available buffer
//         let next_buffer = (self.last_buffer + 1) % self.materials_buffer.len();
//         let next_slot = self.next_slot(ResourceKind::Material);
        
//         let buffer = &mut self.materials_buffer[next_buffer];
//         // look up where the textures are stored so we can pack the material struct with
//         // with references to those buffers
//         let texture_indices = material.texture_paths().iter().map(|path| {
//             match self.textures.get(path) {
//                 Some((texture_index, _, _)) => texture_index.clone(),
//                 None => 0
//             }
//         }).collect::<Vec<u64>>();

//         // insert material at the end of the buffer
//         // TODO: implement material handle such that it contains references to the textures
        
//         let loc = buffer.insert(MaterialHandle::new(material, texture_indices).as_bytes().as_slice(), next_slot); 
//         self.materials.insert(material.id.clone(), (MaterialInfo::size() as u64, next_slot));

//         self.reallocate_flags |= 2;
//         Ok(())
//     }

//     fn register_vulkan_mesh(&mut self, name: String, mesh: &VulkanMesh) {

//         // self.per_draw.insert(name, mesh.per_draw);
//         // self.transforms.insert(name, mesh.transforms);
//         // self.vertex.insert(name, mesh.vertex);
//         let vertex_data = &mesh.vertex_buffer;
//         let index_data = &mesh.index_buffer;
//         let index_storage_data = &mesh.index_storage;
//         let skin = &mesh.skin;
//         //self.shared_context.device().allocate_memory(create_info, allocation_callbacks)

//         self.reallocate_flags |= 3;
//     }

//     // this function will trigger the submission of all memory bind and command recording operations.
//     // it try to group these operations.
//     fn get_descriptors(&self) -> Vec<vk::DescriptorSet> {
//         if self.reallocate_flags > 0 {

//         }
//         Vec::new()
//     }

    
// }


// impl BindlessManager {
//     fn register_mesh(&mut self, name: String, mesh: &Mesh) {
//         // self.per_draw.insert(name, mesh.per_draw);
//         // self.transforms.insert(name, mesh.transforms);
//         // self.vertex.insert(name, mesh.vertex);
//         self.reallocate_flags |= 3;
//     }
//     fn upload(&mut self, buffer: &Vec<Buffer>) {
//         let command_buffer = self.command_buffers[self.last_buffer];
//         let next_idx = (self.last_buffer + 1) % buffer.len();

//     }

//     fn update(&mut self) {
//         // find out which buffers need to be reallocated
//         let upload_ids = Vec::new();
//         match self.reallocate_flags {
//             1 => {
//                 // textures
//                 self.upload(&self.textures_buffer);
//             }
//             2 => {
//                 // materials
//                 self.upload(&self.materials_buffer);
//             }
//             3 => {
//                 //textures and materials
//                 self.command.buffers().texture().sumit()
//             }
//             4 => {
//                 // textures and materials
//                 upload_ids.push(3);
//             }
//         }
//     }
// }


// impl Drop for BindlessManager {
//     fn drop(&mut self) {
//         let device = self.shared_context.device();
//         unsafe {
//             for pool in self.command_pools.iter() {
//                 device.destroy_command_pool(*pool, None);
//             }
//             // buffer drops should be implicitly handled
//         }
//     }
// }

// struct MaterialHandle {
//     info: MaterialInfo, // 80 bytes
//     textures: Option<Vec<u64>>,
// }

// impl MaterialHandle {
//     fn as_bytes(&self) -> Vec<u8> {
//         let info_bytes = unsafe {
//             std::slice::from_raw_parts(
//                 &self.info as *const MaterialInfo as *const u8,
//                 std::mem::size_of::<MaterialInfo>(),
//             )
//         };
//         let textures_len = self.textures.as_ref().map(|t| t.len()).unwrap_or(0);
//         let textures_bytes = self.textures.as_ref().map(|t| {
//             unsafe {
//                 std::slice::from_raw_parts(
//                     t.as_ptr() as *const u64 as *const u8,
//                     textures_len * std::mem::size_of::<u64>(),
//                 )
//             }
//         });
//         let mut bytes = Vec::with_capacity(std::mem::size_of::<MaterialInfo>() + textures_len * std::mem::size_of::<u64>());
//         bytes.extend_from_slice(info_bytes);
//         if let Some(textures_bytes) = textures_bytes {
//             bytes.extend_from_slice(textures_bytes);
//         }
//         bytes
//     }
// }

// impl MaterialHandle {
//     pub fn new<T: num::Num>(material: &Material<T>, texture_indices: Vec<u64>) -> Self {
//         MaterialHandle {
//             info: MaterialInfo::new(material),
//             textures: Some(texture_indices),
//         }
//     }
// }