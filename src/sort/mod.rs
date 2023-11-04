use std::{path::PathBuf, sync::Arc};

use crate::{
    Buffer, ComputePipeline, ComputePipelineInfo, Context, DescriptorSet, DescriptorSetLayout,
    DescriptorSetLayoutInfo, PipelineLayout, PipelineLayoutInfo, Resource, DescriptorSetInfo, BufferInfo,
};
use ash::vk::{self, CommandBuffer};

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct PushConstantsHistograms {
    g_num_elements: u32,             // == NUM_ELEMENTS
    g_shift: u32,                    // (*)
    g_num_workgroups: u32,           // == NUMBER_OF_WORKGROUPS as defined in the section above
    g_num_blocks_per_workgroup: u32, // == NUM_BLOCKS_PER_WORKGROUP
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct PushConstants {
    g_num_elements: u32,             // == NUM_ELEMENTS
    g_shift: u32,                    // (*)
    g_num_workgroups: u32,           // == NUMBER_OF_WORKGROUPS as defined in the section above
    g_num_blocks_per_workgroup: u32, // == NUM_BLOCKS_PER_WORKGROUP
}

struct SortBuf {
    pub buffer: vk::Buffer,
    pub descriptor_info: vk::DescriptorBufferInfo,
}

impl SortBuf {
    pub fn new(buffer: &Buffer) -> Self {
        Self {
            buffer: buffer.handle(),
            descriptor_info: buffer.get_descriptor_info(),
        }
    }
}

pub struct RadixSortInfo {
    num_blocks_per_workgroup: u32,
    sort: SortBuf,
    ord: SortBuf,
    buffer_size: vk::DeviceSize,
    buffer_element_count: u32,
}

impl RadixSortInfo {
    pub fn new(sort_buffer: &Buffer, ord_buffer: &Buffer) -> Self {
        assert!(sort_buffer.get_size() == ord_buffer.get_size());
        Self {
            num_blocks_per_workgroup: 32,
            sort: SortBuf::new(sort_buffer),
            ord: SortBuf::new(ord_buffer),
            buffer_size: sort_buffer.get_size(),
            buffer_element_count: sort_buffer.get_element_count(),
        }
    }
}

impl RadixSortInfo {
    pub fn global_invocation_size(&self) -> (u32, u32, u32) {
        let num_elements = self.buffer_element_count;
        let global_i_size = num_elements / self.num_blocks_per_workgroup;
        let remainder = num_elements % self.num_blocks_per_workgroup;
        if remainder > 0 {
            (global_i_size + 1, 1, 1)
        } else {
            (global_i_size, 1, 1)
        }
    }
}


pub struct RadixSort {
    pipeline: ComputePipeline,
    info: RadixSortInfo,
    pub desc_sets: [DescriptorSet; 2],
    pub desc_set_layout: DescriptorSetLayout,
    pub pipeline_layout: PipelineLayout,
    barriers: [vk::MemoryBarrier; 2],
    push_constants: PushConstants,
    push_constants_histograms: PushConstantsHistograms,
    pub m_buffer0: vk::Buffer, // array to sort (owned by caller)
    m_buffer1: Buffer,
    m_buffer_histograms: Buffer,
    pub m_ord0: vk::Buffer, // orderings (owned by caller)
    m_ord1: Buffer,
}

impl RadixSort {
    pub fn new(context: Arc<Context>, info: RadixSortInfo) -> Self {
        let mut desc_set_layout = DescriptorSetLayout::new(
            context.clone(),
            DescriptorSetLayoutInfo::default()
                // m_buffer0 is the buffer to sort
                .binding(
                    0,
                    vk::DescriptorType::STORAGE_BUFFER,
                    vk::ShaderStageFlags::COMPUTE,
                )
                // m_buffer1 pong buffer
                .binding(
                    1,
                    vk::DescriptorType::STORAGE_BUFFER,
                    vk::ShaderStageFlags::COMPUTE,
                )
                // m_buffer_histograms radix metadata
                .binding(
                    2,
                    vk::DescriptorType::STORAGE_BUFFER,
                    vk::ShaderStageFlags::COMPUTE,
                )
                // ord_buffer0 orderings to be able to "reverse" the sort
                .binding(
                    3,
                    vk::DescriptorType::STORAGE_BUFFER,
                    vk::ShaderStageFlags::COMPUTE,
                )
                // ord_buffer1 orderings pong buffer
                .binding(
                    4,
                    vk::DescriptorType::STORAGE_BUFFER,
                    vk::ShaderStageFlags::COMPUTE,
                ),
        );
        let constant_ranges = vec![
            vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(std::mem::size_of::<PushConstants>() as u32)
                .build(),
            vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(std::mem::size_of::<PushConstants>() as u32)
                .size(std::mem::size_of::<PushConstantsHistograms>() as u32)
                .build(),
        ];
        let pipeline_layout = PipelineLayout::new(
            context.clone(),
            PipelineLayoutInfo::default()
                .desc_set_layouts(&[desc_set_layout.handle(), desc_set_layout.handle()]) // cheap trick to get correct number of sets
                .push_constant_ranges(constant_ranges.as_slice()),
        );
        let pipeline = ComputePipeline::new(
            context.clone(),
            ComputePipelineInfo::default()
                .layout(pipeline_layout.handle())
                .comp(crate::util::find_asset("glsl/sort/multi_radixsort_histograms.comp").unwrap())
                .comp(crate::util::find_asset("glsl/sort/multi_radixsort.comp").unwrap()),
        );

        let barriers = [
            vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build(),
            vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build()
        ];

        let buffer_info = BufferInfo::default()
        .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
        .gpu_only()
        .memory_type_bits(vk::MemoryPropertyFlags::DEVICE_LOCAL.as_raw());

        let m_buffer0 = info.sort.buffer;
        let m_buffer1 = Buffer::new(
            context.clone(),
            buffer_info.clone().name("m_buffer1"),
            info.buffer_size,
            info.buffer_element_count,
        );
        let m_buffer_histograms = Buffer::new(
            context.clone(),
            buffer_info.clone().name("m_buffer_histograms"),
            info.buffer_size,
            info.buffer_element_count,
        );
        let m_ord0 = info.ord.buffer;
        let m_ord1 = Buffer::new(
            context.clone(),
            buffer_info.clone().name("m_ord1"),
            info.buffer_size,
            info.buffer_element_count,
        );


        // setup for first iteration
        let desc_set1 = desc_set_layout.get_or_create(
            DescriptorSetInfo::default()
            .buffer(0, info.sort.descriptor_info)
            .buffer(1, m_buffer_histograms.get_descriptor_info())
        );
        let desc_set2 = desc_set_layout.get_or_create(
            DescriptorSetInfo::default()
            .buffer(0, info.sort.descriptor_info)
            .buffer(1, m_buffer1.get_descriptor_info())
            .buffer(2, m_buffer_histograms.get_descriptor_info())
            .buffer(3, info.ord.descriptor_info)
            .buffer(4, m_ord1.get_descriptor_info())
        );

        let g_num_elements = info.buffer_element_count;
        let g_num_workgroups = (info.global_invocation_size().0 + 256 -1) / 256;
        let g_num_blocks_per_workgroup = 32;

        Self {
            pipeline,
            info,
            desc_sets: [desc_set1, desc_set2],
            desc_set_layout,
            pipeline_layout,
            barriers,
            push_constants: PushConstants {
                g_num_elements,
                g_shift: 0,
                g_num_workgroups,
                g_num_blocks_per_workgroup,
            },
            push_constants_histograms: PushConstantsHistograms {
                g_num_elements,
                g_shift: 0,
                g_num_workgroups,
                g_num_blocks_per_workgroup,
            },
            m_buffer0,
            m_buffer1,
            m_buffer_histograms,
            m_ord0,
            m_ord1,
        }

    }


    fn desc_set_for_iteration(&mut self, index: i32) -> [DescriptorSet; 2] {
        match index {
            0 => {
                let desc_set1 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.info.sort.descriptor_info)
                    .buffer(1, self.m_buffer_histograms.get_descriptor_info())
                );
                let desc_set2 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.info.sort.descriptor_info)
                    .buffer(1, self.m_buffer1.get_descriptor_info())
                    .buffer(2, self.m_buffer_histograms.get_descriptor_info())
                    .buffer(3, self.info.ord.descriptor_info)
                    .buffer(4, self.m_ord1.get_descriptor_info())
                );
                [desc_set1, desc_set2]
            },
            1 => {
                let desc_set1 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.m_buffer1.get_descriptor_info())
                    .buffer(1, self.m_buffer_histograms.get_descriptor_info())
                );
                let desc_set2 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.m_buffer1.get_descriptor_info())
                    .buffer(1, self.info.sort.descriptor_info)
                    .buffer(2, self.m_buffer_histograms.get_descriptor_info())
                    .buffer(3, self.m_ord1.get_descriptor_info())
                    .buffer(4, self.info.ord.descriptor_info)
                );
                [desc_set1, desc_set2]
            }
            2 => {
                let desc_set1 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.info.sort.descriptor_info)
                    .buffer(1, self.m_buffer_histograms.get_descriptor_info())
                );
                let desc_set2 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.info.sort.descriptor_info)
                    .buffer(1, self.m_buffer1.get_descriptor_info())
                    .buffer(2, self.m_buffer_histograms.get_descriptor_info())
                    .buffer(3, self.info.ord.descriptor_info)
                    .buffer(4, self.m_ord1.get_descriptor_info())
                );
                [desc_set1, desc_set2]
            }
            3 => {
                let desc_set1 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.m_buffer1.get_descriptor_info())
                    .buffer(1, self.m_buffer_histograms.get_descriptor_info())
                );
                let desc_set2 = self.desc_set_layout.get_or_create(
                    DescriptorSetInfo::default()
                    .buffer(0, self.m_buffer1.get_descriptor_info())
                    .buffer(1, self.info.sort.descriptor_info)
                    .buffer(2, self.m_buffer_histograms.get_descriptor_info())
                    .buffer(3, self.info.ord.descriptor_info)
                    .buffer(4, self.m_ord1.get_descriptor_info())
                );
                [desc_set1, desc_set2]
            }
            _ => panic!("invalid iteration index in RadixSort"),
        }
    }


    pub fn pass(&mut self, cmd: CommandBuffer, device: &ash::Device) {
        (0..4).into_iter().for_each(|i| {
            self.pass_impl(cmd, device, i);
        });
        assert!(self.push_constants.g_shift == 0);
    }

    fn pass_impl(&mut self, cmd: CommandBuffer, device: &ash::Device, index: i32) {
        let sets = self.desc_set_for_iteration(index);
        let descriptor_sets = [sets[0].handle(), sets[1].handle()];
        let i_size = self.info.global_invocation_size();
        unsafe {

            // histo pass
            device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline.handles().unwrap()[0],
            );
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout.handle(),
                0,
                descriptor_sets.as_slice(),
                &[],
            );
            device.cmd_push_constants(
                cmd,
                self.pipeline_layout.handle(),
                vk::ShaderStageFlags::COMPUTE,
                0,
                &bytemuck::bytes_of(&self.push_constants_histograms)
            );
            device.cmd_dispatch(cmd, i_size.0, i_size.1, i_size.2);
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                [self.barriers[0]].as_slice(),
                &[],
                &[],
            );

            // sort pass
            device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline.handles().unwrap()[1],
            );
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout.handle(),
                0,
                descriptor_sets.as_slice(),
                &[],
            );
            device.cmd_push_constants(
                cmd,
                self.pipeline_layout.handle(),
                vk::ShaderStageFlags::COMPUTE,
                0,
                &bytemuck::bytes_of(&self.push_constants)
            );
            device.cmd_dispatch(cmd, i_size.0, i_size.1, i_size.2);
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                [self.barriers[1]].as_slice(),
                &[],
                &[],
            );
        }

        // increment shift on push constants
        self.push_constants.g_shift += 8;
        self.push_constants_histograms.g_shift += 8;
        self.push_constants.g_shift %= 32;
        self.push_constants_histograms.g_shift %= 32;
    }
}
