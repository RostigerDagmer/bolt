use std::path::PathBuf;

use bolt::AppSettings;
use bolt::SharedContext;
use bolt::resource::skin::RigParser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ash::vk;
use bolt::scene::*;
use rayon::prelude::*;
use bolt::scene::daz::format::*;
use bolt::scene::daz::DSF;
use glam::Mat4;

// single threaded one rig 540us
// multi threaded one rig 250us
fn parse(file: &DSF) -> RigV1<Mat4, BoneV1<Mat4>> {
    type RigSetup = DazRigParserV1<RigV1<Mat4, BoneV1<Mat4>>, Mat4, BoneV1<Mat4>>;
    let res = RigSetup::parse(black_box(&file));
    res.first().unwrap().as_ref().unwrap().clone()
    // match res {
    //     Ok(_) => println!("parsing successful"),
    //     Err(e) => println!("parsing failed: {:?}", e)
    // }
}
// 5us
fn local_transforms_seq_naive(rig: &RigV1<Mat4, BoneV1<Mat4>>) {
    // write local transforms of bones into Vec<Mat4> of which the ordering is consistent with rig.bones
    let local_transforms:Vec<Mat4> = (0..rig.bones.len()).into_iter().map(|i| {
        rig.local_to_global(i)
    }).collect();
}
// 20us
fn local_transforms_par_naive(rig: &RigV1<Mat4, BoneV1<Mat4>>) {
    // write local transforms of bones into Vec<Mat4> of which the ordering is consistent with rig.bones
    let local_transforms:Vec<Mat4> = (0..rig.bones.len()).into_par_iter().map(|i| {
        rig.local_to_global(i)
    }).collect();
}

// try computing local transforms of multiple rigs in parallel
fn local_transforms_par_multiple_rigs(rigs: &Vec<RigV1<Mat4, BoneV1<Mat4>>>) {
    let local_transforms:Vec<Vec<Mat4>> = rigs.par_iter().map(|rig| {
        (0..rig.bones.len()).into_iter().map(|i| {
            rig.local_to_global(i)
        }).collect()
    }).collect();
}

fn local_transforms_seq_multiple_rigs(rigs: &Vec<RigV1<Mat4, BoneV1<Mat4>>>) {
    let local_transforms:Vec<Vec<Mat4>> = rigs.iter().map(|rig| {
        (0..rig.bones.len()).into_iter().map(|i| {
            rig.local_to_global(i)
        }).collect()
    }).collect();
}


// Learnings:
// - parallelizing the computation of local transforms of a single rig is not worth it
// - parallelizing the computation of local transforms of multiple rigs is worth it starting at about CORE_COUNT - 1 rigs
// We'll see how this goes once there are subrigs added into a rig -> more bones

fn criterion_benchmark(c: &mut Criterion) {
    let path = bolt::util::find_asset("models/Genesis9.dsf").unwrap();
    let file = bolt::scene::daz::load::read_from_dsf(&path).unwrap();
    let rig = parse(&file);

    let mut rigs = Vec::new();
    for _ in 0..32 {
        rigs.push(rig.clone());
    }

    c.bench_function("local_transforms_seq_multiple_rigs", |b| b.iter(|| {
        local_transforms_seq_multiple_rigs(&rigs)
    }));

    c.bench_function("local_transforms_par_multiple_rigs", |b| b.iter(|| {
        local_transforms_par_multiple_rigs(&rigs)
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);