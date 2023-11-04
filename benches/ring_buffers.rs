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
use circular_queue::CircularQueue;

// primitive data types: comparativly very slow.
// heap allocated data types: basically the same as array buffer
fn write_to_ring_buffer<'a>(buffer: &'a mut CircularQueue<&'a Vec<i32>>, data: &'a Vec<Vec<i32>>) {
    for i in data {
        buffer.push(i);
    }
}

// primitive data types: very fast. Only nanoseconds
fn write_to_array_ring_buffer<'a>(buffer: &'a mut [&'a Vec<i32>], data: &'a Vec<Vec<i32>>) {
    for (i, d) in data.iter().enumerate() {
        buffer[i as usize % buffer.len()] = d;
    }
}


fn criterion_benchmark(c: &mut Criterion) {
    // let path = bolt::util::find_asset("models/Genesis9.dsf").unwrap();
    // let file = bolt::scene::daz::load::read_from_dsf(&path).unwrap();
    

    c.bench_function("write_circular_queue", |b| b.iter(|| {
        write_to_ring_buffer(&mut CircularQueue::with_capacity(2), &black_box((0..100).into_iter().map(|_| (0..100).collect::<Vec<i32>>()).collect::<Vec<Vec<i32>>>()))
    }));
    c.bench_function("write_ring_array", |b| b.iter(|| {
        write_to_array_ring_buffer(&mut [&Vec::new(), &Vec::new()], &black_box((0..100).into_iter().map(|_| (0..100).collect::<Vec<i32>>()).collect::<Vec<Vec<i32>>>()))
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);