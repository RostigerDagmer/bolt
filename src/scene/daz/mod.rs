use std::{path::PathBuf, collections::HashSet};

pub use self::load::{read_from_dsf};
pub use super::mesh::VulkanMesh;
pub use self::dsf::DSF;
//pub use self::format::Convertable;

pub mod load;
pub mod dsf;
pub mod duf;
pub mod format;
pub mod skin;

pub use skin::*;

use urlencoding::decode;

pub fn parse_url(url: &String) -> (String, Option<String>) {
    let decoded = decode(url).unwrap();
    let mut split = decoded.split("#");
    let location = split.next().unwrap().to_string();
    let fragment = split.next().map(|s| s.to_string());

    (location, fragment)
}

pub fn unique_files(ref_files: Vec<String>) -> HashSet<String> {
    let unique_files = ref_files.iter().map(parse_url).map(|(x, _)| x).collect();
    unique_files
}

pub fn import(filepath: &PathBuf) -> Result<DSF, Box<dyn std::error::Error>> {
    read_from_dsf(filepath)
    // let mesh = match res {
    //     Ok(mesh) => mesh,
    //     Err(e) => return Err(e),
    // };

    // pub struct Mesh {
    //     pub context: Arc<Context>,
    //     pub name: String,
    //     pub vertex_buffer: Buffer,
    //     pub index_buffer: Option<Buffer>,
    //     pub index_storage: Option<Buffer>,
    //     pub transform: glam::Mat4,
    //     pub primitive_sections: Vec<PrimitiveSection>,
    // }
}