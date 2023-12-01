use std::path::PathBuf;

use std::collections::HashMap;
use std::sync::Arc;
use ash::vk::{ImageCreateInfo, self, BufferUsageFlags, IndexType};
use fontdue::Metrics;
use sdf_glyph_renderer::BitmapGlyph;
use rayon::prelude::*;

pub mod parsing;

use crate::{Texture2d, Context, Image2d, Buffer, Resource, Vertex, offset_of, resource::mesh::ModelVertex};

#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub width: f32,
    pub height: f32,
    pub advance: f32,
    pub atlas_x: f32,
    pub atlas_y: f32,
}

impl Glyph {
    // a simple quad (two triangles) for the glyph
    pub fn geometry() -> Vec<ModelVertex> {
        vec![
            ModelVertex { pos: glam::Vec4::new(0.0, 0.0, 0.0, 1.0), normal: glam::Vec4::new(0.0, 0.0, 1.0, 1.0), color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0), uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0) },
            ModelVertex { pos: glam::Vec4::new(1.0, 0.0, 0.0, 1.0), normal: glam::Vec4::new(0.0, 0.0, 1.0, 1.0), color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0), uv: glam::Vec4::new(1.0, 0.0, 0.0, 0.0) },
            ModelVertex { pos: glam::Vec4::new(1.0, 1.0, 0.0, 1.0), normal: glam::Vec4::new(0.0, 0.0, 1.0, 1.0), color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0), uv: glam::Vec4::new(1.0, 1.0, 0.0, 0.0) },
            ModelVertex { pos: glam::Vec4::new(0.0, 0.0, 0.0, 1.0), normal: glam::Vec4::new(0.0, 0.0, 1.0, 1.0), color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0), uv: glam::Vec4::new(0.0, 0.0, 0.0, 0.0) },
            ModelVertex { pos: glam::Vec4::new(1.0, 1.0, 0.0, 1.0), normal: glam::Vec4::new(0.0, 0.0, 1.0, 1.0), color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0), uv: glam::Vec4::new(1.0, 1.0, 0.0, 0.0) },
            ModelVertex { pos: glam::Vec4::new(0.0, 1.0, 0.0, 1.0), normal: glam::Vec4::new(0.0, 0.0, 1.0, 1.0), color: glam::Vec4::new(1.0, 1.0, 1.0, 1.0), uv: glam::Vec4::new(0.0, 1.0, 0.0, 0.0) },
        ]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphInstance {
    pub transform: glam::Mat4,
    pub wh_atlas: glam::Vec4, // width, height, atlas_x, atlas_y
    pub color: glam::Vec4,
}

impl Vertex for GlyphInstance {
    fn stride() -> u32 {
        2 * std::mem::size_of::<glam::Vec4>() as u32 + std::mem::size_of::<glam::Mat4>() as u32
    }

    fn format_offset() -> Vec<(ash::vk::Format, u32)> {
        let vec4_size = std::mem::size_of::<glam::Vec4>();
        let mat4_size = std::mem::size_of::<glam::Mat4>();
        vec![
            (ash::vk::Format::R32G32B32A32_SFLOAT, 0),                // transform col 1
            (ash::vk::Format::R32G32B32A32_SFLOAT, vec4_size as u32), // transform col 2
            (ash::vk::Format::R32G32B32A32_SFLOAT, 2 * vec4_size as u32), // transform col 3
            (ash::vk::Format::R32G32B32A32_SFLOAT, 3 * vec4_size as u32), // transform col 4
            (ash::vk::Format::R32G32B32A32_SFLOAT, mat4_size as u32), // wh_atlas
            (ash::vk::Format::R32G32B32A32_SFLOAT, mat4_size as u32 + vec4_size as u32), // color
        ]
    }
}

impl Vertex for Glyph {
    fn stride() -> u32 {
        5 * std::mem::size_of::<f32>() as u32
    }

    fn format_offset() -> Vec<(ash::vk::Format, u32)> {
        vec![
            (ash::vk::Format::R32_SFLOAT, offset_of!(Glyph, width) as u32),
            (ash::vk::Format::R32_SFLOAT, offset_of!(Glyph, height) as u32),
            (ash::vk::Format::R32_SFLOAT, offset_of!(Glyph, advance) as u32),
            (ash::vk::Format::R32_SFLOAT, offset_of!(Glyph, atlas_x) as u32),
            (ash::vk::Format::R32_SFLOAT, offset_of!(Glyph, atlas_y) as u32),
        ]
    }
}

pub struct GlyphAtlas {
    pub texture: Image2d,
    pub glyphs: HashMap<char, Glyph>,
    pub buffer: Buffer,
    pub current_layout: vk::ImageLayout,
    pub sampler: vk::Sampler,
    pub font_data: Vec<u8>,
}

impl GlyphAtlas {
    pub fn new(context: Arc<Context>, sdfs: Vec<(char, Metrics, Vec<f64>)>, font_data: Vec<u8>) -> Self {
        let mut glyphs = HashMap::new();

        // Define texture size
        let (atlas_width, atlas_height) = (2048 as u32, 2048 as u32);
        let mut data = vec![0.0; (atlas_width * atlas_height) as usize];  // Allocate space for the atlas

        let mut current_x: u32 = 0;
        let mut current_y: u32 = 0;
        let mut max_height_of_current_row: u32 = 0;

        for (character, metrics, sdf) in sdfs {
            let glyph_width = metrics.width as f32;
            let glyph_height = metrics.height as f32;

            // Check if glyph fits into the current row
            if current_x + glyph_width as u32 > atlas_width {
                // Start a new row
                current_x = 0;
                current_y += max_height_of_current_row;
                max_height_of_current_row = 0;
            }

            // Check if glyph fits into the texture
            if current_y + glyph_height as u32 > atlas_height {
                // Not enough space in atlas
                panic!("Not enough space in the glyph texture atlas.");
            }

            // Insert the glyph sdf into the atlas
            for y in 0..glyph_height as u32 {
                for x in 0..glyph_width as u32 {
                    let index = ((y + current_y) * atlas_width + (x + current_x)) as usize;
                    let sdf_value = sdf[(y * glyph_width as u32 + x) as usize];
                    data[index] = sdf_value as f32;
                }
            }

            let glyph = Glyph {
                width: glyph_width,
                height: glyph_height,
                advance: metrics.advance_width as f32,
                atlas_x: current_x as f32,
                atlas_y: current_y as f32,
            };

            glyphs.insert(character, glyph);

            current_x += glyph_width as u32;
            max_height_of_current_row = max_height_of_current_row.max(glyph_height as u32);
        }

        let create_info = ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R32_SFLOAT)
            .extent(vk::Extent3D {
                width: atlas_width,
                height: atlas_height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(
                vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
        
        println!("create info: {:?}", create_info);
        let mut texture = Image2d::new(context.shared().clone(), &create_info, vk::ImageAspectFlags::COLOR, 1, "GlyphAtlas");
        let buffer = Buffer::from_data(
            context.clone(), 
            crate::BufferInfo { 
                name: "GlyphAtlas_sdf_data", 
                usage: BufferUsageFlags::STORAGE_BUFFER, 
                mem_usage: gpu_allocator::MemoryLocation::CpuToGpu, 
                memory_type_bits: None, 
                index_type: None,
                vertex_input_rate: None 
            }, 
            &data
        );

        let cmd = context.begin_single_time_cmd();
        texture.transition_image_layout(cmd, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        context.end_single_time_cmd(cmd);
        texture.copy_to_image(&context, buffer.handle());
        let cmd = context.begin_single_time_cmd();
        texture.transition_image_layout(cmd, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        context.end_single_time_cmd(cmd);

        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .min_filter(vk::Filter::LINEAR)
            .mag_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .min_lod(0.0)
            .max_lod(1 as f32)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .unnormalized_coordinates(false);
        let sampler: vk::Sampler;
        unsafe {
            sampler = context
                .device()
                .create_sampler(&sampler_create_info, None)
                .unwrap();
        }

        Self { 
            texture,
            glyphs,
            buffer,
            current_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            sampler,
            font_data, 
        }
        
    }

    pub fn get_descriptor_info(&self) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo::builder()
        .sampler(self.sampler)
        .image_view(self.texture.get_image_view())
        .image_layout(self.current_layout)
        .build()
    }

    pub fn font_face(&self) -> Option<ttf_parser::Face<'_>> {
        match ttf_parser::Face::parse(&self.font_data, 0) {
            Ok(face) => Some(face),
            Err(_) => None,
        }
    }

    pub fn kerning_table(&self) -> Option<ttf_parser::kern::Table> {
        let fontface = self.font_face()?;
        fontface.tables().kern
    }

    // pub fn transition(&mut self, cmd: vk::CommandBuffer, layout: vk::ImageLayout) {
    //     if (self.current_layout == layout) {
    //         return;
    //     }
    //     self.texture.transition_image_layout(cmd, self.current_layout, layout);
    //     self.current_layout = layout;
    // }

    // pub fn upload_data(&mut self, context: Arc<Context>) {
    //     self.texture.copy_to_image(&context, self.buffer.handle())
    // }
}

pub fn extend_bitmap(bitmap: Vec<u8>, metrics: Metrics, size: usize) -> Vec<u8> {
    // extend the bitmap on every axis by size filling the new pixels with 0
    let mut new_bitmap = vec![0; (metrics.width + size * 2) as usize * (metrics.height + size * 2) as usize];
    for y in 0..metrics.height as usize {
        for x in 0..metrics.width as usize {
            let index = y * metrics.width as usize + x;
            let new_index = (y + size) * (metrics.width + size * 2) as usize + (x + size);
            new_bitmap[new_index] = bitmap[index];
        }
    }
    return new_bitmap;
}


// pub fn enumerate_ascii_chars() -> impl Iterator<Item = char> {
//     (0..128).into_iter().map(|i| i as u8 as char)
// }

pub fn enumerate_ascii_chars() -> impl Iterator<Item = char> {
    " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~".chars()
}

pub fn ftt_to_atlas(context: Arc<Context>, path: &PathBuf) -> GlyphAtlas {
    let fontfile = crate::util::find_asset(path.to_str().unwrap());   
    if fontfile.is_none() {
        panic!("Could not find font file");
    }
    let path = fontfile.unwrap();

    // load binary data from path to font file
    let font_data = std::fs::read(path).unwrap();

    let font = fontdue::Font::from_bytes(font_data.clone(), fontdue::FontSettings::default()).unwrap();
    // Rasterize and get the layout metrics for the letter 'g' at 17px.
    let res = 128.0;
    let radius = 8;

    let sdfs = enumerate_ascii_chars().collect::<Vec<_>>().into_par_iter().map(|c| {
        let (mut metrics, bitmap) = font.rasterize(c, res);
        println!("metrics[{:?}]: {:?}", c, metrics);
        let buffer: usize = (res / 8.0) as usize;
        let bitmap = extend_bitmap(bitmap, metrics, buffer);
        
        let sdfbm = BitmapGlyph::new(bitmap, metrics.width, metrics.height, buffer).unwrap();
        metrics.width = metrics.width + (buffer * 2);
        metrics.height = metrics.height + (buffer * 2);
        (c, metrics, sdfbm.render_sdf(radius))
    }).collect::<Vec<_>>();

    GlyphAtlas::new(context.clone(), sdfs, font_data)

}
