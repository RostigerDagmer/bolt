use serde_json;
use serde_json::Value;
use serde::{Serialize, Deserialize};
// use super::dsf::Node;

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct AssetInfo {
        pub id: String,
        pub r#type: String,
        contributor: struct {
            author: String,
            email: String,
            website: String,
        },
        pub revision: String,
        pub modified: String,
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct ImageRefs {
        pub id: String,
        pub name: String,
        pub map_gamma: f32,
        pub map: Vec<pub struct {
            pub url: String,
            pub label: String,
        }>,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialChannel {
    id: String,
    r#type: String,
    name: String,
    label: Option<String>,
    value: [f32; 3],
    current_value: [f32; 3],
    min: f32,
    max: f32,
    clamped: bool,
    step_size: f32,
    default_image_gamma: f32,
    mappable: bool, 
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct DiffuseMaterial {
        channel: MaterialChannel,
        group: String,
        presentation: struct {
            r#type: String,
            label: String,
            description: String,
            icon_large: String,
            colors: Vec<[f32; 3]>,
        }
    }
}


structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct Material {
        pub id: String,
        pub diffuse: DiffuseMaterial,
        pub extra: Vec<Value>
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoIdentifier {
    pub id: String,
    pub url: String,
    pub name: String,
    pub label: String,
    pub r#type: String,
    pub current_subdivision_level: u8,
    pub edge_interpolation_mode: String,
    pub subd_normal_smoothing_mode: String,
    pub extra: Vec<Value>,
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct Preview {
        pub r#type: String,
        pub oriented_box: Option<pub struct {
            min: [f32; 3],
            max: [f32; 3],
        }>,
        pub center_point: [f32; 3],
        pub end_point: [f32; 3],
        pub rotation_order: String,
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct SceneNode {
        pub id: String,
        pub url: String,
        pub r#type: Option<String>,
        pub parent: Option<String>,
        pub name: String,
        pub label: String,
        pub geometries: Option<Vec<GeoIdentifier>>,
        pub preview: Preview,
        pub extra: Vec<Value>,
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct MaterialNode {
        pub id: Option<String>,
        pub url: String,
        pub geometry: Option<String>,
        pub groups: Vec<String>,
        pub diffuse: pub struct {
            pub channel: pub struct {
                pub id: String,
                pub r#type: String,
                pub name: String,
                pub value: [f32; 3],
                pub current_value: Option<[f32; 3]>,
                pub image: Option<String>,
            },
        },
        pub uv_set: Option<String>, // url
        pub extra: Vec<Value>,
    }
}



structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct Scene {
        pub nodes: Option<Vec<SceneNode>>,
        pub materials: Option<Vec<MaterialNode>>,
        pub modifiers: Option<Vec<Value>>,
        pub extra: Option<Vec<Value>>,
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct DUF {
        pub file_version: String,
        pub asset_info: AssetInfo,
        pub image_library: Option<Vec<ImageRefs>>,
        pub material_library: Option<Vec<Material>>,
        pub scene: Scene,
    }
}


impl DUF {
    // TODO: make a return enum for this
    pub fn get_file_refs(&self) -> (Vec<String>, Vec<String>) {
        let mut external_refs = Vec::new();
        let mut in_file_refs = Vec::new();
        // check all referenced scene nodes
        if let Some(scene_nodes) = &self.scene.nodes {
            for node in scene_nodes {
                if node.url.starts_with("#") {
                    in_file_refs.push(node.url.to_string());
                } else {
                    external_refs.push(node.url.to_string());
                }
            }
        };
        // check all referenced material nodes
        if let Some(material_nodes) = &self.scene.materials {
            for node in material_nodes {
                // filter in file references
                if node.url.starts_with("#") {
                    in_file_refs.push(node.url.to_string());
                } else {
                    external_refs.push(node.url.to_string());
                }
            }
        }
        (external_refs, in_file_refs)
    }

    pub fn get_image_refs(&self) -> Vec<String> {
        let mut refs = Vec::new();
        // check references if image library is present

        if let Some(images) = &self.image_library {
            for image in images {
                for map in &image.map {
                    refs.push(map.url.to_string());
                }
            }
        }
        refs
    }
}

