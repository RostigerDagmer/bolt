use std::env;
use ash::vk::PipelineLayoutCreateInfo;
use bolt::RadixSort;

use bolt::prelude::*;
use bolt::scene;
use rayon::prelude::*;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct SceneData {
    mvp: Mat4,
    normal: Mat4,
    gravity: Vec4,
    bounds_min: Vec4,
    bounds_max: Vec4,
}

pub struct Scene {
    size: usize,
    positions: bolt::Buffer,
    velocities: bolt::Buffer,
    densities: bolt::Buffer,
    pressures: bolt::Buffer,
    cells: bolt::Buffer,
    cell_lookup: bolt::Buffer,
}

pub struct PerFrameData {
    pub ubo: bolt::Buffer,
    pub desc_set: bolt::DescriptorSet,
}

pub struct AppData {
    pub scene: Scene,
    pub graphics_pipeline: bolt::Pipeline,
    pub compute_pipeline: bolt::ComputePipeline,
    pub hash_layout: bolt::DescriptorSetLayout,
    pub hash_pipeline: bolt::ComputePipeline,
    pub hash_pipeline_layout: bolt::PipelineLayout,
    pub sort: bolt::RadixSort,
    pub desc_set_layout: bolt::DescriptorSetLayout,
    pub pass_layout: bolt::DescriptorSetLayout,
    pub pipeline_layout: bolt::PipelineLayout,
    pub per_frame: Vec<PerFrameData>,
    pub manip: scene::CameraManip,
}


// struct SPHComputePipeline {
//     pub hash: (bolt::DescriptorSetLayout, bolt::ComputePipeline),
//     pub sort: bolt::RadixSort,
//     pub integrate: (bolt::DescriptorSetLayout, bolt::ComputePipeline),
// }

// impl bolt::ComputePass for SPHComputePipeline {
//     fn pass(self, cmd: vk::CommandBuffer, context: Arc<bolt::Context>) {
//         todo!()
//     }
// }

pub struct AppDataBuilder<'a> {
    pub app: &'a bolt::App,

    // all fields of AppData but Optional
    pub scene: Option<Scene>,
    pub graphics_pipeline: Option<bolt::Pipeline>,
    pub hash_layout: Option<bolt::DescriptorSetLayout>,
    pub hash_pipeline: Option<bolt::ComputePipeline>,
    hash_pipeline_layout: Option<bolt::PipelineLayout>,
    pub sort: Option<bolt::RadixSort>,
    pub compute_pipeline: Option<bolt::ComputePipeline>,
    pub desc_set_layout: Option<bolt::DescriptorSetLayout>,
    pub pass_layout: Option<bolt::DescriptorSetLayout>,
    pub pipeline_layout: Option<bolt::PipelineLayout>,
    pub per_frame: Option<Vec<PerFrameData>>,
    pub manip: Option<scene::CameraManip>,
}

impl<'b: 'a, 'a> AppDataBuilder<'a> {
    pub fn new(app: &'b mut bolt::App) -> AppDataBuilder {
        AppDataBuilder {
            app: app,
            scene: None,
            graphics_pipeline: None,
            hash_layout: None,
            hash_pipeline: None,
            hash_pipeline_layout: None,
            sort: None,
            compute_pipeline: None,
            desc_set_layout: None,
            pass_layout: None,
            pipeline_layout: None,
            per_frame: None,
            manip: None,
        }
    }
    pub fn scene(mut self, scene: Scene) -> Self {
        self.scene = Some(scene);
        self
    }
    pub fn descriptor_set_layout(mut self, info: bolt::DescriptorSetLayoutInfo) -> Self {
        self.desc_set_layout = Some(bolt::DescriptorSetLayout::new(
            self.app.renderer.context.clone(),
            info
        ));
        self
    }

    fn pipeline_layout<F: FnOnce(bolt::PipelineLayoutInfo) -> bolt::PipelineLayoutInfo>(mut self, extensions: Option<F>) -> Self {

        let layout = bolt::PipelineLayoutInfo::default().desc_set_layouts(&[
            self.desc_set_layout.as_ref().expect("specify a descriptor set layout before building the pipeline layout").handle(),
            self.pass_layout.as_ref().expect("specify a descriptor set layout for the render pass before building the pipeline layout").handle(),
            ]);
        let layout =  match extensions {
            Some(f) => f(layout),
            None => layout,
        };
        self.pipeline_layout = Some(bolt::PipelineLayout::new(
            self.app.renderer.context.clone(),
            layout,
        ));
        self
    }

    fn graphics_pipeline<F: FnOnce(bolt::PipelineInfo) -> bolt::PipelineInfo>(mut self, pipeline_opts: F) -> Self {
        self.graphics_pipeline = Some(bolt::Pipeline::new(
            self.app.renderer.context.clone(),
            pipeline_opts(
                bolt::PipelineInfo::default()
                .layout(self.pipeline_layout.as_ref().expect("specify a pipeline layout before building the pipeline").handle())
                .render_pass_info(self.app.renderer.swapchain.get_transient_render_pass_info())
            )
        ));
        self
    }

    fn compute_pipeline<F: FnOnce(bolt::ComputePipelineInfo) -> bolt::ComputePipelineInfo>(mut self, pipeline_opts: F) -> Self {
        self.compute_pipeline = Some(bolt::ComputePipeline::new(
            self.app.renderer.context.clone(),
            pipeline_opts(
                bolt::ComputePipelineInfo::default()
                .layout(self.pipeline_layout.as_ref().expect("specify a pipeline layout before building the pipeline").handle())
            )
        ));
        self
    }

    fn hash_pipeline(mut self) -> Self {

        let hash_layout = bolt::DescriptorSetLayout::new(
            self.app.renderer.context.clone(),
            bolt::DescriptorSetLayoutInfo::default()
            .binding(0, // spatial data to hash
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::COMPUTE
            )
            .binding(1, // hash output
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::COMPUTE
            )
        );

        let hash_pipeline_layout = bolt::PipelineLayout::new(
            self.app.renderer.context.clone(),
            bolt::PipelineLayoutInfo::default()
            .desc_set_layouts(&[hash_layout.handle()])
            .push_constant_range(
                vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .size(size_of::<GridHashConstants>() as u32)
                .build()
            )
        );

        let pipeline_info = bolt::ComputePipelineInfo::default()
            .layout(hash_pipeline_layout.handle())
            .comp(bolt::util::find_asset("glsl/hash.comp").unwrap());


        self.hash_pipeline = Some(bolt::ComputePipeline::new(
                self.app.renderer.context.clone(),
                pipeline_info,
            )
        );
        self.hash_layout = Some(hash_layout);
        self.hash_pipeline_layout = Some(hash_pipeline_layout);
        self
    }

    fn sort_pipeline<F: FnOnce(&Scene) -> bolt::RadixSortInfo>(mut self, attrib_selector: F) -> Self {
        self.sort = Some(bolt::RadixSort::new(
            self.app.renderer.context.clone(),
            attrib_selector(self.scene.as_ref().expect("specify a scene before trying to sort buffers of scene")),
        ));
        self
    }

    fn pass_layout(mut self, info: bolt::DescriptorSetLayoutInfo) -> Self {
        self.pass_layout = Some(bolt::DescriptorSetLayout::new(
            self.app.renderer.context.clone(),
            info
        ));
        self
    }

    fn per_frame<F: Fn(&mut AppDataBuilder) -> PerFrameData>(mut self, data: F) -> Self {
        self.per_frame = Some((0..self.app.renderer.get_frames_count()).into_iter().map(|_| data(& mut self)).collect());   
        self
    }

    fn manip(mut self, manip: scene::CameraManip) -> Self {
        self.manip = Some(manip);
        self
    }

    fn build(self) -> AppData {
        AppData {
            scene: self.scene.expect("specify a scene before building the app data"),
            graphics_pipeline: self.graphics_pipeline.expect("specify a graphics pipeline before building the app data"),
            compute_pipeline: self.compute_pipeline.expect("specify a compute pipeline before building the app data"),
            desc_set_layout: self.desc_set_layout.expect("specify a descriptor set layout before building the app data"),
            pass_layout: self.pass_layout.expect("specify a descriptor set layout for the render pass before building the app data"),
            pipeline_layout: self.pipeline_layout.expect("specify a pipeline layout before building the app data"),
            per_frame: self.per_frame.expect("specify per frame data before building the app data"),
            manip: self.manip.expect("specify a camera manipulator before building the app data"),
            hash_layout: self.hash_layout.expect("specify a hash descriptor set layout before building the app data"),
            hash_pipeline: self.hash_pipeline.expect("specify a hash pipeline before building the app data"),
            hash_pipeline_layout: self.hash_pipeline_layout.expect("specify a hash pipeline layout before building the app data"),
            sort: self.sort.expect("specify sorting before building the app data"),
        }
    }


}


pub fn construct_scene(app: &mut bolt::App) -> Scene {
    // generate a cube of particles positions
    let size: usize = 1000;
    let dim = (size as f32).powf(1.0 / 3.0).round() as usize;
    let positions = (0..size)
        .into_par_iter()
        .map(|i| {
            let x = i % dim;
            let y = (i / dim) % dim;
            let z = i / (dim * dim);
            let x = x as f32 / (dim - 1) as f32;
            let y = y as f32 / (dim - 1) as f32;
            let z = z as f32 / (dim - 1) as f32;
            vec4(x, y, z, 1.0)
        })
        .collect::<Vec<_>>();
    println!("dim: {}", dim);
    let velocities = vec![Vec4::ZERO; positions.len()];
    let densities = vec![0.0; positions.len()];
    let pressures = vec![0.0; positions.len()];
    let cells = vec![0 as u32; positions.len()];
    let cell_lookup = (0..size as u32).collect::<Vec<u32>>();
    let context = app.renderer.context.clone();
    let positions = bolt::Buffer::from_data(
        context.clone(),
        bolt::BufferInfo::default()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER).memory_type_bits(vk::MemoryAllocateFlags::DEVICE_ADDRESS.as_raw())
            .cpu_to_gpu(),
        &positions,
    );  
    let velocities = bolt::Buffer::from_data(
        context.clone(),
        bolt::BufferInfo::default()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .cpu_to_gpu(),
        &velocities,
    );
    let densities = bolt::Buffer::from_data(
        context.clone(),
        bolt::BufferInfo::default()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .cpu_to_gpu(),
        &densities,
    );
    let pressures = bolt::Buffer::from_data(
        context.clone(),
        bolt::BufferInfo::default()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .cpu_to_gpu(),
        &pressures,
    );
    let cells = bolt::Buffer::from_data(
        context.clone(),
        bolt::BufferInfo::default()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .cpu_to_gpu(),
        &cells,
    );

    let cell_lookup = bolt::Buffer::from_data(
        context.clone(),
        bolt::BufferInfo::default()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .cpu_to_gpu(),
        &cell_lookup,
    );

    Scene {
        size,
        positions,
        velocities,
        densities,
        pressures,
        cells,
        cell_lookup
    }

}

pub fn cmd_draw(context: Arc<bolt::Context>, cmd: vk::CommandBuffer, scene: &Scene) {
    unsafe {
        context.device().cmd_bind_vertex_buffers(
            cmd,
            0,
            &[scene.positions.handle(), scene.velocities.handle()],
            &[0, 0],
        );
        context.device().cmd_draw(cmd, scene.size as u32, 1, 0, 0);
    }
}

struct Particle {
    position: Vec4,
}

impl Vertex for Particle {
    fn stride() -> u32 {
        std::mem::size_of::<Particle>() as u32
    }

    fn format_offset() -> Vec<(vk::Format, u32)> {
        vec![
            (
                vk::Format::R32G32B32A32_SFLOAT,
                bolt::offset_of!(Particle, position) as u32,
            ),
        ]
    }
}


pub fn setup(app: &mut bolt::App) -> AppData {
    let context = app.renderer.context.clone();
    let window_size = app.window.get_size();
    let mut camera = scene::Camera::new(window_size);
    camera.look_at(Vec3::splat(3.0), vec3(0.0, 0.5, 0.0), -Vec3::Y);
    let scene = construct_scene(app);
    let scene_data = SceneData {
        mvp: camera.perspective_matrix() * camera.view_matrix() * Mat4::IDENTITY,
        normal: (camera.view_matrix() * Mat4::IDENTITY)
            .inverse()
            .transpose(),
        gravity: vec4(0.0, -0.98, 0.0, 0.0),
        bounds_min: vec4(-1.5, -1.5, -1.5, 1.0),
        bounds_max: vec4(1.5, 1.5, 1.5, 1.0),
    };

    let per_frame = |b: &mut AppDataBuilder| -> PerFrameData {
        let ubo = bolt::Buffer::from_data(
            context.clone(),
            bolt::BufferInfo::default()
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .cpu_to_gpu(),
            &[scene_data],
        );
        let desc_set = b.desc_set_layout.as_mut().expect("descriptor set layout not specified yet.")
            .get_or_create(bolt::DescriptorSetInfo::default().buffer(0, ubo.get_descriptor_info()));
        PerFrameData { ubo, desc_set }
    };

    AppDataBuilder::new(app)
    .scene(scene)
    .descriptor_set_layout(
        bolt::DescriptorSetLayoutInfo::default().binding(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::ShaderStageFlags::ALL,
        )
    )
    .pass_layout(
        bolt::DescriptorSetLayoutInfo::default()
        .binding(0,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
        )
        .binding(1,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
        )
        .binding(2,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
        )
        .binding(3,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
        )
        .binding(4,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
        )
        .binding(5,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE
        )
    )    
    .pipeline_layout(Some(|info: bolt::PipelineLayoutInfo| {
        info.push_constant_range(vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS ^ vk::ShaderStageFlags::COMPUTE)
        .size(size_of::<Constants>() as u32)
        .build())
    }))
    .graphics_pipeline(|pipeline_info| {
        pipeline_info
            .vert(bolt::util::find_asset("glsl/SPH/particle.vert").unwrap())
            .frag(bolt::util::find_asset("glsl/SPH/particle.frag").unwrap())
            .front_face(vk::FrontFace::CLOCKWISE)
            .primitive_topology(vk::PrimitiveTopology::POINT_LIST)
            .cull_mode(vk::CullModeFlags::NONE)
            .vertex_type::<Particle>()
            .blend_mode(bolt::PipelineBlendMode::Alpha)
    })
    .compute_pipeline(|pipeline_info| {
        pipeline_info
            .comp(bolt::util::find_asset("glsl/SPH/pressure.comp").unwrap())
            .comp(bolt::util::find_asset("glsl/SPH/sph.comp").unwrap())
    })
    .sort_pipeline(|scene: &Scene| {
        bolt::RadixSortInfo::new(&scene.cells, &scene.cell_lookup)
    })
    .hash_pipeline()
    .per_frame(per_frame)
    .manip(scene::CameraManip {
        camera,
        input: scene::CameraInput::default(),
        mode: scene::CameraMode::Fly,
        ..Default::default()
    })
    .build()
}

pub fn window_event(app: &mut bolt::App, data: &mut AppData, event: &winit::event::WindowEvent) {
    data.manip.update(&event);
    match event {
        winit::event::WindowEvent::KeyboardInput { input, .. } => {
            if let winit::event::KeyboardInput {
                virtual_keycode: Some(key),
                state: winit::event::ElementState::Pressed,
                ..
            } = input
            {
                match key {
                    winit::event::VirtualKeyCode::Space => {
                        data.manip.mode = match data.manip.mode {
                            scene::CameraMode::Fly => scene::CameraMode::Spherical,
                            scene::CameraMode::Spherical => scene::CameraMode::Fly,
                            _ => scene::CameraMode::Fly,
                        }
                    }
                    winit::event::VirtualKeyCode::R => {
                        data.scene = construct_scene(app);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Constants {
    mass: f32,
    radius: f32,
    stiffness: f32,
    viscosity: f32,
    rest_density: f32,
    delta_time: f32,
    num_vertices: u32,
    smoothing_radius: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridHashConstants {
    num_vertices: u32,
    radius: f32,
}


pub fn render(app: &mut bolt::App, data: &mut AppData) -> Result<(), bolt::AppRenderError> {
    let (image_aquired_semaphore, cmd) = app.renderer.begin_frame_no_renderpass()?;
    let ref camera = data.manip.camera;
    //TODO: move mesh transform in push constant?
    let scene_data = SceneData {
        mvp: camera.perspective_matrix() * camera.view_matrix() * Mat4::IDENTITY,
        normal: (camera.view_matrix() * Mat4::IDENTITY)
            .inverse()
            .transpose(),
        gravity: vec4(0.0, -0.98, 0.0, 1.0),
        bounds_min: vec4(-1.5, -1.5, -1.5, 1.0),
        bounds_max: vec4(1.5, 1.5, 1.5, 1.0),
    };
    data.per_frame[app.renderer.active_frame_index]
        .ubo
        .update(&[scene_data]);
    let pipeline_layout = data.pipeline_layout.handle();

    // create the render pass descriptor set
    let desc_set = data.pass_layout.get_or_create(bolt::DescriptorSetInfo::default()
        .buffer(0, data.scene.positions.get_descriptor_info())
        .buffer(1, data.scene.velocities.get_descriptor_info())
        .buffer(2, data.scene.densities.get_descriptor_info())
        .buffer(3, data.scene.pressures.get_descriptor_info())
        .buffer(4, data.scene.cells.get_descriptor_info())
        .buffer(5, data.scene.cell_lookup.get_descriptor_info())
    );

    let descriptor_sets = [data.per_frame[app.renderer.active_frame_index].desc_set.handle(), desc_set.handle()];
    let device = app.renderer.context.device();

    let grid_params = GridHashConstants {
        num_vertices: data.scene.size as u32,
        radius: 0.2,
    };

    // hash
    unsafe {
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, data.hash_pipeline.handles().unwrap()[0]);
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            data.hash_pipeline_layout.handle(),
            0,
            &[data.hash_layout.get_or_create(bolt::DescriptorSetInfo::default()
                .buffer(0, data.scene.positions.get_descriptor_info())
                .buffer(1, data.scene.cells.get_descriptor_info())
            ).handle()],
            &[],
        );
        device.cmd_push_constants(
            cmd,
            data.pipeline_layout.handle(),
            vk::ShaderStageFlags::COMPUTE,
            0,
            &bytemuck::bytes_of(&grid_params),
        );
        device.cmd_dispatch(cmd, data.scene.size as u32, 1, 1);
    }

    // memory barrier
    unsafe {
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::COMPUTE_SHADER, // Source stage
            vk::PipelineStageFlags::COMPUTE_SHADER, // Destination stage
            vk::DependencyFlags::empty(),
            &[], // Memory barriers
            &[vk::BufferMemoryBarrier {
                src_access_mask: vk::AccessFlags::SHADER_WRITE, // compute pass writes to the buffer
                dst_access_mask: vk::AccessFlags::SHADER_READ,  // compute pass reads from the buffer
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                buffer: data.scene.cells.handle(), // The buffer being accessed
                offset: 0,                  // Start offset of the buffer
                size: vk::WHOLE_SIZE,       // Size of the buffer
                ..Default::default()
            }],
            &[], // Image memory barriers
        );
    }

    // sort
    data.sort.pass(cmd, device);


    // compute

    let dim = (data.scene.size as f32).powf(1.0 / 3.0).ceil();
    let radius = 1.0 / dim;

    let constants = Constants {
        mass: 1.0,
        radius: radius,
        stiffness: 0.5,
        viscosity: 0.5,
        rest_density: 1000.0,
        delta_time: 0.01,
        num_vertices: data.scene.size as u32,
        smoothing_radius: grid_params.radius,
    };


    unsafe {
        for p in data.compute_pipeline.handles().unwrap() {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, *p);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                pipeline_layout,
                0,
                descriptor_sets.as_slice(),
                &[],
            );
            device.cmd_push_constants(
                cmd,
                data.pipeline_layout.handle(),
                vk::ShaderStageFlags::COMPUTE,
                0,
                &bytemuck::bytes_of(&constants),
            );
            // let dim = (data.scene.size as f32).powf(1.0 / 2.0).ceil() as u32;
            device.cmd_dispatch(cmd, data.scene.size as u32, 1, 1);
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .build()],
                &[],
                &[],
            );
        }
    }

    // // memory barrier
    // unsafe {
    //     device.cmd_pipeline_barrier(
    //         cmd,
    //         vk::PipelineStageFlags::COMPUTE_SHADER, // Source stage
    //         vk::PipelineStageFlags::VERTEX_INPUT, // Destination stage
    //         vk::DependencyFlags::empty(),
    //         &[],
    //         &[vk::BufferMemoryBarrier {
    //             src_access_mask: vk::AccessFlags::SHADER_WRITE, // compute pass writes to the buffer
    //             dst_access_mask: vk::AccessFlags::SHADER_READ,  // graphics pass reads from the buffer
    //             src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
    //             dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
    //             buffer: data.scene.positions.handle(), // The buffer being accessed
    //             offset: 0,                  // Start offset of the buffer
    //             size: vk::WHOLE_SIZE,       // Size of the buffer
    //             ..Default::default()
    //         }],
    //         &[],
    //     );
    // }

    app.renderer.begin_renderpass(cmd, app.renderer.swapchain.get_extent());

    unsafe {
        device.cmd_set_scissor(cmd, 0, &[app.window.get_rect()]);
        device.cmd_set_viewport(cmd, 0, &[app.window.get_viewport()]);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, data.graphics_pipeline.handle());
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            pipeline_layout,
            0,
            descriptor_sets.as_slice(),
            &[],
        );
    }
    cmd_draw(app.renderer.context.clone(), cmd, &data.scene);
    // data.scene.vulkan_meshes.par_iter().for_each(|mesh| mesh.cmd_draw(cmd));
    app.renderer.end_frame_default(image_aquired_semaphore, cmd)
}

pub fn prepare() -> bolt::AppSettings {
    bolt::AppSettings {
        name: "SPH".to_string(),
        resolution: [1500, 800],
        render: bolt::RendererSettings {
            extensions: vec![vk::KhrGetPhysicalDeviceProperties2Fn::name()],
            device_extensions: vec![vk::KhrDeferredHostOperationsFn::name(), vk::KhrBufferDeviceAddressFn::name()],
            samples: 8,
            clear_color: Vec4::splat(0.15),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn main() {
    env_logger::init();
    log::info!("Starting SPH example");
    env::set_var("RUST_BACKTRACE", "full");
    bolt::App::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
