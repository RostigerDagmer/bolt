use serde_json;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use super::dsf::DSF;
use super::duf::DUF;

#[derive(Debug)]
pub enum DazComponentFile {
    Dsf(DSF),
    Duf(DUF),
}

pub fn read_from_dsf(path: &Path) -> Result<DSF, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;

    Ok(data)
}

pub fn read_from_duf(path: &Path) -> Result<DUF, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;

    Ok(data)
}

fn read_daz_component_file<'a>(path: &Path) -> Result<DazComponentFile, Box<dyn Error>>{
    let extension = path.extension().unwrap().to_str().unwrap();
    match extension {
        "dsf" => {
            let dsf = read_from_dsf(path);
            match dsf {
                Ok(dsf) => Ok(DazComponentFile::Dsf(dsf)),
                Err(e) => Err(e),
            }
            
        }
        "duf" => {
            let duf = read_from_duf(path);
            match duf {
                Ok(duf) => Ok(DazComponentFile::Duf(duf)),
                Err(e) => Err(e),
            }
        }
        _ => Err("Unsupported file type.".into()),
    }
}

pub fn build_daz(from: &PathBuf) -> Result<(), Box<dyn Error>> {
    // TODO: make this a config option
    let daz_install_path = String::from("C:/Daz 3D/Applications/Data/DAZ 3D/My DAZ 3D Library");

    if from.extension() != Some("duf".as_ref()) {
        return Err("Daz models can only be built from DUF entry points.".into());
    }
    
    // check if the loader failed
    let entry = read_daz_component_file(from);
    if let Err(e) = entry {
        return Err(e);
    }
    // check if something is very wrong
    let duf = match entry.unwrap() {
        DazComponentFile::Duf(duf) => duf,
        _ => return Err("Daz models can only be built from DUF entry points.".into()),
    };

    let mut scene:HashMap<String, &DazComponentFile> = HashMap::new();
    let key = duf.asset_info.id.clone();

    // collect references to dsf files in the duf
    let (external_references, internal_references) = duf.get_file_refs();
    let image_refs = duf.get_image_refs();
    let referenced_files = crate::scene::daz::unique_files(external_references);
    
    scene.insert(key, &DazComponentFile::Duf(duf));
    println!("file refs: {:#?}", referenced_files);

    // let linked_files = referenced_files.iter().map(|x| {
    //     let mut path = from.parent().unwrap().to_path_buf();
    //     path.push(x);
    //     path
    // });


    
    let referenced_images = crate::scene::daz::unique_files(image_refs);
    let images = crate::resource::image::load_images::<u8>(&Some(daz_install_path), &referenced_images.iter().map(|i| i.clone()).collect());
    
    println!("image refs: {:#?}", referenced_images);
    //println!("scene duf root: {:#?}", scene.get(&key).unwrap());

    Ok(())
}

