// //use bolt::prelude::*;
// use bolt::sim;
// use bolt::resource::BindlessManager;

// #[derive(Debug, Clone, Default)]
// struct MPMnode {
//     pub position: Vec<f32>,
//     pub velocity: Vec<f32>,
//     pub mass: f32,
//     pub volume: f32,
//     pub density: f32,
//     pub pressure: f32,
//     pub force: Vec<f32>,
//     pub stress: Vec<f32>,
//     pub strain: Vec<f32>,
// }

// pub fn setup(app: &mut bolt::App) -> AppData {

//     let resource_manager = app.resource_manager();

//     let MPM_grid = sim::Grid::<MPMnode>::new(3, 0.1, vec![0.0, 0.0, 0.0], vec![1.0, 1.0, 1.0]);
//     let MPM = sim::PipelineBuildPlan::<MPMnode>::new(MPM_grid);

//     let sim_manager = sim::SimulationManager::new();
//     let pathtracing = ray::PathtracingPass::new();
//     let debug = debug::Renderer::new();

//     sim_manager.register(MPM_grid, "/assets/shaders/sim/MPM.comp");
//     pathtracing.register(sim_manager);
//     debug.register(sim_manager);

//     p_sim = sim_manager.instanciate(app.context());
//     p_pathtracing = pathtracing.instanciate(context);
//     p_debug = debug.instanciate(context);

//     let composed = p_sim => p_pathtracing => p_debug;

//     bolt::Buffer::from_data(&[0.0, 0.0, 0.0, 1.0], bolt::BufferUsage::UniformBuffer, context);
// }

fn main() {

    // App.setup(setup);
    // println!("{:?}", sim_manager)
}