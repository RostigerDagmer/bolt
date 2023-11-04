use std::env;
use std::ops::Deref;
use bolt::prelude::*;
use bolt::ray;
use bolt::ray::GeometryInstance;
use bolt::scene;
use bolt::scene::HalfEdgeID;
use bolt::scene::ID;
use bolt::scene::VertexID;
use winit::event::WindowEvent;
use rayon::prelude::*;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct SceneUniforms {
    model: Mat4,
    view: Mat4,
    view_inverse: Mat4,
    projection: Mat4,
    projection_inverse: Mat4,
    model_view_projection: Mat4,
    frame: UVec3,
    padding: u32,
}

impl SceneUniforms {
    pub fn from(camera: &scene::Camera, frame: UVec3) -> SceneUniforms {
        let vp = camera.perspective_matrix() * camera.view_matrix();
        SceneUniforms {
            model: Mat4::IDENTITY,
            view: camera.view_matrix(),
            view_inverse: camera.view_matrix().inverse(),
            projection: camera.perspective_matrix(),
            projection_inverse: camera.perspective_matrix().inverse(),
            model_view_projection: vp,
            frame,
            ..Default::default()
        }
    }
}

pub struct PerFrameData {
    pub ubo: bolt::Buffer,
    pub desc_set: bolt::DescriptorSet,
}
pub struct AppData {
    pub scene: scene::Scene,
    pub graphics_pipeline_layout: bolt::PipelineLayout,
    pub layout_scene: bolt::DescriptorSetLayout,
    pub graphics_layout_pass: bolt::DescriptorSetLayout,
    pub per_frame: Vec<PerFrameData>,
    pub manip: scene::CameraManip,

    // Raytracing tools & data
    pub scene_description: ray::SceneDescription,
    pub graphics_pipeline: ray::Pipeline,
    pub sbt: ray::ShaderBindingTable,
    pub accumulation_start_frame: u32,
    pub accum_target: bolt::Image2d,
    pub render_target: bolt::Image2d,
    pub skydome: Option<bolt::Texture2d>,

    // collision pipeline that reuses the acceleration structures
    // pub collision_pipeline: ray::Pipeline,
    // pub collision_sbt: ray::ShaderBindingTable,

    // compute pipelines
    // pub layout_compute: bolt::DescriptorSetLayout,
    // pub compute_pipeline: bolt::ComputePipeline,
    // pub pipeline_layout_compute: bolt::PipelineLayout,

    //pub collision_target_buffer: bolt::Buffer,

    pub enable_sky: bool, //Temporary shader hack for sky/sun light
}

fn create_image_target(
    context: &Arc<bolt::Context>,
    window: &bolt::Window,
    format: vk::Format,
) -> bolt::Image2d {
    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(window.get_extent_3d())
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    bolt::Image2d::new(
        context.shared().clone(),
        &image_info,
        vk::ImageAspectFlags::COLOR,
        1,
        "TargetRT"
    )
}

fn build_pipeline_sbt(
    context: &Arc<bolt::Context>,
    pipeline_layout: &bolt::PipelineLayout,
    enable_sky: bool,
) -> (ray::Pipeline, ray::ShaderBindingTable) {
    let pipeline = ray::Pipeline::new(
        context.clone(),
        ray::PipelineInfo::default()
            .layout(pipeline_layout.handle())
            .shader(
                bolt::util::find_asset("glsl/spectral.rgen").unwrap(),
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .shader(
                bolt::util::find_asset("glsl/spectral.rmiss").unwrap(),
                vk::ShaderStageFlags::MISS_KHR,
            )
            .shader(
                bolt::util::find_asset("glsl/spectral.rchit").unwrap(),
                vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            )
            .specialization(&[enable_sky as u32], 0)
            .name("AO_mat".to_string()),
    );
    let sbt = ray::ShaderBindingTable::new(
        context.clone(),
        pipeline.handle(),
        ray::ShaderBindingTableInfo::default()
            .raygen(0)
            .miss(1)
            .hitgroup(2),
    );

    (pipeline, sbt)
}


fn build_collision_pipeline_sbt(
    context: &Arc<bolt::Context>,
    pipeline_layout: &bolt::PipelineLayout,
) -> (ray::Pipeline, ray::ShaderBindingTable) {
    let pipeline = ray::Pipeline::new(
        context.clone(),
        ray::PipelineInfo::default()
        .layout(pipeline_layout.handle())
        .shader(
            bolt::util::find_asset("glsl/collision.rgen").unwrap(),
            vk::ShaderStageFlags::RAYGEN_KHR,
        )
        .shader(
            bolt::util::find_asset("glsl/collision.rmiss").unwrap(),
            vk::ShaderStageFlags::MISS_KHR,
        )
        .shader(
            bolt::util::find_asset("glsl/collision.rchit").unwrap(),
            vk::ShaderStageFlags::CLOSEST_HIT_KHR,
        ).name("Collision".to_string()),
    );
    let sbt = ray::ShaderBindingTable::new(
        context.clone(),
        pipeline.handle(),
        ray::ShaderBindingTableInfo::default()
            .raygen(0)
            .miss(1)
            .hitgroup(2),
    );
    (pipeline, sbt)
}


pub fn setup(app: &mut bolt::App) -> AppData {
    let context = &app.renderer.context;
    let index = std::env::args().position(|arg| arg == "--model").unwrap();
    let mut scene = scene::load_scene(
        context.clone(),
        &bolt::util::find_asset(&std::env::args().nth(index + 1).expect("no file given"))
            .unwrap(),
    );
    let enable_sky = std::env::args().any(|arg| arg == "--sky");
    let mut skydome = None;
    if enable_sky {
        let skymap_idx = std::env::args().position(|arg| arg == "--sky").unwrap();
        let skymap_path = &bolt::util::find_asset(&std::env::args().nth(skymap_idx + 1).expect("no skymap given"))
            .unwrap();
        let skymap = bolt::Texture2d::new(context.clone(), skymap_path.clone().to_path_buf());
        skydome = Some(skymap);
    }
    let camera = match scene.camera {
        Some(scene_camera) => {
            let mut cam = scene_camera;
            cam.set_window_size(app.window.get_size());
            cam
        }
        None => scene::Camera::new(app.window.get_size())
    };

    let scene_description = ray::SceneDescription::from_scene(context.clone(), &mut scene);

    let mut per_frame = Vec::<PerFrameData>::new();

    let mut layout_scene = bolt::DescriptorSetLayout::new(
        context.clone(),
        bolt::DescriptorSetLayoutInfo::default().binding(
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::ShaderStageFlags::ALL,
        ),
    );
    let instance_count = scene_description.get_instances_buffer().get_element_count();
    let texture_count = scene_description.get_texture_descriptors().len();
    let layout_pass = bolt::DescriptorSetLayout::new(
        context.clone(),
        bolt::DescriptorSetLayoutInfo::default()
            .binding(
                0,
                vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .binding(
                1,
                vk::DescriptorType::STORAGE_IMAGE,
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .binding(
                2,
                vk::DescriptorType::STORAGE_IMAGE,
                vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .binding(
                3,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::CLOSEST_HIT_KHR ^ vk::ShaderStageFlags::RAYGEN_KHR,
            )
            .bindings(
                4,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::CLOSEST_HIT_KHR ^ vk::ShaderStageFlags::RAYGEN_KHR,
                instance_count,
            )
            .bindings(
                5,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::CLOSEST_HIT_KHR ^ vk::ShaderStageFlags::RAYGEN_KHR,
                instance_count,
            )
            .bindings(
                6,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::CLOSEST_HIT_KHR ^ vk::ShaderStageFlags::RAYGEN_KHR,
                instance_count,
            )
            .bindings(
                7,
                vk::DescriptorType::STORAGE_BUFFER,
                vk::ShaderStageFlags::CLOSEST_HIT_KHR ^ vk::ShaderStageFlags::RAYGEN_KHR,
                instance_count,
            )
            .bindings(8,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                vk::ShaderStageFlags::CLOSEST_HIT_KHR ^ vk::ShaderStageFlags::RAYGEN_KHR, 
                texture_count as u32)
            .binding(
                9,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                vk::ShaderStageFlags::MISS_KHR,
            )
    );


    // compute pipeline /////////////////////////////////////


    // let layout_compute = bolt::DescriptorSetLayout::new(
    //     context.clone(),
    //     bolt::DescriptorSetLayoutInfo::default()
    //         .binding(
    //             0,
    //             vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
    //             vk::ShaderStageFlags::COMPUTE,
    //         )
    //         .binding(
    //             1,
    //             vk::DescriptorType::STORAGE_BUFFER,
    //             vk::ShaderStageFlags::COMPUTE,
    //         )
    //         .binding(
    //             2,
    //             vk::DescriptorType::STORAGE_BUFFER,
    //             vk::ShaderStageFlags::COMPUTE,
    //         )
    //         .binding(
    //             3,
    //             vk::DescriptorType::STORAGE_BUFFER,
    //             vk::ShaderStageFlags::COMPUTE,
    //         ),
    // );

    // let pipeline_layout_compute = bolt::PipelineLayout::new(
    //     context.clone(),
    //     bolt::PipelineLayoutInfo::default().desc_set_layouts(&[layout_compute.handle()]),
    // );
    
    // let compute_pipeline = bolt::ComputePipeline::new(
    //     context.clone(),
    //     bolt::ComputePipelineInfo::default()
    //         .layout(pipeline_layout_compute.handle())
    //         .comp(bolt::find_assets("glsl/mass_spring_solve.comp".into()))
    // );

    ////// Uniforms //////////////////////////////////////////////

    for _ in 0..app.renderer.get_frames_count() {
        let uniforms = SceneUniforms::from(
            &camera,
            uvec3(app.window.get_width(), app.window.get_height(), 0),
        );
        let ubo = bolt::Buffer::from_data(
            context.clone(),
            bolt::BufferInfo::default()
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .cpu_to_gpu(),
            &[uniforms],
        );
        let desc_set = layout_scene
            .get_or_create(bolt::DescriptorSetInfo::default().buffer(0, ubo.get_descriptor_info()));
        per_frame.push(PerFrameData { ubo, desc_set });
    }

    let pipeline_layout = bolt::PipelineLayout::new(
        context.clone(),
        bolt::PipelineLayoutInfo::default()
            .desc_set_layouts(&[layout_scene.handle(), layout_pass.handle()])
            .push_constant_range(
                vk::PushConstantRange::builder()
                    .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                    .size(size_of::<u32>() as u32)
                    .build(),
            ),
    );

    let (graphics_pipeline, sbt) = build_pipeline_sbt(&context, &pipeline_layout, enable_sky);
    // let (collision_pipeline, collision_sbt) = build_collision_pipeline_sbt(&context, &pipeline_layout);
    let mut accum_target =
        create_image_target(&context, &app.window, vk::Format::R32G32B32A32_SFLOAT);

    let cmd = context.begin_single_time_cmd();
    accum_target.transition_image_layout(cmd, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);
    context.end_single_time_cmd(cmd);

    let render_target = create_image_target(&context, &app.window, vk::Format::R8G8B8A8_UNORM);

    AppData {
        scene,
        graphics_pipeline_layout: pipeline_layout,
        layout_scene,
        graphics_layout_pass: layout_pass,
        per_frame,
        manip: scene::CameraManip {
            camera,
            input: scene::CameraInput::default(),
            mode: scene::CameraMode::Fly,
            ..Default::default()
        },
        scene_description,
        graphics_pipeline,
        // collision_pipeline,
        // layout_compute,
        // compute_pipeline,
        // pipeline_layout_compute,
        // collision_sbt,
        sbt,
        accumulation_start_frame: 0,
        accum_target,
        render_target,
        enable_sky,
        skydome,
    }
}

pub fn window_event(app: &mut bolt::App, data: &mut AppData, event: &WindowEvent) {
    if data.manip.update(&event) {
        data.accumulation_start_frame = app.elapsed_ticks as u32;
    }
    match event {
        WindowEvent::Resized(_) => {
            data.accum_target = create_image_target(
                &app.renderer.context,
                &app.window,
                vk::Format::R32G32B32A32_SFLOAT,
            );
            data.render_target = create_image_target(
                &app.renderer.context,
                &app.window,
                vk::Format::R8G8B8A8_UNORM,
            );
            data.accumulation_start_frame = app.elapsed_ticks as u32;
            data.graphics_layout_pass.reset_pool();
        }
        WindowEvent::KeyboardInput { input, .. } => {
            if input.state == winit::event::ElementState::Pressed {
                if input.virtual_keycode == Some(winit::event::VirtualKeyCode::R) {
                    unsafe {
                        app.renderer
                            .context
                            .device()
                            .queue_wait_idle(app.renderer.context.graphics_queue())
                            .unwrap();
                    }
                    let (pipeline, sbt) = build_pipeline_sbt(
                        &app.renderer.context,
                        &data.graphics_pipeline_layout,
                        data.enable_sky,
                    );
                    data.graphics_pipeline = pipeline;
                    data.sbt = sbt;
                    data.accumulation_start_frame = app.elapsed_ticks as u32;
                }
            }
        }
        _ => {}
    }
}

pub fn render(app: &mut bolt::App, data: &mut AppData) -> Result<(), bolt::AppRenderError> {
    let (semaphore, frame_index) = app.renderer.acquire_next_image()?;

    let ref mut frame_ubo = data.per_frame[frame_index].ubo;
    frame_ubo.update(&[SceneUniforms::from(
        &data.manip.camera,
        uvec3(app.window.get_width(), app.window.get_height(), app.elapsed_ticks as u32)
    )]);

    let cmd = app.renderer.begin_command_buffer();
    let device = app.renderer.context.device();

    unsafe {
        device.cmd_push_constants(
            cmd,
            data.graphics_pipeline_layout.handle(),
            vk::ShaderStageFlags::RAYGEN_KHR,
            0,
            &data.accumulation_start_frame.to_ne_bytes(),
        );
    }

    // let new_geos = data.scene.vulkan_meshes.iter().enumerate().map(|(i, mesh)| {
    //     (i, mesh.primitive_sections.iter().map(|primitive| {
    //             let (index_buffer, index_count, index_offset_size) = match &mesh.index_buffer {
    //                 Some(buffer) => (
    //                     Some(buffer.get_device_address()),
    //                     Some(primitive.get_index_count()),
    //                     Some(primitive.get_index_offset_size::<u32>()),
    //                 ),
    //                 None => (None, None, None),
    //             };
    //             GeometryInstance {
    //             vertex_buffer: mesh.vertex_buffer.get_device_address(),
    //             vertex_count: primitive.get_vertex_count(),
    //             vertex_offset_size: primitive.get_vertex_offset_size(),
    //             vertex_offset: primitive.get_vertex_offset(),
    //             index_buffer,
    //             index_count,
    //             index_offset_size,
    //             transform: glam::Mat4::IDENTITY, //TODO: Does this work?? <- no
    //         }
    //     }))
    // });

    // data.scene_description.blas_regenerate(cmd,new_geos);
    data.scene_description.tlas_regenerate(cmd);

    data.render_target.transition_image_layout(
        cmd,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::GENERAL,
    );

    let desc_pass = data.graphics_layout_pass.get_or_create(
        bolt::DescriptorSetInfo::default()
            .accel_struct(0, data.scene_description.tlas().handle())
            .image(1, data.accum_target.get_descriptor_info())
            .image(2, data.render_target.get_descriptor_info())
            .buffer(
                3,
                data.scene_description
                    .get_instances_buffer()
                    .get_descriptor_info(),
            )
            .buffers(4, data.scene_description.get_vertex_descriptors().clone())
            .buffers(5, data.scene_description.get_index_descriptors().clone())
            .buffers(6, data.scene_description.get_material_descriptors().clone())
            .buffers(7, data.scene_description.get_physics_descriptors().clone())
            .images(8, data.scene_description.get_texture_descriptors().clone())
            .image_opt(9, &data.skydome),
    );

    //let (half_edge_desc, vertex_to_he_desc) = data.scene_description.get_half_edge_descriptors();

    // let compute_pass = data.layout_compute.get_or_create(
    //     bolt::DescriptorSetInfo::default()
    //     .buffers(0, vertex_to_he_desc.clone())
    //     .buffers(1, data.scene_description.get_vertex_descriptors().clone())
    //     .buffers(2, half_edge_desc.clone())
    //     .buffers(2, data.scene_description.get_physics_descriptors().clone())
    // );

    let descriptor_sets = [data.per_frame[frame_index].desc_set.handle(), desc_pass.handle()];
    // unsafe {
    //     device.cmd_set_scissor(cmd, 0, &[app.window.get_rect()]);
    //     device.cmd_set_viewport(cmd, 0, &[app.window.get_viewport()]);
    //     // run collision pass
    //     // device.cmd_bind_pipeline(
    //     //     cmd,
    //     //     vk::PipelineBindPoint::RAY_TRACING_KHR,
    //     //     data.collision_pipeline.handle(),
    //     // );
    //     device.cmd_bind_descriptor_sets(
    //         cmd,
    //         vk::PipelineBindPoint::RAY_TRACING_KHR,
    //         data.graphics_pipeline_layout.handle(),
    //         0,
    //         descriptor_sets.as_slice(),
    //         &[],
    //     );
    // }
    // todo depth should be instance count
    // let instance_count = data.scene_description.get_instances_buffer().get_element_count();
    // data.collision_sbt.cmd_trace_rays(cmd, vk::Extent3D::builder().width(1024).height(1024).depth(instance_count).build());


    unsafe {
        // device.cmd_pipeline_barrier(
        //     cmd,
        //     vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR, // Source stage
        //     vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR, // Destination stage
        //     vk::DependencyFlags::empty(),
        //     &[],
        //     &[vk::BufferMemoryBarrier {
        //         src_access_mask: vk::AccessFlags::SHADER_WRITE, // The collision pass writes to the buffer
        //         dst_access_mask: vk::AccessFlags::SHADER_READ,  // The graphics pass reads from the buffer
        //         src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        //         dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        //         buffer: data.scene.meshes[0].vertices.handle(), // The buffer being accessed
        //         offset: 0,                  // Start offset of the buffer
        //         size: vk::WHOLE_SIZE,       // Size of the buffer
        //         ..Default::default()
        //     }],
        //     &[],
        // );
        // run the graphics pass
        device.cmd_set_scissor(cmd, 0, &[app.window.get_rect()]);
        device.cmd_set_viewport(cmd, 0, &[app.window.get_viewport()]);
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::RAY_TRACING_KHR, 
            data.graphics_pipeline.handle()
        );
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            data.graphics_pipeline_layout.handle(),
            0,
            descriptor_sets.as_slice(),
            &[],
        );
    }
    data.sbt.cmd_trace_rays(cmd, app.window.get_extent_3d());
    let present_image = app.renderer.swapchain.get_present_image(frame_index);
    data.render_target.cmd_blit_to(cmd, present_image, true);
    present_image.transition_image_layout(
        cmd,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::PRESENT_SRC_KHR,
    );
    app.renderer.end_command_buffer(cmd);
    app.renderer.submit_and_present(cmd, semaphore)
}

pub fn prepare() -> bolt::AppSettings {
    bolt::AppSettings {
        name: "Pathtrace App".to_string(),
        resolution: [1280, 720],
        render: bolt::RendererSettings {
            extensions: vec![vk::KhrGetPhysicalDeviceProperties2Fn::name()],
            ..Default::default()
        },
        ..Default::default()
    }
}

/// WTF something is wrong. NVidia driver 532.03 runs this no problemo. Upwards we get ERROR_DEVICE_LOST.
pub fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    bolt::App::build(setup)
        .prepare(prepare)
        .render(render)
        .window_event(window_event)
        .run();
}
