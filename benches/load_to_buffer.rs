// use std::sync::Arc;

// use bolt::Window;
// use bolt::{resource::{ResourceManager, BindlessManager}, AppSettings, Context};
// use winit::event_loop::EventLoop;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use bolt::resource::image::*;


// fn prepare_manager(settings: AppSettings, event_loop: &EventLoop<()>) -> BindlessManager {
//     let mut window = Window::new(
//         settings.resolution[0],
//         settings.resolution[1],
//         settings.name.clone(),
//         &event_loop,
//     );
//     let manager = bolt::resource::BindlessManager::new(&mut window, &settings.clone().manager, Some(settings.render));
//     let renderer = bolt::AppRenderer::new(&mut window, &manager);
//     manager
    
// }


fn criterion_benchmark(c: &mut Criterion) {
    // let mut group = c.benchmark_group("model_load");
    // group.sample_size(10);
    // let mut resource_manager = prepare_manager(AppSettings::default(), &EventLoop::new());
    // let mut textures = load_images::<u8>(None, &vec!["assets/textures/smallsky.png".to_string()]);
    
    // group.bench_function("load_seq", |b| b.iter(|| {
    //     resource_manager.register_texture(black_box(&mut textures[0]));
    // }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);