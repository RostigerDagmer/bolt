use glam::*;
use std::error::Error;
use std::ops::{Mul, Add};
use std::fmt::Debug;

use crate::scene::daz::dsf;

pub trait Transform: Mul<Output = Self> + Add<Output = Self> + Sized + Clone + Copy + Send + Sync + Debug {
    fn from_mat4(mat: Mat4) -> Self;
    fn from_scale_rotation_translation(scale: Vec3, orientation:Vec3, rotation: Vec3, translation: Vec3, rotation_order: RotationOrder) -> Self;
    fn get_position(&self) -> Vec3;
    fn get_rotation(&self) -> Vec3;
    fn get_scale(&self) -> Vec3;
    fn get_matrix(&self) -> Mat4;
    fn get_inverse<T: Transform>(&self) -> T;
    fn get_inverse_transpose(&self) -> Mat4;
    fn get_d_quat(&self) -> (Quat, Quat);
    fn zero() -> Self;
    fn identity() -> Self {
        Self::from_mat4(Mat4::IDENTITY)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RotationOrder {
    XYZ,
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

impl Into<RotationOrder> for dsf::RotationOrder {
    fn into(self) -> RotationOrder {
        match self {
            dsf::RotationOrder::XYZ => RotationOrder::XYZ,
            dsf::RotationOrder::XZY => RotationOrder::XZY,
            dsf::RotationOrder::YXZ => RotationOrder::YXZ,
            dsf::RotationOrder::YZX => RotationOrder::YZX,
            dsf::RotationOrder::ZXY => RotationOrder::ZXY,
            dsf::RotationOrder::ZYX => RotationOrder::ZYX,
        }
    }   
}

fn apply_rotation_order(rotation_order: RotationOrder, rX: Mat4, rY: Mat4, rZ: Mat4) -> Mat4 {
    match rotation_order {
        RotationOrder::XYZ => rX * rY * rZ,
        RotationOrder::XZY => rX * rZ * rY,
        RotationOrder::YXZ => rY * rX * rZ,
        RotationOrder::YZX => rY * rZ * rX,
        RotationOrder::ZXY => rZ * rX * rY,
        RotationOrder::ZYX => rZ * rY * rX,
    }
}

impl Transform for Mat4 {
    fn from_mat4(mat: Mat4) -> Self {
        mat
    }
    fn from_scale_rotation_translation(scale: Vec3, orientation:Vec3, rotation: Vec3, translation: Vec3, rotation_order: RotationOrder) -> Self {
        // would be nice if this just worked vvvv
        // Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_arc(rotation, orientation), translation)
        
        let cos_x = rotation.x.cos();
        let cos_y = rotation.y.cos();
        let cos_z = rotation.z.cos();
        let sin_x = rotation.x.sin();   
        let sin_y = rotation.y.sin();
        let sin_z = rotation.z.sin();

        let rX = glam::mat4(
            vec4(1.0, 0.0, 0.0, 0.0),
            vec4(0.0, cos_x, -sin_x, 0.0),
            vec4(0.0, sin_x, cos_x, 0.0), 
            vec4(0.0, 0.0, 0.0, 0.0) // <- zero because we add the translation later
            );
        let rY = glam::mat4(
            vec4(cos_y, 0.0, sin_y, 0.0),
            vec4(0.0, 1.0, 0.0, 0.0),
            vec4(-sin_y, 0.0, cos_y, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0)
        );
        
        let rZ = glam::mat4(
            vec4(cos_z, -sin_z, 0.0, 0.0),
            vec4(sin_z, cos_z, 0.0, 0.0),
            vec4(0.0, 0.0, 1.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0)
        );

        apply_rotation_order(rotation_order, rX, rY, rZ) * Mat4::from_scale(scale) + Mat4::from_translation(translation)

    }
    fn get_position(&self) -> Vec3 {
        self.w_axis.xyz()
    }
    fn get_rotation(&self) -> Vec3 {
        self.w_axis.xyz()
    }
    fn get_scale(&self) -> Vec3 {
        self.w_axis.xyz()
    }
    fn get_matrix(&self) -> Mat4 {
        *self
    }
    fn get_inverse<T: Transform>(&self) -> T {
        T::from_mat4(self.inverse())
    }
    fn get_inverse_transpose(&self) -> Mat4 {
        self.inverse().transpose()
    }
    fn get_d_quat(&self) -> (Quat, Quat) {
        let real = glam::Quat::from_mat4(self);
        // super ugly because the type stored in the matrix is not known
        let mut v = glam::Vec4::ZERO;
        let z = self.z_axis.xyz();
        v[1] = z.x;
        v[2] = z.y;
        v[3] = z.z;
        let dual = glam::Quat::from_vec4(v);
        (real, dual)
    }
    fn zero() -> Self {
        Mat4::IDENTITY
    }
}


pub trait Bone<T: Transform>: Clone + Send + Sync {
    fn from(name: String, id: String, label: String, index: usize, parent: Option<usize>, children: Vec<String>, center_point: Vec3, end_point: Vec3, local_transform: T, global_transform: T) -> Self;
    fn get_name(&self) -> &str;
    fn get_id(&self) -> &str;
    fn get_index(&self) -> usize;
    fn get_parent(&self) -> Option<usize>;
    fn set_parent(&mut self, parent: Option<usize>);
    fn get_children(&self) -> Vec<String>;
    fn get_child_count(&self) -> usize;
    fn get_local_transform(&self) -> &T;
    fn set_local_transform(&mut self, local_transform: T);
    fn set_inverse_bind_matrix(&mut self, inverse_bind_matrix: T);
    fn get_global_transform(&self, chain_transform: &T) -> T;
    fn set_global_transform(&mut self, global_transform: T);
    fn global_transform(&self) -> &T;
    fn add_child(&mut self, child: String);
}

pub trait Rig<S: Transform, T: Bone<S>>: IntoIterator<Item = T> + Clone {
    fn get_root_bone(&self) -> &T;
    fn get_bones(&self) -> &Vec<T>;
    fn get_bone(&self, name: &str) -> Option<&T>;
    fn get_bone_by_id(&self, id: &str) -> Option<&T>;
    fn get_bone_by_index(&self, index: usize) -> Option<&T>;
    fn get_bone_count(&self) -> usize;
    fn local_to_global(&self, index: usize) -> S;
}


pub trait RigParser<R: Rig<S, T>, S: Transform, T: Bone<S>> {
    fn parse(file: &dsf::DSF) -> Vec<Result<R, Box<dyn Error>>>;
}