use std::path::PathBuf;

use bolt::AppSettings;
use bolt::SharedContext;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ash::vk;
use bolt::scene::*;
use rayon::prelude::*;
use bolt::scene::daz::format::*;
use bolt::scene::daz::DSF;
use glam::Mat4;

// single threaded one rig 540us
// multi threaded one rig 250us
fn test_parse(file: &DSF) {
    type RigSetup = DazRigParserV1<Mat4, BoneV1<Mat4>, RigV1<Mat4, BoneV1<Mat4>>>;
    let res = RigSetup::parse(black_box(&file));
    // match res {
    //     Ok(_) => println!("parsing successful"),
    //     Err(e) => println!("parsing failed: {:?}", e)
    // }
}


fn criterion_benchmark(c: &mut Criterion) {
    let path = bolt::util::find_asset("models/Genesis9.dsf").unwrap();
    let file = bolt::scene::daz::load::read_from_dsf(&path).unwrap();
    

    c.bench_function("parse_par", |b| b.iter(|| {
        test_parse(&file)
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);