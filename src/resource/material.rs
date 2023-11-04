use std::{path::PathBuf, fmt};

use glam;
use super::image::Image;

pub struct Material<T: num::Num> {
    pub id: String,
    pub color: glam::Vec3,
    pub albedo: Option<Image<T>>,
    pub sss: Option<Image<T>>,
    pub normal: Option<Image<T>>,
    pub roughness: Option<Image<T>>,
    pub metallic: Option<Image<T>>,
    pub ao: Option<Image<T>>,
    pub emissive: Option<Image<T>>,
    pub opacity: Option<Image<T>>,
    pub displacement: Option<Image<T>>,
    pub displacement_scale: f32,
    pub displacement_bias: f32,

    pub albedo_factor: glam::Vec3,
    pub sss_factor: glam::Vec3,
    pub normal_factor: f32,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub ao_factor: f32,
    pub emissive_factor: glam::Vec3,
    pub opacity_factor: f32,

}

impl<T:num::Num> Default for Material<T> {
    fn default() -> Self {
        Material {
            id: String::new(),
            color: glam::vec3(0.8, 0.8, 0.8),
            albedo: None,
            sss: None,
            normal: None,
            roughness: None,
            metallic: None,
            ao: None,
            emissive: None,
            opacity: None,
            displacement: None,
            displacement_scale: 0.0,
            displacement_bias: 0.0,

            albedo_factor: glam::vec3(1.0, 1.0, 1.0),
            sss_factor: glam::vec3(1.0, 1.0, 1.0),
            normal_factor: 1.0,
            roughness_factor: 1.0,
            metallic_factor: 1.0,
            ao_factor: 1.0,
            emissive_factor: glam::vec3(1.0, 1.0, 1.0),
            opacity_factor: 1.0,
        }
    }
}

impl<T: num::Num> Material<T> {
    pub fn new(id: String) -> Self {
        Material {
            id,
            ..Default::default()
        }
    }

    pub fn texture_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(ref albedo) = self.albedo {
            paths.push(albedo.path.clone());
        }
        if let Some(ref sss) = self.sss {
            paths.push(sss.path.clone());
        }
        if let Some(ref normal) = self.normal {
            paths.push(normal.path.clone());
        }
        if let Some(ref roughness) = self.roughness {
            paths.push(roughness.path.clone());
        }
        if let Some(ref metallic) = self.metallic {
            paths.push(metallic.path.clone());
        }
        if let Some(ref ao) = self.ao {
            paths.push(ao.path.clone());
        }
        if let Some(ref emissive) = self.emissive {
            paths.push(emissive.path.clone());
        }
        if let Some(ref opacity) = self.opacity {
            paths.push(opacity.path.clone());
        }
        if let Some(ref displacement) = self.displacement {
            paths.push(displacement.path.clone());
        }
        paths
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MaterialInfo {
    pub color: glam::Vec3,
    pub displacement_scale: f32,
    pub displacement_bias: f32,
    pub albedo_factor: glam::Vec3,
    pub sss_factor: glam::Vec3,
    pub normal_factor: f32,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub ao_factor: f32,
    pub emissive_factor: glam::Vec3,
    pub opacity_factor: f32,
    // texture indices
    pub albedo: i32,
    pub sss: i32,
    pub normal: i32,
    pub roughness: i32,
    pub metallic: i32,
    pub ao: i32,
    pub emissive: i32,
    pub opacity: i32,
    pub displacement: i32,
} // size: 128 bytes

impl Default for MaterialInfo {
    fn default() -> Self {
        MaterialInfo {
            color: glam::vec3(0.8, 0.8, 0.8),
            displacement_scale: 0.0,
            displacement_bias: 0.0,
            albedo_factor: glam::vec3(1.0, 1.0, 1.0),
            sss_factor: glam::vec3(1.0, 1.0, 1.0),
            normal_factor: 1.0,
            roughness_factor: 1.0,
            metallic_factor: 1.0,
            ao_factor: 1.0,
            emissive_factor: glam::vec3(1.0, 1.0, 1.0),
            opacity_factor: 0.3,
            albedo: -1,
            sss: -1,
            normal: -1,
            roughness: -1,
            metallic: -1,
            ao: -1,
            emissive: -1,
            opacity: -1,
            displacement: -1,
        }
    }
}

impl MaterialInfo  {
    pub fn new<T: num::Num>(material: &Material<T>) -> Self {
        MaterialInfo {
            color: material.color,
            displacement_scale: material.displacement_scale,
            displacement_bias: material.displacement_bias,
            albedo_factor: material.albedo_factor,
            sss_factor: material.sss_factor,
            normal_factor: material.normal_factor,
            roughness_factor: material.roughness_factor,
            metallic_factor: material.metallic_factor,
            ao_factor: material.ao_factor,
            emissive_factor: material.emissive_factor,
            opacity_factor: material.opacity_factor,
            ..Default::default()
        }
    }
    pub fn size() -> usize {
        return 20 * std::mem::size_of::<f32>();
    }
    pub fn set_albedo(&mut self, index: i32) {
        self.albedo = index
    }
    pub fn set_sss(&mut self, index: i32) {
        self.sss = index
    }
    pub fn set_normal(&mut self, index: i32) {
        self.normal = index
    }
    pub fn set_roughness(&mut self, index: i32) {
        self.roughness = index
    }
    pub fn set_ao(&mut self, index: i32) {
        self.ao = index
    }
    pub fn set_emissive(&mut self, index: i32) {
        self.emissive = index
    }
    pub fn set_opacity(&mut self, index: i32) {
        self.opacity = index
    }
    pub fn set_displacement(&mut self, index: i32) {
        self.displacement = index
    }
}

impl<'a> From<gltf::Material<'a>> for MaterialInfo {
    fn from(mat: gltf::Material) -> Self {
        MaterialInfo {
            color: glam::Vec3::from_slice(
                &mat.pbr_metallic_roughness().base_color_factor()[0..3],
            ),
            //double_sided: mat.double_sided(),
            metallic_factor: mat.pbr_metallic_roughness().metallic_factor(),
            roughness_factor: mat.pbr_metallic_roughness().roughness_factor(),
            emissive_factor: glam::Vec3::from_slice(&mat.emissive_factor()),
            albedo: mat.pbr_metallic_roughness()
            .base_color_texture()
            .map(|s| {
                let i = s.texture().index();
                i as i32
            })
            .unwrap_or(-1),
            normal: mat.normal_texture()
            .map(|t| t.texture().index() as i32)
            .unwrap_or(-1),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum MaterialError {
    AlreadyRegistered(String),
    MissingTexture(String),
    InvalidTexture(String),
    InvalidValue(String),
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}