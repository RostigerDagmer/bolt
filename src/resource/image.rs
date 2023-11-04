use std::{path::PathBuf, cmp::max, self};
use ash::vk;
use image::{ImageError, GenericImageView, DynamicImage};
use rayon::prelude::*;
use num;
use std::hash::BuildHasher;
use ahash::RandomState;

pub enum SupportedTypes {
    PNG8(u8),
    PNG16(u16),
    JPG(u8),
    TGA8(u8),
    TGA16(u16),
    TGA32(f32),
    BMP8(u8),
    BMP16(u8),
    // DDS,
    // KTX,
    // PVR,
    // ASTC,
    HDR16(u16),
    HDR32(f32),
    EXR16(u16),
    EXR32(f32),
    TIF16(u16),
    TIF32(f32),
    // WEBP,
    // GIF,
    // ICO,
    // PNM,
    // PAM,
    // PPM,
    // PGM,
    // PBM,
    // DXT,
    // TGA_RAW,
    // TGA_RLE,
    // TGA_NO_RLE,
    // TGA_NO_RLE_NO_COLOR,
    // TGA_NO_RLE_COLOR,
    // TGA_RLE_NO_COLOR,
    // TGA_RLE_COLOR,
    // TGA_NO_COLOR,
    // TGA_COLOR,
    // TGA_NO_RLE_NO_COLOR_16,
    // TGA_NO_RLE_COLOR_16,
    // TGA_RLE_NO_COLOR_16,
    // TGA_RLE_COLOR_16,
    // TGA_NO_COLOR_16,
    // TGA_COLOR_16,
    // TGA_NO_RLE_NO_COLOR_24,
    // TGA_NO_RLE_COLOR_24,
    // TGA_RLE_NO_COLOR_24,
    // TGA_RLE_COLOR_24,
    // TGA_NO_COLOR_24,
    // TGA_COLOR_24,
    // TGA_NO_RLE_NO_COLOR_32,
    // TGA_NO_RLE_COLOR_32,
    // TGA_RLE_NO_COLOR_32,
    // TGA_RLE_COLOR_32,
    // TGA_NO_COLOR_32,
    // TGA_COLOR_32,
    // TGA_NO_RLE_NO_COLOR_48,
    // TGA_NO_RLE_COLOR_48,
    // TGA_RLE_NO_COLOR_48,
    // TGA_RLE_COLOR_48,
    // TGA_NO_COLOR_48,
    // TGA_COLOR_48,
    // TGA_NO_RLE_NO_COLOR_64,
    // TGA_NO_RLE_COLOR_64,
    // TGA_RLE_NO_COLOR_64,
    // TGA_RLE_COLOR_64,
    // TGA_NO_COLOR_64,
    // TGA_COLOR_64,
    // TGA_NO_RLE_NO_COLOR_8,
    // TGA_NO_RLE_COLOR_8,
    // TGA_RLE_NO_COLOR_8,
    // TGA_RLE_COLOR_8,
    // TGA_NO_COLOR_8,
    // TGA_COLOR_8,
    // TGA_NO_RLE_NO_COLOR_GRAY,
    // TGA_NO_RLE_COLOR_GRAY,
    // TGA_RLE_NO_COLOR_GRAY,
    // TGA_RLE_COLOR_GRAY,
}

pub struct Image<T: num::Num> {
    pub path: PathBuf,
    pub hash: u64,
    pub info: vk::ImageCreateInfo,
    pub sampler_info: vk::SamplerCreateInfo,
    pub extent: vk::Extent3D,
    pub mip_levels: u32,
    pub subresource_range: vk::ImageSubresourceRange,
    pub format: vk::Format,
    pub data: DynamicImage,
    raw_data: Vec<T>,
}

pub trait ToRawData <T: num::Num>{
    fn raw_data(&mut self) -> &Vec<T>;
}

impl ToRawData<u8> for Image<u8> {
    fn raw_data(&mut self) -> &Vec<u8> {
        // caching
        if self.raw_data.len() > 0 {
            return &self.raw_data;
        }
        self.raw_data = self.data.to_rgba8().into_raw();
        &self.raw_data
    }
}

impl ToRawData<u16> for Image<u16> {
    fn raw_data(&mut self) -> &Vec<u16> {
        // caching
        if self.raw_data.len() > 0 {
            return &self.raw_data;
        }
        self.raw_data = self.data.to_rgba16().into_raw();
        &self.raw_data
    }
}

impl ToRawData<f32> for Image<f32> {
    fn raw_data(&mut self) -> &Vec<f32> {
        // caching
        if self.raw_data.len() > 0 {
            return &self.raw_data;
        }
        self.raw_data = self.data.to_rgba32f().into_raw();
        &self.raw_data
    }
}

unsafe impl<T: num::Num> Send for Image<T> {}

impl<T: num::Num> Image<T> {
    pub fn new(path: PathBuf) -> Result<Self, ImageError> {

        let pb = path.clone();
        let source_image = image::open(path);
        
        let mut source_image = match source_image {
            Ok(image) => image,
            Err(e) => {
                println!("Failed to load texture: {}", e);
                return Err(e);
            }
        };
        source_image = source_image.flipv();
        let size = source_image.dimensions();
        
        let mip_levels = (max(size.0, size.1) as f32).log2().floor() as u32 + 1;

        let format = vk::Format::UNDEFINED;

        let extent = vk::Extent3D {
            width: size.0,
            height: size.1,
            depth: 1,
        };

        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent.clone())
            .mip_levels(mip_levels)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(
                vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(mip_levels)
            .layer_count(1)
            .build();
        
        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .min_filter(vk::Filter::LINEAR)
            .mag_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(mip_levels as f32)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .unnormalized_coordinates(false);
        
        let hash_builder = RandomState::with_seed(42);
        let hash = hash_builder.hash_one(source_image.as_bytes());

        Ok(Self {
            path: pb,
            hash,
            info: image_info.build(),
            sampler_info: sampler_create_info.build(),
            extent,
            mip_levels,
            subresource_range,
            format,
            data: source_image,
            raw_data: Vec::new(),
        })
    }

    pub fn set_format(&mut self, format: vk::Format) {
        let format = format;
        let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(self.extent)
        .mip_levels(self.mip_levels)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
        )
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
        self.info = image_info.build();
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }
}


pub fn load_images_par<T: num::Num>(paths: &Vec<String>) -> Vec<Image<T>> {
    let mut images: Vec<Image<T>> = Vec::new();
    paths.into_par_iter().map(|path| {
        let res = Image::<T>::new(PathBuf::from(path));
        let image = match res {
            Ok(i) => i,
            Err(e) => {
                println!("Failed to load image: {}\n{}", path, e);
                return Image::<T>::new(PathBuf::from("assets/textures/missing.png")).unwrap();
            }
        };
        image
    }).collect_into_vec(&mut images);

    images
}

pub fn load_images<T: num::Num>(base_path: &Option<String>, image_paths: &Vec<String>) -> Vec<Image<T>> {
    match base_path {
        Some(base) => {
            let paths = image_paths.iter().map(|path| {
                base.clone() + &path.clone()
            }).collect();
            load_images_par::<T>(&paths)
        },
        None => {
            load_images_par::<T>(image_paths)
        }
    }
}