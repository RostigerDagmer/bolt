use std::marker::PhantomData;
use std::ops::Mul;
use std::{collections::HashMap, error::Error};
use std::fmt::{Debug, Formatter};

use super::DSF;
use super::dsf::Handle;
use glam::*;
use rayon::prelude::*;

pub trait Transform: Mul<Output = Self> + Sized + Clone + Send + Sync{
    fn from_mat4(mat: Mat4) -> Self;
    fn from_scale_rotation_translation(scale: Vec3, orientation:Vec3, rotation: Vec3, translation: Vec3) -> Self;
    fn get_position(&self) -> Vec3;
    fn get_rotation(&self) -> Vec3;
    fn get_scale(&self) -> Vec3;
    fn get_matrix(&self) -> Mat4;
    fn get_inverse(&self) -> Mat4;
    fn get_inverse_transpose(&self) -> Mat4;
    fn get_d_quat(&self) -> (Quat, Quat);
    fn zero() -> Self;
}

impl Transform for Mat4 {
    fn from_mat4(mat: Mat4) -> Self {
        mat
    }
    fn from_scale_rotation_translation(scale: Vec3, orientation:Vec3, rotation: Vec3, translation: Vec3) -> Self {
        Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_arc(orientation, rotation), translation)
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
    fn get_inverse(&self) -> Mat4 {
        self.inverse()
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
    fn get_global_transform(&self, chain_transform: T) -> T;
    fn add_child(&mut self, child: String);
}

pub trait Rig<S: Transform, T: Bone<S>>: IntoIterator<Item = T> + Clone {
    fn get_root_bone(&self) -> &T;
    fn get_bones(&self) -> &Vec<T>;
    fn get_bone(&self, name: &str) -> Option<&T>;
    fn get_bone_by_id(&self, id: &str) -> Option<&T>;
    fn get_bone_by_index(&self, index: usize) -> Option<&T>;
    fn get_bone_count(&self) -> usize;
}

type GenericRig<S, T> = Box<dyn Rig<S, T, IntoIter = DepthFirstIterator<S, T>>>;
pub trait RigParser<S: Transform, T: Bone<S>, V: Rig<S, T>> {
    fn parse(file: &DSF) -> Result<V, Box<dyn Error>>;
}

pub struct DazRigParserV1<S: Transform, T: Bone<S>, V> {
    rig: PhantomData<V>,
    transform: PhantomData<S>,
    bone: PhantomData<T>,
}

fn handle_to_vec(handle: Vec<Handle>) -> Vec3 {
    Vec3::new(handle[0].value, handle[1].value, handle[2].value)
}

impl<S: Transform + 'static, T: Bone<S> + 'static> RigParser<S, T, RigV1<S, T>> for DazRigParserV1<S, T, RigV1<S, T>> {

    fn parse(file: &DSF) -> Result<RigV1<S,T>, Box<dyn Error>> {
        let mut bone_map: HashMap<String, usize> = HashMap::new();
        let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
        let bones = file
        .node_library
        .iter()
        .filter(|n| n.r#type == "bone")
        .collect::<Vec<&super::dsf::Node>>()
        .par_iter()
        .enumerate()
        .map(|(i, n)| {
            let parent = if let Some(parent) = &n.parent {
                // parent is a fragment (#name)
                // we have to strip the hash
                let parent_name = parent.strip_prefix("#");
                parent_name
                // let parent_idx = bone_map.get(parent_name.unwrap_or(parent));
                // parent_idx.map(|idx| *idx)
            } else {
                // root_bone = i;
                None
            };
            let bone = T::from(
                n.name.clone(),
                n.id.clone(),
                n.label.clone(),
                i,
                None,
                Vec::new(),
                handle_to_vec(n.center_point.clone()),
                handle_to_vec(n.end_point.clone()),
                S::from_scale_rotation_translation(
                    handle_to_vec(n.scale.clone()),
                    handle_to_vec(n.orientation.clone()),
                    handle_to_vec(n.rotation.clone()),
                    handle_to_vec(n.translation.clone()),
                ),
                S::zero(), // this can not be know before sweeping through the hierarchy once
            );
            // bone_map.insert(n.id.clone(), i);
            (bone, parent)
        })
        .collect::<Vec<(T, Option<&str>)>>();
        if bones.is_empty() {
            return Err("No rig found in file".into());
        }
        // collect bones vector into bone_map and add parents
        let mut bones = bones.iter().map(|(bone, parent)| {
            if let Some(parent) = parent.map(|p| p.to_string()) {
                let mut b = bone.clone();
                b.set_parent(bone_map.get(&parent).copied());
                bone_map.insert(bone.get_id().to_string(), bone.get_index());
                return b
            }
            bone.clone()
            
        }).collect::<Vec<_>>();

        //let (bones, _) = bones.into_iter().unzip::<T, Option<&str>, Vec<T>, Vec<Option<&str>>>();
        //collect children into children_map
        for bone in bones.iter() {
            if let Some(parent) = bone.get_parent() {
                let parent_name = bones.get(parent).unwrap().get_name().to_string();
                children_map.entry(parent_name).or_insert(Vec::new()).push(bone.get_name().to_string());
            }
        }

        // sweep through the hierarchy and add children to parents
        for (parent_name, children) in children_map {
            let parent_idx = bone_map.get(&parent_name);
            match parent_idx {
                Some(idx) => {
                    let parent = bones.get_mut(*idx).unwrap();
                    for child in children {
                        parent.add_child(child);
                    }
                },
                None => {
                    println!(" (X) couldn't index {:?} in bone array at [{:?}] even though it was retrieved from bone map", parent_name, parent_idx);
                }
            }
        }

        Ok(RigV1 {
            bone_map,
            bones: bones,
            root_transform: S::zero(),
            root_bone: 0,
        })
    }
}



#[derive(Clone)]
pub struct RigV1<S: Transform, T: Bone<S>> {
    pub bone_map: HashMap<String, usize>,
    pub bones: Vec<T>,
    pub root_bone: usize,
    pub root_transform: S,
}

impl<S: Transform, T: Bone<S>> IntoIterator for RigV1<S, T> {
    type Item = T;
    type IntoIter = BreadthFirstIterator<S, T>;

    fn into_iter(self) -> Self::IntoIter {
        let start = self.root_bone;
        BreadthFirstIterator {
            rig: self,
            visited: Vec::new(),
            queue: vec![start],
        }
    }
}

pub struct BreadthFirstIterator<S: Transform, T: Bone<S>> {
    rig: RigV1<S, T>,
    visited: Vec<usize>,
    queue: Vec<usize>,
}

pub struct DepthFirstIterator<S: Transform, T: Bone<S>> {
    rig: RigV1<S, T>,
    visited: Vec<usize>,
    queue: Vec<usize>,
}

impl<S: Transform, T: Bone<S>> Iterator for BreadthFirstIterator<S, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.queue.is_empty() {
            return None;
        }
        let next = self.queue.remove(0);
        let bone = self.rig.bones[next].clone();
        self.visited.push(next);
        for child in bone.get_children() {
            let id = self.rig.get_bone(&child).unwrap().get_index();
            if !self.visited.contains(&id) {
                self.queue.push(id);
            }
        }
        Some(bone)
    }
}

impl<S: Transform, T: Bone<S>> Iterator for DepthFirstIterator<S, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.queue.is_empty() {
            return None;
        }
        let next = self.queue.remove(0);
        let bone = self.rig.bones[next].clone();
        self.visited.push(next);
        for child in bone.get_children() {
            let id = self.rig.get_bone(&child).unwrap().get_index();
            if !self.visited.contains(&id) {
                self.queue.insert(0, id);
            }
        }
        Some(bone)
    }
}

// impl Debug for RigV1<Mat4, BoneV1<Mat4>> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("RigV1")
//             .field("bones", &self.bones)
//             .finish()
//     }
// }

impl<S: Transform, T: Bone<S>> Rig<S, T> for RigV1<S, T> {
    fn get_root_bone(&self) -> &T {
        &self.bones[self.root_bone]
    }
    fn get_bones(&self) -> &Vec<T> {
        &self.bones
    }
    fn get_bone(&self, name: &str) -> Option<&T> {
        self.bones.iter().find(|b| b.get_name() == name)
    }
    fn get_bone_by_id(&self, id: &str) -> Option<&T> {
        self.bones.iter().find(|b| b.get_id() == id)
    }
    fn get_bone_by_index(&self, index: usize) -> Option<&T> {
        self.bones.iter().find(|b| b.get_index() == index)
    }
    fn get_bone_count(&self) -> usize {
        self.bones.len()
    }
}

impl<T: Transform, S: Bone<T>> RigV1<T, S> {
    pub fn local_to_global(&self, start_idx: usize) -> T {
        let mut acc = T::zero();
        let mut bone_index = start_idx;
        loop {
            acc = acc.mul(self.bones[bone_index].get_local_transform().to_owned());
            match self.bones[bone_index].get_parent() {
                Some(parent) => {
                    bone_index = parent;
                },
                None => break,
            }
        }
        acc
    }
}

// FROM DSF
// "id" : "l_thigh",
// "name" : "l_thigh",
// "type" : "bone",
// "label" : "Left Thigh",
// "parent" : "#pelvis",
// "rotation_order" : "YZX",
// "inherits_scale" : false,
#[derive(Debug, Clone)]
pub struct BoneV1<T: Transform> {
    pub name: String,
    pub id: String,
    pub label: String,
    pub index: usize,
    pub parent: Option<usize>,
    pub children: Vec<String>,
    pub center_point: Vec3,
    pub end_point: Vec3,
    pub local_transform: T,
    pub global_transform: T,
}

impl<T: Transform + Mul> Bone<T> for BoneV1<T> {
    fn from(
        name: String,
        id: String,
        label: String,
        index: usize,
        parent: Option<usize>,
        children: Vec<String>,
        center_point: Vec3,
        end_point: Vec3,
        local_transform: T,
        global_transform: T,
    ) -> Self {
        BoneV1 {
            name,
            id,
            label,
            index,
            parent,
            children,
            center_point,
            end_point,
            local_transform,
            global_transform,
        }
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_id(&self) -> &str {
        &self.id
    }
    fn get_index(&self) -> usize {
        self.index
    }
    fn get_parent(&self) -> Option<usize> {
        self.parent
    }
    fn set_parent(&mut self, parent: Option<usize>) {
        self.parent = parent;
    }
    fn get_children(&self) -> Vec<String> {
        self.children.clone()
    }
    fn get_child_count(&self) -> usize {
        self.children.len()
    }
    fn get_local_transform(&self) -> &T {
        &self.local_transform
    }
    fn get_global_transform(&self, chain_transform: T) -> T {
        chain_transform.mul(self.local_transform.clone())
    }
    fn add_child(&mut self, child: String) {
        self.children.push(child);
    }
}

#[derive(Clone)]
pub struct TransformV1 {
    pub orientation: Vec3,
    pub rotation: Vec3,
    pub translation: Vec3,
    pub scale: Vec3,
}

impl Mul for TransformV1 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::from_mat4(self.get_matrix().mul_mat4(&rhs.get_matrix()))
    }
}

impl Transform for TransformV1 {
    fn from_mat4(mat: Mat4) -> Self {
        let (scale, rotation, translation) = mat.to_scale_rotation_translation();
        let w = rotation.w;
        let r_ = rotation.xyz();
        let phi = 2.0 * w.acos();
        let orientation = r_ / (phi / 2.0).sin();
        let rotation = orientation * phi;
        TransformV1 {
            orientation,
            rotation,
            translation,
            scale,
        }
    }
    fn from_scale_rotation_translation(scale: Vec3, orientation:Vec3, rotation: Vec3, translation: Vec3) -> Self {
        TransformV1 {
            orientation,
            rotation,
            translation,
            scale,
        }
    }
    fn get_position(&self) -> Vec3 {
        self.translation
    }
    fn get_rotation(&self) -> Vec3 {
        self.rotation
    }
    fn get_scale(&self) -> Vec3 {
        self.scale
    }
    fn get_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, Quat::from_rotation_arc(self.orientation, self.rotation), self.translation)
    }
    fn get_inverse(&self) -> Mat4 {
        self.get_matrix().inverse()
    }
    fn get_inverse_transpose(&self) -> Mat4 {
        self.get_matrix().inverse().transpose()
    }
    fn get_d_quat(&self) -> (Quat, Quat) {
        self.get_matrix().get_d_quat()
    }

    fn zero() -> Self {
        Self {
            orientation: Vec3::ZERO,
            rotation: Vec3::ZERO,
            translation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

