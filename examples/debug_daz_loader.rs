#![allow(dead_code)]
use std::error::Error;
use std::path::PathBuf;

use bolt;
use bolt::scene::daz::{format::*, read_from_dsf};
use glam::Mat4;
use url::Url;
use urlencoding::decode;
use debug_tree::*;


fn test_read_duf(asset: String) -> bolt::scene::daz::duf::DUF {
    let fpath = &bolt::util::find_asset(&asset).unwrap();
    let file = bolt::scene::daz::load::read_from_duf(fpath).unwrap();
    file
}

fn test_read_dsf(path: &PathBuf) -> Result<bolt::scene::daz::dsf::DSF, Box<dyn Error>> {
    // let fpath = &bolt::util::find_asset("models/Genesis9.dsf").unwrap();
    bolt::scene::daz::load::read_from_dsf(path) //.unwrap();
    //file
}

// fn test_rig_parsing() {
//     let file = test_read_dsf();
//     type RigSetup = DazRigParserV1<Mat4, BoneV1<Mat4>, RigV1<Mat4, BoneV1<Mat4>>>;
//     let rig = RigSetup::parse(&file).unwrap();
//     println!("bones: {:?}", rig.bone_map);

//     //root children
//     //let root_bone = rig.root_bone;
//     println!("pelvis parent: {:?}", rig.bones.get(1).unwrap().parent);

//     // println!{"len bonemap: {:?}", rig.bone_map.into_keys().len().clone()}
//     // println!{"len bones: {:?}", rig.bones.len()}
//     // println!{"root index: {:?}", rig.root_bone}
//     defer_print!();
//     add_branch!("Rig");
//     // println!("bone children count: {:?}", rig.get_bones()
//     // .iter()
//     // .map(|b| {
//     //     (b.name.clone(), b.get_child_count())
//     // })
//     // .collect::<Vec<(String, usize)>>());
//     let local_transforms = rig.into_iter().fold(Vec::new(), |mut acc, x| {
//         if x.get_child_count() == 0 {
//             add_leaf!("{}", x.get_name());
//         } else {
//             add_branch!("{}", x.get_name());
//         }
//         acc.push(x.get_local_transform().clone());
//         acc
//     });
    
//     println!("Transforms-----------------------------\n{:?}", local_transforms);
// }


fn test_material_parsing() {
    let daz_install_path = String::from("C:/Daz 3D/Applications/Data/DAZ 3D/My DAZ 3D Library");
    let file = test_read_duf("models/Genesis 9.duf".to_string());
    println!("nodes: {:?}", file.scene.nodes);
    let dsfs = match &file.scene.nodes {
        Some(nodes) => {
            let paths = nodes.iter().map( |node| {
                // println!("node: {:?}", node);
                //println!("node url: {:?}", node.url);
                //println!("node url decoded: {:?}", decode(&node.url).unwrap());
                // println!("node url decoded: {:?}", Url::parse(&decode(&(daz_install_path.clone() + &node.url)).unwrap()).unwrap());
                // println!("node url decoded: {:?}", Url::parse(&decode(&node.url).unwrap()).unwrap().path());
                // println!("node url decoded: {:?}", PathBuf::from(Url::parse(&decode(&node.url).unwrap()).unwrap().path()));
                let url = Url::parse(&decode(&(daz_install_path.clone() + &node.url)).unwrap()).unwrap();

                PathBuf::from(url.scheme().to_uppercase() + ":" + url.path())
            }).collect::<Vec<PathBuf>>();
            println!("paths: {:?}", paths);
            Some(paths.iter().filter_map(|path| match read_from_dsf(path) {
                Ok(dsf) => Some(dsf),
                Err(e) => {
                    println!("error reading dsf: {:?}", e);
                    None
                }
            }).collect::<Vec<bolt::scene::daz::dsf::DSF>>())
        },
        None => {println!("no nodes"); None},
    }.unwrap();
    println!("dsfs: {:?}", dsfs.iter().map(|dsf| dsf.asset_info.id.clone()).collect::<Vec<String>>());
    //println!("node materials: {:?}", file.scene.materials);
}   

fn main() {
    //let daz_install_path = String::from("C:/Daz 3D/Applications/Data/DAZ 3D/My DAZ 3D Library");
    // let fpath = &bolt::util::find_asset("models/Genesis 9.duf").unwrap();

    // let res = bolt::scene::daz::load::build_daz(fpath);
    // println!("{:?}", res);
    test_material_parsing();

}