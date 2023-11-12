#![allow(dead_code)]
use std::error::Error;
use std::path::PathBuf;

use bolt;
use bolt::resource::BreadthFirstIterator;
use bolt::resource::skin::{RigParser, Bone};
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

fn test_rig_parsing() {
    let path = &bolt::util::find_asset("models/Genesis9.dsf").unwrap();
    let file = test_read_dsf(path);
    type RigSetup = DazRigParserV1<RigV1<Mat4, BoneV1<Mat4>>, Mat4, BoneV1<Mat4>>;
    let rigs = RigSetup::parse(&file.unwrap());
    for rig in rigs {
        let rig = rig.unwrap();
        println!("bones: {:?}", rig.bone_map);
        for bone in rig.into_iter() {
            println!("bone: {:?}", bone.get_local_transform());
        }
    }

    
    // print all bones with their children

    // defer_print!();
    // add_branch!("Rig");

    // for (name, bone_id) in rig.bone_map.iter() {
    //     let bone = rig.bones.get(*bone_id).unwrap();
    //     add_branch!("{}", bone.get_name());
    //     if (bone.get_child_count() == 0) {
    //         add_leaf!("{}", bone.get_name());   
    //     }
    // }

    // rig.into_iter().for_each(|bone| {
    //     // add_branch!("{}", bone.get_name());
    //     // if (bone.get_child_count() == 0) {
    //     //     add_leaf!("{}", bone.get_name());   
    //     // }
    //     println!("bone: {:?}", bone.get_name());
    //     println!("bone children: {:?}", bone.get_children());   
    // }); 


    // let global_transforms = rig.into_iter().fold(Vec::new(), |mut acc, x| {
    //     let last_transform = acc.last();
    //     match last_transform {
    //         Some(t) => {
    //             acc.push(x.get_global_transform(t));
    //             acc
    //         },
    //         None => {
    //             acc.push(*x.global_transform());
    //             acc
    //         }
    //     }
    // });

    
    // println!("Transforms-----------------------------\n{:?}", global_transforms);
    
}


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


fn test_skins() {
    let path = &bolt::util::find_asset("models/Genesis9.dsf").unwrap();
    let file = test_read_dsf(path).unwrap();
    let skins = file.skins();

    println!("skins: {:?}", skins[0].transforms.iter().filter_map(|t| {
        if (*t != Mat4::IDENTITY) {
            Some(t)
        } else {
            None
        }
    }).collect::<Vec<_>>());
}

fn main() {
    //let daz_install_path = String::from("C:/Daz 3D/Applications/Data/DAZ 3D/My DAZ 3D Library");
    // let fpath = &bolt::util::find_asset("models/Genesis 9.duf").unwrap();

    // let res = bolt::scene::daz::load::build_daz(fpath);
    // println!("{:?}", res);
    // test_material_parsing();
    test_rig_parsing();
    // test_skins();

}