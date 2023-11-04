use serde_json;
use serde_json::Value;
use serde::{Serialize, Deserialize};

//use super::mesh::{Vertex, Mesh};
//use super::format::Convertable;

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct AssetInfo {
        pub id: String,
        pub r#type: String,
        pub contributor: struct {
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
    pub struct Region {
        id: String,
        label: String,
        display_hint: String,
        map: Option<struct {
            count: u32,
            values: Vec<u32>,
        }>,
        children: Option<Vec<Region>>,
    }
}

// #[strikethrough[derive(Debug, Serialize, Deserialize)]
// struct MaterialSet {
//     name: String,
//     parent: Option<String>,
//     materials: Option<Vec<String>>,
// }

// #[strikethrough[derive(Debug, Serialize, Deserialize)]
// struct Extra {
//     r#type: String,
//     material_selection_sets: Vec<MaterialSet>,
// }

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct GeometryLibrary {
        pub id: String,
        pub name: String,
        pub id_aliases: Option<Vec<String>>,
        pub r#type: String,
        pub edge_interpolation_mode: String,
        pub subd_normal_smoothing_mode: String,
        pub vertices: struct {
            pub count: u32,
            pub values: Vec<[f32; 3]>,
        },
        pub polygon_groups: struct {
            pub count: u32,
            pub values: Vec<String>,
        },
        pub polygon_material_groups: struct {
            pub count: u32,
            pub values: Vec<String>,
        },
        pub polylist: struct {
            pub count: u32,
            pub values: Vec<Vec<u32>>,
        },
        pub default_uv_set: String,
        pub root_region: Region,
        pub graft: Option<Value>,
        pub extra: Vec<Value>
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Handle {
    pub id: String,
    pub r#type: String,
    pub name: String,
    pub label: String,
    pub auto_follow: Option<bool>,
    pub visible: Option<bool>,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub display_as_percent: Option<bool>,
    pub step_size: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Operation {
    op: String,
    val: Option<f32>,
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Formula {
    output: String,
    stage: Option<String>,
    operations: Vec<Operation>,
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct Node {
        pub id: String,
        pub name: String,
        pub id_aliases: Option<Vec<String>>,
        pub name_aliases: Option<Vec<String>>,
        pub extended_asset_ids: Option<Vec<String>>,
        pub r#type: String,
        pub label: String,
        pub parent: Option<String>,
        pub rotation_order: String,
        pub inherits_scale: bool,
        pub center_point: Vec<Handle>,
        pub end_point: Vec<Handle>,
        pub orientation: Vec<Handle>,
        pub rotation: Vec<Handle>,
        pub translation: Vec<Handle>,
        pub scale: Vec<Handle>,
        pub general_scale: Option<Handle>,
        pub formulas: Option<Vec<Formula>>,
        pub presentation: Option<pub struct {
            r#type: String,
            label: String,
            description: String,
            icon_large: String,
            colors: Vec<Vec<f32>>,
            auto_fit_base: String,
            extended_bases: Option<Vec<String>>,
        }>,
        pub extra: Vec<Value>,
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct Joint {
        pub id: String,
        pub node: String,
        pub node_weights: pub struct {
            count: i32,
            values: Vec<(i32, f32)>,
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymmetryMapping {
    pub id: String,
    pub mappings: Vec<(String, String)>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Skin {
    pub node: String,
    pub geometry: String,
    pub vertex_count: u32,
    pub joints: Vec<Joint>,
    pub selection_map: Vec<SymmetryMapping>,
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Serialize, Deserialize)]]
    pub struct Modifier {
        pub id: String,
        pub name: String,
        pub parent: String,
        pub skin: Skin,
        pub extra: Vec<pub struct {
            r#type: String,
            auto_normalize_general: bool,
            auto_normalize_local: bool,
            auto_normalize_scale: bool,
            binding_mode: String,
            general_map_mode: String,
            scale_mode: String,
        }>,
    }
}

// Daz Surface File
#[derive(Debug, Serialize, Deserialize)]
pub struct DSF {
    pub file_version: String,
    pub asset_info: AssetInfo,
    pub geometry_library: Vec<GeometryLibrary>,
    pub node_library: Vec<Node>,
    pub modifier_library: Vec<Modifier>,
}

// impl Convertable for DSF {
//     fn to_mesh(&self) -> Mesh {
//         Mesh {
//             vertices: self.geometry_library[0].vertices.values
//                 .iter()
//                 .map(|pos|
//                     Vertex{
//                         position: pos.clone(),
//                         ..Vertex::default()
//                     }
//                 )
//                 .collect(),
//             indices: self.geometry_library[0].polylist.values
//                 .iter()
//                 .fold(Vec::new(), |mut acc, quad| {
//                     // reorder
//                     acc.push(quad[2].clone());
//                     acc.push(quad[3].clone());
//                     acc.push(quad[4].clone());
//                     acc.push(quad[2].clone());
//                     acc.push(quad[5].clone());
//                     acc.push(quad[4].clone());
//                     acc
//                 })
//         }
//     }
// }
