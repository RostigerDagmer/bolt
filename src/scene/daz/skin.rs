

use std::collections::HashMap;
use glam::{Vec3, Mat4};

use crate::resource::skin::Skin;
use crate::resource::skin::SkinJoint;
use super::dsf::DSF;
use super::dsf::Joint as DsfJoint;
use super::dsf::Node;

impl Node {

    pub fn transform_mat4(&self) -> Mat4 {
        let translation = Vec3::new(self.translation[0].value, self.translation[1].value, self.translation[2].value);
        let rotation = Vec3::new(self.rotation[0].value, self.rotation[1].value, self.rotation[2].value);
        let scale = Vec3::new(self.scale[0].value, self.scale[1].value, self.scale[2].value);

        let translation_matrix = Mat4::from_translation(translation);
        let scale_matrix = Mat4::from_scale(scale);
        
        // Create rotation matrices
        let x_rot = Mat4::from_rotation_x(rotation.x);
        let y_rot = Mat4::from_rotation_y(rotation.y);
        let z_rot = Mat4::from_rotation_z(rotation.z);

        // TODO: Multiply rotation matrices in correct order defined by "rotation_order"
        let rotation_matrix = x_rot * y_rot * z_rot;

        // compose translation, rotation and scaling matrices into transform matrix
        let transformation_matrix = translation_matrix * rotation_matrix * scale_matrix;
        
        transformation_matrix
    }
}

impl DSF {
    // get skins directly.
    // use rigs -> rig.into alternatively to hold onto a rig and use it to get skins
    pub fn skins(self) -> Vec<Skin> {
        let node_map = self.node_library
            .into_iter()
            .map(|node| (node.id.clone(), node))
            .collect::<HashMap<String, Node>>();

        // println!("node_map: {:?}", node_map);
        self.modifier_library.iter().filter(|m| m.skin.is_some()).map(|m| {
            let mut joints = Vec::new();
            let mut transforms = Vec::new();
            let mut bone_transforms = Vec::new();
            let mut inverse_bind_matrices = Vec::new();
            let skin = m.skin.as_ref().unwrap();
            let joint_id_map: HashMap<String, u32> = skin.joints
                .iter()
                .enumerate()
                .map(|(i, DsfJoint { id, node, node_weights })| {
                    // The joint transform is a Matrix4x4 which is not provided in DsfJoint
                    // You need to find a way to calculate or fetch it. Here it's represented by a placeholder identity matrix for the sake of the example.
                    let tr = node_map.get(id);
                    // println!("id {:?}", id);
                    // println!("node {:?}", node);
                    // println!("tr {:?}", tr);
                    match tr {
                        Some(node) => {
                            transforms.push(node.transform_mat4());
                            bone_transforms.push(node.transform_mat4());
                            inverse_bind_matrices.push(Mat4::IDENTITY);
                        },
                        None => {
                            println!("node not found: {:?}", id);
                            transforms.push(Mat4::IDENTITY);
                        }
                    }
    
                    // Flatten the node_weights values into SkinJoint and collect into joints
                    joints.extend(node_weights.values.iter().map(|(vertex_id, weight)| SkinJoint {
                        joint_id: i as u32,
                        vertex_id: *vertex_id as u32,
                        weight: *weight,
                    }));
    
                    (id.clone(), i as u32)
                })
                .collect();
            Skin {
                name: m.name.clone(),
                transforms,
                global_bone_transforms: bone_transforms,
                joints,
                joint_id_map,
                inverse_bind_matrices,
            }   
        }).collect()
    }
}