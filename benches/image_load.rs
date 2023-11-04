use std::path::PathBuf;

use bolt::AppSettings;
use bolt::SharedContext;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ash::vk;
use bolt::scene::*;
use rayon::prelude::*;



// 25 textures in 2.8 - 3.5s
fn load_material_textures(base_path: &String, image_refs: &Vec<String>) {
    let images = image_refs.iter().map(|image_ref| {
        let fullpath = base_path.clone() + &image_ref.clone();
        let image_path = PathBuf::from(fullpath);
        let image = bolt::resource::image::Image::<u8>::new(image_path).unwrap();
        image
    }).collect::<Vec<_>>();
    println!("{}", images.len());
}

// 25 textures in 800 - 1000ms
fn load_material_textures_par(base_path: &String, image_refs: &Vec<String>) {
    let mut images: Vec<bolt::resource::image::Image<u8>> = Vec::new();
    image_refs.into_par_iter().map(|image_ref| {
        let fullpath = base_path.clone() + &image_ref.clone();
        let image_path = PathBuf::from(fullpath);
        let image = bolt::resource::image::Image::new(image_path).unwrap();
        image
    }).collect_into_vec(&mut images);

    println!("{}", images.len());
}


fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("image_load");
    group.sample_size(10);
    let daz_install_path = String::from("C:/Daz 3D/Applications/Data/DAZ 3D/My DAZ 3D Library");

    let fpath = &bolt::util::find_asset("models/G9 Feminine Skin 02 MAT.duf").unwrap();
    let file = bolt::scene::daz::load::read_from_duf(fpath).unwrap();
    let (external_references, internal_references) = file.get_file_refs();
    let referenced_files = bolt::scene::daz::unique_files(external_references);
    let image_refs = file.get_image_refs();
    let referenced_images = bolt::scene::daz::unique_files(image_refs).into_iter().collect();

    group.bench_function("load_seq", |b| b.iter(|| {
        load_material_textures(&daz_install_path, black_box(&referenced_images));
    }));
    group.bench_function("load_par", |b| b.iter(|| {
        load_material_textures_par(&daz_install_path, black_box(&referenced_images));
    }));
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);